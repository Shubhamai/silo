use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::indexer::ContentIndexer;
use fuser::FileAttr;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub output_folder: PathBuf,
}

impl AppState {
    pub async fn new(db_path: String, output_folder: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {}", db_path))?;

        let app_state = Self {
            db: Arc::new(Mutex::new(conn)),
            output_folder,
        };

        app_state.initialize_database().await?;

        Ok(app_state)
    }

    async fn initialize_database(&self) -> Result<()> {
        let conn = self.db.lock().await;
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
            "CREATE TABLE IF NOT EXISTS next_inode (
                id TEXT PRIMARY KEY,
                next_inode INTEGER
            )",
            [],
        )?;

        Ok(())
    }

    pub async fn save_to_sqlite(&self, fs: &ContentIndexer) -> Result<()> {
        let conn = self.db.lock().await;
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

    pub async fn save_next_inode(&self, next_inode: u64) -> Result<()> {
        let conn = self.db.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO next_inode (id, next_inode) 
             VALUES (?1, ?2)",
            params!["next_inode", next_inode],
        )?;

        Ok(())
    }

    pub async fn load_next_inode(&self) -> Result<u64> {
        let conn = self.db.lock().await;
        conn.query_row(
            "SELECT next_inode FROM next_inode WHERE id = 'next_inode'",
            [],
            |row| row.get(0),
        )
        .or_else(|_| {
            conn.execute(
                "INSERT INTO next_inode (id, next_inode) 
                 VALUES ('next_inode', 1)",
                [],
            )?;
            Ok(1)
        })
    }

    pub async fn get_indexed_images(&self) -> Result<Vec<String>> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT id FROM indexer")?;
        let image_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(image_names)
    }

    pub async fn get_image_data(
        &self,
        image_name: &str,
    ) -> Result<(
        HashMap<u64, HashMap<String, u64>>,
        HashMap<u64, FileAttr>,
        HashMap<u64, String>,
    )> {
        let conn = self.db.lock().await;

        let mut stmt = conn.prepare(
            "SELECT directory, file_attr, inode_to_hash 
             FROM indexer 
             WHERE id = ?1",
        )?;

        let (directory, file_attr, inode_to_hash) = stmt.query_row(params![image_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let directory: HashMap<u64, HashMap<String, u64>> = serde_json::from_str(&directory)?;
        let file_attr: HashMap<u64, FileAttr> = serde_json::from_str(&file_attr)?;
        let inode_to_hash: HashMap<u64, String> = serde_json::from_str(&inode_to_hash)?;

        Ok((directory, file_attr, inode_to_hash))
    }
}
