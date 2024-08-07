use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use moka::future::Cache;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use walkdir::WalkDir;

use fuser::{FileAttr, FileType};

// CLI argument parser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, default_value = "127.0.0.1")] // short,
    host: String,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    #[arg(short, long, default_value = "./content")]
    storage: PathBuf,

    #[arg(short, long, default_value = "indexer.db")]
    db: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(name = "index", about = "Index a podman image")]
    Index { image_name: String },

    #[clap(name = "list", about = "List indexed podman images")]
    List,
}

// Struct to hold the state of the content indexer
struct ContentIndexer {
    image_name: String,
    last_saved_inode: u64,
    next_inode: u64,
    directory: HashMap<u64, HashMap<String, u64>>,
    file_attr: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
    output_folder: PathBuf,
    total_files: usize,
    processed_files: Arc<AtomicUsize>,
}

impl ContentIndexer {
    fn new(image_name: &str, last_saved_inode: u64, output_folder: PathBuf) -> Self {
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

    // Save directory structure and file attributes
    fn save_directory(&mut self, path: &Path, parent_ino: u64) -> io::Result<u64> {
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
        print!(
            "\rProgress: {}/{} files processed",
            processed, self.total_files
        );
        io::stdout().flush()?;

        Ok(ino)
    }

    // Save file content and create a hash
    fn save_content(&mut self, ino: u64, content: &[u8]) -> io::Result<()> {
        let hash = self.hash_content(content);
        let file_path = self.output_folder.join(&hash);

        if !file_path.exists() {
            File::create(file_path)?.write_all(content)?;
        }

        self.inode_to_hash.insert(ino, hash);
        Ok(())
    }

    // Generate SHA256 hash for content
    fn hash_content<T: AsRef<[u8]>>(&self, content: T) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}

// Pull podman image
fn pull_image(image_name: &str) -> io::Result<()> {
    let output = Command::new("sudo")
        .args(&["podman", "pull", image_name])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to pull image {}", image_name),
        ));
    }
    Ok(())
}

// Run podman container
fn run_container(image_name: &str) -> io::Result<String> {
    let output = Command::new("sudo")
        .args(&["podman", "run", "-dt", image_name])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to run container",
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// Mount podman container
fn mount_container(container_id: &str) -> io::Result<PathBuf> {
    let output = Command::new("sudo")
        .args(&["podman", "mount", container_id])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to mount container",
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

// Save indexed data to SQLite database
fn save_to_sqlite(
    conn: &Connection,
    fs: &ContentIndexer,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS indexer (
            id TEXT PRIMARY KEY,
            next_inode INTEGER,
            directory TEXT,
            file_attr TEXT,
            inode_to_hash TEXT
        )",
        [],
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO indexer (id, next_inode, directory, file_attr, inode_to_hash) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            fs.image_name,
            fs.next_inode,
            serde_json::to_string(&fs.directory)?,
            serde_json::to_string(&fs.file_attr)?,
            serde_json::to_string(&fs.inode_to_hash)?
        ],
    )?;

    Ok(())
}

// Save next inode value to SQLite database
fn save_next_inode(conn: &Connection, next_inode: u64) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS next_inode (
            id TEXT PRIMARY KEY,
            next_inode INTEGER
        )",
        [],
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO next_inode (id, next_inode) 
         VALUES (?1, ?2)",
        params!["next_inode", next_inode],
    )?;

    Ok(())
}

