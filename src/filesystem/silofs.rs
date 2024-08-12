use crate::filesystem::mount::SiloFSMount;
use dashmap::DashMap;
use fuser::MountOption;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

/// Time-to-live for file system entries
pub const TTL: std::time::Duration = std::time::Duration::from_secs(20);

/// Data structure for serializing and deserializing file system metadata
#[derive(Serialize, Deserialize)]
pub struct DatatoSend {
    pub directory_cache: HashMap<u64, HashMap<String, u64>>,
    pub file_attr_cache: HashMap<u64, fuser::FileAttr>,
    pub inode_to_hash: HashMap<u64, String>,
}

/// In-memory representation of image data
pub struct ImageData {
    pub directory_cache: HashMap<u64, HashMap<String, u64>>,
    pub file_attr_cache: HashMap<u64, fuser::FileAttr>,
    pub inode_to_hash: HashMap<u64, String>,
    pub content_cache: DashMap<String, Vec<u8>>,
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
                log::error!("Failed to mount {}: {}", mount_location, e);
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
        log::info!("Loading {} cache from indexer...", image_name);

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

        log::info!("Loaded {} cache from indexer", image_name);

        Ok(Arc::new(ImageData {
            content_cache: DashMap::new(),
            directory_cache: data.directory_cache,
            file_attr_cache: data.file_attr_cache,
            inode_to_hash: data.inode_to_hash,
        }))
    }
}
