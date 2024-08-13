use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use log::{debug};

use fuser::{FileAttr, FileType};
use sha2::{Digest, Sha256};

pub struct ContentIndexer {
    pub image_name: String,
    pub last_saved_inode: u64,
    pub next_inode: u64,
    pub directory: HashMap<u64, HashMap<String, u64>>,
    pub file_attr: HashMap<u64, FileAttr>,
    pub inode_to_hash: HashMap<u64, String>,
    pub output_folder: PathBuf,
    pub total_files: usize,
    pub processed_files: Arc<AtomicUsize>,
}

impl ContentIndexer {
    pub fn new(image_name: &str, last_saved_inode: u64, output_folder: PathBuf) -> Self {
        Self {
            image_name: image_name.to_string(),
            last_saved_inode,
            next_inode: last_saved_inode,
            directory: HashMap::new(),
            file_attr: HashMap::new(),
            inode_to_hash: HashMap::new(),
            output_folder,
            total_files: 0,
            processed_files: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn save_directory(&mut self, path: &Path, parent_ino: u64) -> io::Result<u64> {
        let ino = if self.last_saved_inode == self.next_inode {
            1
        } else {
            self.next_inode
        };

        self.next_inode += 1;

        let metadata = fs::symlink_metadata(path)?;
        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else if metadata.is_symlink() {
            FileType::Symlink
        } else {
            FileType::RegularFile
        };

        let attr = FileAttr {
            ino,
            size: metadata.len(),
            blocks: metadata.blocks(),
            atime: metadata.accessed().unwrap_or(UNIX_EPOCH),
            mtime: metadata.modified().unwrap_or(UNIX_EPOCH),
            ctime: metadata.created().unwrap_or(UNIX_EPOCH),
            crtime: metadata.created().unwrap_or(UNIX_EPOCH),
            kind: file_type,
            perm: metadata.mode() as u16,
            nlink: metadata.nlink() as u32,
            uid: 1000,
            gid: 1000,
            rdev: metadata.rdev() as u32,
            flags: 0,
            blksize: 4096,
        };

        self.file_attr.insert(ino, attr);

        match file_type {
            FileType::Directory => {
                let mut children = HashMap::new();
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let file_name = entry.file_name().into_string().unwrap();
                    let child_ino = self.save_directory(&entry.path(), ino)?;
                    children.insert(file_name, child_ino);
                }
                self.directory.insert(ino, children);
            }
            FileType::Symlink => {
                let target = fs::read_link(path)?;
                let content = target.to_string_lossy().into_owned();
                self.save_content(ino, content.as_bytes())?;
            }
            FileType::RegularFile => {
                let content = fs::read(path)?;
                self.save_content(ino, &content)?;
            }
            _ => {}
        }

        if parent_ino != ino {
            self.directory
                .entry(parent_ino)
                .or_insert_with(HashMap::new)
                .insert(path.file_name().unwrap().to_str().unwrap().to_string(), ino);
        }

        let processed = self.processed_files.fetch_add(1, Ordering::SeqCst) + 1;
        debug!(
            "Progress: {}/{} files processed",
            processed, self.total_files
        );

        Ok(ino)
    }

    fn save_content(&mut self, ino: u64, content: &[u8]) -> io::Result<()> {
        let hash = self.hash_content(content);
        let file_path = self.output_folder.join(&hash);

        if !file_path.exists() {
            File::create(file_path)?.write_all(content)?;
        }

        self.inode_to_hash.insert(ino, hash);
        Ok(())
    }

    fn hash_content<T: AsRef<[u8]>>(&self, content: T) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}