// Load next inode value from SQLite database
fn load_next_inode(conn: &Connection) -> Result<u64, Box<dyn std::error::Error>> {
    conn.query_row(
        "SELECT next_inode FROM next_inode WHERE id = 'next_inode'",
        [],
        |row| row.get(0),
    )
    .or_else(|_| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS next_inode (
                id TEXT PRIMARY KEY,
                next_inode INTEGER
            )",
            [],
        )?;
        conn.execute(
            "INSERT INTO next_inode (id, next_inode) 
             VALUES ('next_inode', 1)",
            [],
        )?;
        Ok(1)
    })
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    output_folder: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct DatatoSend {
    directory_cache: HashMap<u64, HashMap<String, u64>>,
    file_attr_cache: HashMap<u64, FileAttr>,
    inode_to_hash: HashMap<u64, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let output_folder = args.storage;
    let db_path = args.db;

    // Check if the output folder exists
    if !output_folder.exists() {
        fs::create_dir_all(&output_folder)?;
    }

    let conn = Connection::open(db_path)?;
    let db = Arc::new(Mutex::new(conn));

    let app_state = AppState {
        db,
        output_folder: output_folder.clone(),
    };

    // Start the TCP server in the background
    let tcp_server = tokio::spawn({
        let app_state = app_state.clone();
        async move {
            run_tcp_server(app_state, &args.host, args.port)
                .await
                .unwrap();
        }
    });

    // Handle CLI commands
    match args.command {
        Some(Commands::Index { image_name }) => {
            index_image(&image_name, &app_state).await?;
        }
        Some(Commands::List) => {
            list_images(&app_state).await?;
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    // Wait for the TCP server to finish (which it never will in this case)
    tcp_server.await?;

    Ok(())
}

async fn index_image(image_name: &str, state: &AppState) -> Result<()> {
    let start_time = std::time::Instant::now();

    println!("Pulling image: {}", image_name);
    pull_image(image_name)?;

    println!("Running container: {}", image_name);
    let container_id = run_container(image_name)?;

    println!("Mounting container: {} ({})", image_name, container_id);
    let mount_path = mount_container(&container_id)?;

    let conn = state.db.lock().await;
    let last_saved_inode = load_next_inode(&conn).unwrap();

    let mut fs = ContentIndexer::new(image_name, last_saved_inode, state.output_folder.clone());

    fs.total_files = WalkDir::new(&mount_path).into_iter().count();
    println!("Total files to process: {}", fs.total_files);

    // Process files
    fs.save_directory(&mount_path, 0)?;

    let elapsed = start_time.elapsed().as_secs_f64();

    println!(
        "\nSaving data for image: {}, elapsed: {:.2}s",
        image_name, elapsed
    );

    save_to_sqlite(&conn, &fs).unwrap();
    save_next_inode(&conn, fs.next_inode).unwrap();

    println!("Image indexed successfully!");

    Ok(())
}

async fn list_images(state: &AppState) -> Result<()> {
    let conn = state.db.lock().await;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS indexer (
            id TEXT PRIMARY KEY,
            next_inode INTEGER,
            directory TEXT,
            file_attr TEXT,
            inode_to_hash TEXT
        )",
        [],
    )?;

    let mut stmt = conn.prepare("SELECT id FROM indexer")?;
    let image_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;

    if image_names.is_empty() {
        println!("No images indexed yet.");
    } else {
        println!("Indexed images:");
        for (i, name) in image_names.iter().enumerate() {
            println!("{}. {}", i + 1, name);
        }
    }

    Ok(())
}

async fn run_tcp_server(state: AppState, host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    println!("TCP server listening on {}", addr);

    let cache: Cache<String, Arc<Vec<u8>>> = Cache::new(10_000);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let state = state.clone();
        let cache = cache.clone();

        println!("New client connected: {:?}", socket.peer_addr());

        tokio::spawn(async move {
            if let Err(e) =
                handle_client(&mut socket, &state.output_folder.clone(), cache, state).await
            {
                eprintln!("Client error: {}", e);
            }
        });
    }
}

async fn get_data(image_name: &str, state: &AppState) -> Result<DatatoSend> {
    let conn = state.db.lock().await;

    let mut stmt = conn.prepare(
        "SELECT next_inode, directory, file_attr, inode_to_hash 
         FROM indexer 
         WHERE id = ?1",
    )?;

    let (_, directory, file_attr, inode_to_hash) = stmt.query_row(params![image_name], |row| {
        Ok((
            row.get::<_, u64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let directory: HashMap<u64, HashMap<String, u64>> = serde_json::from_str(&directory)?;
    let file_attr: HashMap<u64, FileAttr> = serde_json::from_str(&file_attr)?;
    let inode_to_hash: HashMap<u64, String> = serde_json::from_str(&inode_to_hash)?;

    Ok(DatatoSend {
        directory_cache: directory,
        file_attr_cache: file_attr,
        inode_to_hash,
    })
}

async fn handle_client(
    socket: &mut tokio::net::TcpStream,
    storage: &PathBuf,
    cache: Cache<String, Arc<Vec<u8>>>,
    state: AppState,
) -> Result<()> {
    let mut buf = [0; 64];

    loop {
        // match socket.read_exact(&mut buf).await {
        match socket.read(&mut buf).await {
            Ok(_) => {
                let request = String::from_utf8_lossy(&buf)
                    .trim_end_matches('\0')
                    .to_string();

                if request.starts_with("GET_DATA:") {
                    let image_name = request.strip_prefix("GET_DATA:").unwrap();
                    let data = get_data(image_name, &state).await?;
                    let serialized = serde_json::to_vec(&data)?;

                    socket
                        .write_all(&(serialized.len() as u64).to_be_bytes())
                        .await?;

                    socket.write_all(&serialized).await?;
                } else {
                    // Handle file requests as before
                    let file_path = storage.join(&request);

                    if let Some(file) = cache.get(&request).await {
                        socket.write_all(&(file.len() as u64).to_be_bytes()).await?;
                        socket.write_all(&file).await?;
                        continue;
                    }

                    let file = tokio::fs::read(&file_path).await?;
                    socket.write_all(&(file.len() as u64).to_be_bytes()).await?;
                    socket.write_all(&file).await?;
                    cache.insert(request, Arc::new(file)).await;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("Client disconnected");
                break;
            }
            Err(e) => {
                return Err(e).context("Failed to read from socket");
            }
        }
    }

    Ok(())
}
