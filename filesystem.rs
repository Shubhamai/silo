use dashmap::DashMap;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request,
};
use libc::ENOENT;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Time-to-live for file system entries
const TTL: Duration = Duration::from_secs(20);

/// Data structure for serializing and deserializing file system metadata
#[derive(Serialize, Deserialize)]
struct DatatoSend {
    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
}

/// In-memory representation of image data
struct ImageData {
    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
    content_cache: DashMap<String, Vec<u8>>,
}

/// Main structure for SiloFS
pub struct SiloFS {
    stream: Arc<Mutex<std::net::TcpStream>>,
    images: DashMap<String, Arc<ImageData>>,
}

impl SiloFS {
    /// Create a new SiloFS instance
    pub fn new(tcp_addr: &str) -> io::Result<Self> {
        let stream = std::net::TcpStream::connect(tcp_addr).map_err(|e| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Failed to connect to {}: {}", tcp_addr, e),
            )
        })?;

        stream.set_nodelay(true).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to set TCP_NODELAY: {}", e),
            )
        })?;

        Ok(SiloFS {
            stream: Arc::new(Mutex::new(stream)),
            images: DashMap::new(),
        })
    }

    /// Mount an image at a specified location
    pub fn mount(
        &self,
        image_name: &str,
        mount_location: &str,
    ) -> io::Result<thread::JoinHandle<()>> {
        let image_data = self.load_or_get_image_data(image_name)?;
        let fs = SiloFSMount {
            stream: self.stream.clone(),
            image_data,
        };

        let options = vec![
            MountOption::RO,
            MountOption::FSName("silofs".to_string()),
            MountOption::AutoUnmount,
            MountOption::AllowOther,
            MountOption::Exec,
        ];

        let mount_location = mount_location.to_string();
        let handle = thread::spawn(move || {
            if let Err(e) = fuser::mount2(fs, mount_location.clone(), &options) {
                error!("Failed to mount {}: {}", mount_location, e);
            }
        });

        Ok(handle)
    }

    /// Load or get image data from cache
    fn load_or_get_image_data(&self, image_name: &str) -> io::Result<Arc<ImageData>> {
        if let Some(image_data) = self.images.get(image_name) {
            return Ok(image_data.clone());
        }

        let image_data = self.load_cache(image_name)?;
        self.images
            .insert(image_name.to_string(), image_data.clone());
        Ok(image_data)
    }

    /// Load cache from indexer
    fn load_cache(&self, image_name: &str) -> io::Result<Arc<ImageData>> {
        info!("Loading {} cache from indexer...", image_name);

        let request = format!("GET_DATA:{}", image_name);

        let mut stream = self.stream.lock().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to lock stream: {}", e),
            )
        })?;
        stream.write_all(request.as_bytes()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to write request: {}", e),
            )
        })?;

        let mut size_buf = [0u8; 8];
        stream.read_exact(&mut size_buf).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to read size: {}", e))
        })?;
        let size = u64::from_be_bytes(size_buf);

        let mut data = vec![0u8; size as usize];
        stream.read_exact(&mut data).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to read data: {}", e))
        })?;

        let data: DatatoSend = serde_json::from_slice(&data).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to deserialize data: {}", e),
            )
        })?;

        info!("Loaded {} cache from indexer", image_name);

        Ok(Arc::new(ImageData {
            content_cache: DashMap::new(),
            directory_cache: data.directory_cache,
            file_attr_cache: data.file_attr_cache,
            inode_to_hash: data.inode_to_hash,
        }))
    }
}

/// Structure representing a mounted SiloFS instance
struct SiloFSMount {
    stream: Arc<Mutex<std::net::TcpStream>>,
    image_data: Arc<ImageData>,
}

