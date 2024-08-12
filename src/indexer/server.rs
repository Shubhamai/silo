use anyhow::Result;
use log::{debug, error, info};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::database::AppState;
use fuser::FileAttr;

#[derive(Serialize, Deserialize)]
struct DataToSend {
    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
}

pub async fn run_tcp_server(state: AppState, host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;


    info!("TCP server listening on {}", addr);

    let cache: Cache<String, Arc<Vec<u8>>> = Cache::new(10_000);

    loop {
        let (mut socket, addr) = listener.accept().await?;
        socket.set_nodelay(true)?;
        
        let state = state.clone();
        let cache = cache.clone();

        info!("New client connected: {:?}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(&mut socket, &state, cache).await {
                error!("Client error: {}", e);
            }
        });
    }
}

async fn handle_client(
    socket: &mut tokio::net::TcpStream,
    state: &AppState,
    cache: Cache<String, Arc<Vec<u8>>>,
) -> Result<()> {
    let mut buf = [0; 64];

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                info!("Client disconnected");
                break;
            }
            Ok(_) => {
                let start = Instant::now();

                let request = String::from_utf8_lossy(&buf)
                    .trim_end_matches('\0')
                    .to_string();

                if request.starts_with("GET_DATA:") {
                    let image_name = request.strip_prefix("GET_DATA:").unwrap();
                    debug!("Received GET_DATA request for image: {}", image_name);
                    let data = get_data(image_name, state).await?;
                    let serialized = serde_json::to_vec(&data)?;

                    socket
                        .write_all(&(serialized.len() as u64).to_be_bytes())
                        .await?;
                    socket.write_all(&serialized).await?;
                } else {
                    debug!("Received file request: {}", request);
                    let file_path = state.output_folder.join(&request);

                    if let Some(file) = cache.get(&request).await {
                        socket.write_all(&(file.len() as u64).to_be_bytes()).await?;
                        socket.write_all(&file).await?;
                    } else {
                        let file = tokio::fs::read(&file_path).await?;
                        socket.write_all(&(file.len() as u64).to_be_bytes()).await?;
                        socket.write_all(&file).await?;
                        cache.insert(request, Arc::new(file)).await;
                    }
                }

                // print that takes more than 2ms to read the content
                if start.elapsed().as_millis() > 2 {
                    info!("Read content in {}ms", start.elapsed().as_millis());
                }
            }
            Err(e) => {
                error!("Failed to read from socket: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn get_data(image_name: &str, state: &AppState) -> Result<DataToSend> {
    let (directory, file_attr, inode_to_hash) = state.get_image_data(image_name).await?;

    Ok(DataToSend {
        directory_cache: directory,
        file_attr_cache: file_attr,
        inode_to_hash,
    })
}
