use clap::Parser;
use dashmap::DashMap;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request,
};
use futures::executor::block_on;
use libc::ENOENT;
use log::{debug, info};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(20);

#[derive(Serialize, Deserialize)]
struct DatatoSend {
    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
}

pub struct SiloFS {
    stream: std::net::TcpStream,

    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,

    content_cache: DashMap<String, Vec<u8>>,

    total_read_ms: u128,
    total_tcp_commands: u128,
}

impl SiloFS {
    // async
    pub fn run(tcp_addr: &str, mountpoint: &str, image_name: &str) -> io::Result<()> {
        let stream = std::net::TcpStream::connect(tcp_addr)?;
        stream.set_nodelay(true)?;

        let mut fs = SiloFS {
            stream,
            content_cache: DashMap::new(),
            directory_cache: HashMap::new(),
            file_attr_cache: HashMap::new(),
            inode_to_hash: HashMap::new(),
            total_read_ms: 0,
            total_tcp_commands: 0,
        };

        fs.load_cache(image_name)?;

        let options = vec![
            MountOption::RO,
            MountOption::FSName("silofs".to_string()),
            MountOption::AutoUnmount,
            MountOption::AllowOther,
            MountOption::Exec,
        ];

        fuser::mount2(fs, mountpoint, &options)?;

        // Ok(fs)
        Ok(())
    }

    fn load_cache(&mut self, image_name: &str) -> io::Result<()> {
        info!("Loading {} cache from indexer...", image_name);

        // Request data for a specific image
        let request = format!("GET_DATA:{}", image_name);

        self.stream.write_all(request.as_bytes())?;

        // Read the response size
        let mut size_buf = [0u8; 8];
        self.stream.read_exact(&mut size_buf)?;
        let size = u64::from_be_bytes(size_buf);

        // Read the response data
        let mut data = vec![0u8; size as usize];
        self.stream.read_exact(&mut data)?;

        // Deserialize the response
        let data: DatatoSend = serde_json::from_slice(&data)?; //.context("Failed to deserialize response")?;

        info!("Loaded {} cache from indexer", image_name);

        self.directory_cache = data.directory_cache;
        self.file_attr_cache = data.file_attr_cache;
        self.inode_to_hash = data.inode_to_hash;

        Ok(())
    }

    fn get_contents(&mut self, ino: u64) -> io::Result<Vec<u8>> {
        let start = Instant::now();

        let hash = self.inode_to_hash.get(&ino).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Inode not found in hash map")
        })?;

        if let Some(content) = self.content_cache.get(hash) {
            return Ok(content.to_vec());
        }

        self.stream.write_all(hash.as_bytes())?;
        let mut size_buf = [0u8; 8];
        self.stream.read_exact(&mut size_buf)?;
        let size = u64::from_be_bytes(size_buf);

        let mut content = vec![0u8; size as usize];
        self.stream.read_exact(&mut content)?;

        // Update in-memory cache
        self.content_cache.insert(hash.to_string(), content.clone());

        self.total_read_ms += start.elapsed().as_millis();
        self.total_tcp_commands += 1;

        Ok(content)
    }

    fn get_attr(&mut self, ino: u64) -> io::Result<FileAttr> {
        self.file_attr_cache
            .get(&ino)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Inode not found"))
    }

    fn get_children(&mut self, ino: u64) -> io::Result<HashMap<String, u64>> {
        match self.directory_cache.get(&ino) {
            Some(children) => Ok(children.clone()),
            None => Ok(HashMap::new()),
        }
    }
}

