use crate::grpc::{PythonInput, PythonOutput};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ContainerStatus {
    Running,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Container {
    pub hostname: String,
    pub status: ContainerStatus,
    pub start_time: i64,
    pub end_time: i64,
}

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("silo.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            func TEXT NOT NULL,
            args TEXT NOT NULL,
            kwargs TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS containers (
            hostname TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            hostname TEXT PRIMARY KEY,
            func TEXT NOT NULL,
            args TEXT NOT NULL,
            kwargs TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS results (
            hostname TEXT PRIMARY KEY,
            output TEXT NOT NULL
        )",
        [],
    )?;

    Ok(conn)
}

pub fn get_python_input_db(conn: &Connection, hostname: String) -> Result<Option<PythonInput>> {
    let mut stmt =
        conn.prepare("SELECT func, args, kwargs, hostname FROM tasks WHERE hostname = ?")?;
    let mut rows = stmt.query(params![hostname])?;

    if let Some(row) = rows.next()? {
        let func_base64: String = row.get(0)?;
        let args_base64: String = row.get(1)?;
        let kwargs_base64: String = row.get(2)?;
        let hostname: String = row.get(3)?;
        let func = general_purpose::STANDARD.decode(func_base64).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let args = general_purpose::STANDARD.decode(args_base64).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let kwargs = general_purpose::STANDARD
            .decode(kwargs_base64)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok(Some(PythonInput {
            func,
            args,
            kwargs,
            hostname,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_all_tasks(conn: &Connection) -> Result<Vec<PythonInput>> {
    let mut stmt = conn.prepare("SELECT func, args, kwargs, hostname FROM tasks")?;
    let rows = stmt.query_map([], |row| {
        let func_base64: String = row.get(0)?;
        let args_base64: String = row.get(1)?;
        let kwargs_base64: String = row.get(2)?;
        let hostname: String = row.get(3)?;
        let func = general_purpose::STANDARD.decode(func_base64).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let args = general_purpose::STANDARD.decode(args_base64).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let kwargs = general_purpose::STANDARD
            .decode(kwargs_base64)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok(PythonInput {
            func,
            args,
            kwargs,
            hostname,
        })
    })?;

    let mut inputs = Vec::new();
    for input in rows {
        inputs.push(input?);
    }

    Ok(inputs)
}

pub fn get_python_output_db(conn: &Connection, hostname: String) -> Result<Option<PythonOutput>> {
    let mut stmt = conn.prepare("SELECT output FROM results WHERE hostname = ?")?;
    let mut rows = stmt.query(params![hostname])?;
    if let Some(row) = rows.next()? {
        let output_base64: String = row.get(0)?;
        let output = general_purpose::STANDARD
            .decode(output_base64)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok(Some(PythonOutput { output, hostname }))
    } else {
        Ok(None)
    }
}

pub fn get_all_containers_db(conn: &Connection) -> Result<Vec<Container>> {
    let mut stmt = conn.prepare("SELECT hostname, status, start_time, end_time FROM containers")?;
    let rows = stmt.query_map([], |row| {
        let hostname: String = row.get(0)?;
        let status: String = row.get(1)?;
        let start_time: i64 = row.get(2)?;
        let end_time: i64 = row.get(3)?;
        // let start_time: i64 = row.get(2)?;
        // let uptime = Utc::now().timestamp() - start_time;
        Ok(Container {
            hostname,
            status: match status.as_str() {
                "Running" => ContainerStatus::Running,
                "Stopped" => ContainerStatus::Stopped,
                _ => unreachable!(),
            },
            start_time,
            end_time,
        })
    })?;

    let mut containers = Vec::new();
    for container in rows {
        containers.push(container?);
    }

    Ok(containers)
}