impl SiloFSMount {
    /// Get contents of a file by inode
    fn get_contents(&self, ino: u64) -> io::Result<Vec<u8>> {
        let hash = self.image_data.inode_to_hash.get(&ino).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Inode {} not found in hash map", ino),
            )
        })?;

        if let Some(content) = self.image_data.content_cache.get(hash) {
            return Ok(content.to_vec());
        }

        let mut stream = self.stream.lock().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to lock stream: {}", e),
            )
        })?;

        stream.write_all(hash.as_bytes()).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to write hash: {}", e))
        })?;

        let mut size_buf = [0u8; 8];
        stream.read_exact(&mut size_buf).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to read size: {}", e))
        })?;
        let size = u64::from_be_bytes(size_buf);

        let mut content = vec![0u8; size as usize];
        stream.read_exact(&mut content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to read content: {}", e),
            )
        })?;

        self.image_data
            .content_cache
            .insert(hash.to_string(), content.clone());

        Ok(content)
    }

    /// Get file attributes by inode
    fn get_attr(&self, ino: u64) -> io::Result<FileAttr> {
        self.image_data
            .file_attr_cache
            .get(&ino)
            .copied()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("Inode {} not found", ino))
            })
    }

    /// Get children of a directory by inode
    fn get_children(&self, ino: u64) -> io::Result<HashMap<String, u64>> {
        self.image_data
            .directory_cache
            .get(&ino)
            .cloned()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Directory inode {} not found", ino),
                )
            })
    }
}

impl Filesystem for SiloFSMount {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let start = Instant::now();

        match self.get_children(parent) {
            Ok(children) => {
                if let Some(&child_ino) = children.get(name.to_str().unwrap_or("")) {
                    match self.get_attr(child_ino) {
                        Ok(attr) => {
                            reply.entry(&TTL, &attr, 0);
                        }
                        Err(e) => {
                            error!("Failed to get attributes for inode {}: {}", child_ino, e);
                            reply.error(ENOENT);
                        }
                    }
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(e) => {
                error!("Failed to get children for parent inode {}: {}", parent, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            debug!("Lookup latency: {:?}ms", start.elapsed().as_millis());
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let start = Instant::now();

        match self.get_attr(ino) {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(e) => {
                error!("Failed to get attributes for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
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

        match self.get_contents(ino) {
            Ok(content) => {
                let start = offset as usize;
                let end = (offset as usize + size as usize).min(content.len());
                reply.data(&content[start..end]);
            }
            Err(e) => {
                error!("Failed to read contents for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
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

        // Check if the file exists before opening
        match self.get_attr(ino) {
            Ok(_) => reply.opened(0, 0),
            Err(e) => {
                error!("Failed to open file with inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

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
        match self.get_children(ino) {
            Ok(children) => {
                let mut entries = BTreeMap::new();

                entries.insert(".".to_string(), (ino, FileType::Directory));
                entries.insert("..".to_string(), (ino, FileType::Directory));

                for (name, &child_ino) in children.iter() {
                    if let Ok(attr) = self.get_attr(child_ino) {
                        entries.insert(name.clone(), (child_ino, attr.kind));
                    }
                }

                let mut sorted_entries: Vec<_> = entries.into_iter().collect();
                sorted_entries.sort_unstable_by(|(name_a, (_, type_a)), (name_b, (_, type_b))| {
                    match (type_a, type_b) {
                        (FileType::Directory, FileType::Directory) => name_a.cmp(name_b),
                        (FileType::Directory, _) => Ordering::Less,
                        (_, FileType::Directory) => Ordering::Greater,
                        _ => name_a.cmp(name_b),
                    }
                });

                for (i, (name, (child_ino, file_type))) in
                    sorted_entries.into_iter().enumerate().skip(offset as usize)
                {
                    if reply.add(child_ino, (i + 1) as i64, file_type, name) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(e) => {
                error!("Failed to read directory contents for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }
    }

    fn readlink(&mut self, _req: &Request, ino: u64, reply: ReplyData) {
        let start = Instant::now();

        match self.get_contents(ino) {
            Ok(content) => reply.data(&content),
            Err(e) => {
                error!("Failed to read symlink for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            debug!(
                "Readlink latency: {:?}ms for inode {}",
                start.elapsed().as_millis(),
                ino
            );
        }
    }
}

/// Custom error type for SiloFS operations
#[derive(Debug)]
enum SiloFSError {
    Io(io::Error),
    SerdeJson(serde_json::Error),
}

impl From<io::Error> for SiloFSError {
    fn from(error: io::Error) -> Self {
        SiloFSError::Io(error)
    }
}

impl From<serde_json::Error> for SiloFSError {
    fn from(error: serde_json::Error) -> Self {
        SiloFSError::SerdeJson(error)
    }
}

impl std::fmt::Display for SiloFSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiloFSError::Io(e) => write!(f, "I/O error: {}", e),
            SiloFSError::SerdeJson(e) => write!(f, "Serde JSON error: {}", e),
        }
    }
}

impl std::error::Error for SiloFSError {}
