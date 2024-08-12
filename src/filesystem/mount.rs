use crate::filesystem::silofs::{ImageData, TTL};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request,
};
use libc::ENOENT;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Structure representing a mounted SiloFS instance
pub struct SiloFSMount {
    pub stream: Arc<Mutex<std::net::TcpStream>>,
    pub image_data: Arc<ImageData>,
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
    fn get_children(&self, ino: u64) -> io::Result<std::collections::HashMap<String, u64>> {
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
                            log::error!("Failed to get attributes for inode {}: {}", child_ino, e);
                            reply.error(ENOENT);
                        }
                    }
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(e) => {
                log::error!("Failed to get children for parent inode {}: {}", parent, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            log::debug!("Lookup latency: {:?}ms", start.elapsed().as_millis());
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let start = Instant::now();

        match self.get_attr(ino) {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(e) => {
                log::error!("Failed to get attributes for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            log::debug!("GetAttr latency: {:?}ms", start.elapsed().as_millis());
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
                log::error!("Failed to read contents for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            log::debug!(
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
                log::error!("Failed to open file with inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            log::debug!(
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
                log::error!("Failed to read directory contents for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }
    }

    fn readlink(&mut self, _req: &Request, ino: u64, reply: ReplyData) {
        let start = Instant::now();

        match self.get_contents(ino) {
            Ok(content) => reply.data(&content),
            Err(e) => {
                log::error!("Failed to read symlink for inode {}: {}", ino, e);
                reply.error(ENOENT);
            }
        }

        if start.elapsed().as_millis() > 2 {
            log::debug!(
                "Readlink latency: {:?}ms for inode {}",
                start.elapsed().as_millis(),
                ino
            );
        }
    }
}