impl Filesystem for SiloFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let start = Instant::now();

        match self.get_children(parent) {
            Ok(children) => {
                if let Some(&child_ino) = children.get(name.to_str().unwrap()) {
                    match self.get_attr(child_ino) {
                        Ok(attr) => {
                            reply.entry(&TTL, &attr, 0);
                            return;
                        }

                        Err(_) => {
                            reply.error(ENOENT);
                            return;
                        }
                    }
                }
            }
            Err(_) => {
                reply.error(ENOENT);
                return;
            }
        }

        reply.error(ENOENT);

        if start.elapsed().as_millis() > 2 {
            debug!("Lookup latency: {:?}ms", start.elapsed().as_millis());
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let start = Instant::now();

        match self.get_attr(ino) {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(_) => reply.error(ENOENT),
        }

        if start.elapsed().as_millis() > 2 {
            debug!("GetAttr latency: {:?}ms", start.elapsed().as_millis());
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let start = Instant::now();

        if let Ok(content) = self.get_contents(ino) {
            let start = offset as usize;
            let end = (offset as usize + size as usize).min(content.len());
            reply.data(&content[start..end]);
        } else {
            reply.error(ENOENT);
        }
        if start.elapsed().as_millis() > 2 {
            debug!(
                "Read latency: {:?}ms ino={ino}",
                start.elapsed().as_millis()
            );
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let start = Instant::now();

        // Perform read-ahead when a file is opened
        if let Ok(content) = self.get_contents(ino) {
            if let Some(hash) = self.inode_to_hash.get(&ino) {
                self.content_cache.insert(hash.clone(), content.clone());
            }
        }
        reply.opened(0, 0);

        if start.elapsed().as_millis() > 2 {
            debug!(
                "Open latency: {:?}ms ino={ino}",
                start.elapsed().as_millis()
            );
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if let Ok(children) = self.get_children(ino) {
            let mut entries = BTreeMap::new();

            // Add "." and ".." entries
            entries.insert(".".to_string(), (ino, FileType::Directory));
            entries.insert("..".to_string(), (ino, FileType::Directory));

            // Add children entries
            for (name, &child_ino) in children.iter() {
                if let Ok(attr) = self.get_attr(child_ino) {
                    entries.insert(name.clone(), (child_ino, attr.kind));
                }
            }

            // Use custom sorting for directories
            let mut sorted_entries: Vec<_> = entries.into_iter().collect();
            sorted_entries.sort_unstable_by(|(name_a, (_, type_a)), (name_b, (_, type_b))| match (
                type_a, type_b,
            ) {
                (FileType::Directory, FileType::Directory) => name_a.cmp(name_b),
                (FileType::Directory, _) => Ordering::Less,
                (_, FileType::Directory) => Ordering::Greater,
                _ => name_a.cmp(name_b),
            });

            for (i, (name, (child_ino, file_type))) in
                sorted_entries.into_iter().enumerate().skip(offset as usize)
            {
                if reply.add(child_ino, (i + 1) as i64, file_type, name) {
                    break;
                }
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn readlink(&mut self, _req: &Request, ino: u64, reply: ReplyData) {
        let start = Instant::now();

        if let Ok(content) = self.get_contents(ino) {
            reply.data(&content);
        } else {
            reply.error(ENOENT);
        }
        if start.elapsed().as_millis() > 2 {
            debug!("Readlink latency: {:?}ms", start.elapsed().as_millis());
        }
    }
}
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Mount point for the filesystem
    #[clap(short, long, default_value = "fusefs_mount")]
    mount: PathBuf,

    /// TCP server address
    #[clap(short, long, default_value = "127.0.0.1:8080")]
    tcp_addr: String,

    /// Name of the docker image to mount
    #[clap(
        short,
        long,
        default_value = "r8.im/shubhamai/yolov10@sha256:e387e93e8f7f55fa5ae21e94585cfae5361468376c45fc874defa1dd5ca67f5d"
    )]
    name: String,
}

// #[async_std::main]
// async fn main() -> io::Result<()> {
//     env_logger::init();

//     let args = Args::parse();

//     let fs = SiloFS::new(&args)?; //.await?;
//     println!("Mounting filesystem...");

//     Ok(())

//     // let options = vec![
//     //     MountOption::RO,
//     //     MountOption::FSName("simplefs".to_string()),
//     //     MountOption::AutoUnmount,
//     //     MountOption::AllowOther,
//     //     MountOption::Exec,
//     // ];

//     // fuser::mount2(fs, &args.mount, &options)
// }
