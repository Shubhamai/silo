use std::str::FromStr;

use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub id: Option<i64>,
    pub name: String,
    pub function: String,
    pub function_str: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Option<i64>,
    pub func: String,
    pub args: String,
    pub kwargs: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ContainerStatus {
    Starting,
    Running,
    Completed,
    Failed,
}

impl FromStr for ContainerStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Starting" => Ok(ContainerStatus::Starting),
            "Running" => Ok(ContainerStatus::Running),
            "Completed" => Ok(ContainerStatus::Completed),
            "Failed" => Ok(ContainerStatus::Failed),
            _ => Err(()),
        }
    }
}

impl ToString for ContainerStatus {
    fn to_string(&self) -> String {
        match self {
            ContainerStatus::Starting => "Starting".to_string(),
            ContainerStatus::Running => "Running".to_string(),
            ContainerStatus::Completed => "Completed".to_string(),
            ContainerStatus::Failed => "Failed".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Container {
    pub hostname: String,
    pub status: ContainerStatus,
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub task_id: i64,
    pub output: String,
}

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("silo.db")?;

    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            function TEXT NOT NULL,
            function_str TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
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
        "CREATE TABLE IF NOT EXISTS results (
            task_id INTEGER PRIMARY KEY,
            output TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        )",
        [],
    )?;

    Ok(conn)
}

impl Function {
    pub fn insert(&self, conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO functions (name, function, function_str) VALUES (?1, ?2, ?3)",
            params![self.name, self.function, self.function_str],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get(conn: &Connection, id: i64) -> Result<Option<Function>> {
        conn.query_row(
            "SELECT id, name, function, function_str FROM functions WHERE id = ?1",
            params![id],
            |row| {
                Ok(Function {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    function: row.get(2)?,
                    function_str: row.get(3)?,
                })
            },
        )
        .optional()
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Function>> {
        let mut stmt = conn.prepare("SELECT id, name, function, function_str FROM functions")?;
        let rows = stmt.query_map([], |row| {
            Ok(Function {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                function: row.get(2)?,
                function_str: row.get(3)?,
            })
        })?;

        let mut functions = Vec::new();
        for function in rows {
            functions.push(function?);
        }

        Ok(functions)
    }

    pub fn update(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE functions SET name = ?1, function = ?2 function_str = ?3 WHERE id = ?4",
            params![self.name, self.function, self.function_str, self.id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        conn.execute("DELETE FROM functions WHERE id = ?1", params![id])?;
        Ok(())
    }
}

impl Task {
    pub fn insert(&self, conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO tasks (func, args, kwargs) VALUES (?1, ?2, ?3)",
            params![self.func, self.args, self.kwargs],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get(conn: &Connection, id: i64) -> Result<Option<Task>> {
        conn.query_row(
            "SELECT id, func, args, kwargs FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                Ok(Task {
                    id: Some(row.get(0)?),
                    func: row.get(1)?,
                    args: row.get(2)?,
                    kwargs: row.get(3)?,
                })
            },
        )
        .optional()
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Task>> {
        let mut stmt = conn.prepare("SELECT id, func, args, kwargs FROM tasks")?;
        let rows = stmt.query_map([], |row| {
            Ok(Task {
                id: Some(row.get(0)?),
                func: row.get(1)?,
                args: row.get(2)?,
                kwargs: row.get(3)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }

        Ok(tasks)
    }

    pub fn update(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE tasks SET func = ?1, args = ?2, kwargs = ?3 WHERE id = ?4",
            params![self.func, self.args, self.kwargs, self.id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }
}

impl Container {
    pub fn insert(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "INSERT INTO containers (hostname, status, start_time, end_time) VALUES (?1, ?2, ?3, ?4)",
            params![self.hostname, self.status.to_string(), self.start_time, self.end_time],
        )?;
        Ok(())
    }

    pub fn get(conn: &Connection, hostname: &str) -> Result<Option<Container>> {
        conn.query_row(
            "SELECT hostname, status, start_time, end_time FROM containers WHERE hostname = ?1",
            params![hostname],
            |row| {
                let status_str: String = row.get(1)?;
                Ok(Container {
                    hostname: row.get(0)?,
                    status: ContainerStatus::from_str(&status_str)
                        .unwrap_or(ContainerStatus::Failed),
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                })
            },
        )
        .optional()
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Container>> {
        let mut stmt =
            conn.prepare("SELECT hostname, status, start_time, end_time FROM containers")?;
        let container_iter = stmt.query_map([], |row| {
            let status_str: String = row.get(1)?;
            Ok(Container {
                hostname: row.get(0)?,
                status: ContainerStatus::from_str(&status_str).unwrap_or(ContainerStatus::Failed),
                start_time: row.get(2)?,
                end_time: row.get(3)?,
            })
        })?;

        container_iter.collect()
    }

    pub fn update_status_and_endtime(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE containers SET status = ?1, end_time = ?2 WHERE hostname = ?3",
            params![self.status.to_string(), self.end_time, self.hostname],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, hostname: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM containers WHERE hostname = ?1",
            params![hostname],
        )?;
        Ok(())
    }
}

impl Output {
    pub fn insert(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "INSERT INTO results (task_id, output) VALUES (?1, ?2)",
            params![self.task_id, self.output],
        )?;
        Ok(())
    }

    pub fn get(conn: &Connection, task_id: i64) -> Result<Option<Output>> {
        conn.query_row(
            "SELECT task_id, output FROM results WHERE task_id = ?1",
            params![task_id],
            |row| {
                Ok(Output {
                    task_id: row.get(0)?,
                    output: row.get(1)?,
                })
            },
        )
        .optional()
    }

    pub fn update(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE results SET output = ?1 WHERE task_id = ?2",
            params![self.output, self.task_id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, task_id: i64) -> Result<()> {
        conn.execute("DELETE FROM results WHERE task_id = ?1", params![task_id])?;
        Ok(())
    }
}
