use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Option<i64>,
    pub func: String,
    pub args: String,
    pub kwargs: String,
    pub func_str: String,
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
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            func TEXT NOT NULL,
            args TEXT NOT NULL,
            kwargs TEXT NOT NULL,
            func_str TEXT NOT NULL
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

impl Task {
    pub fn insert(&self, conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO tasks (func, args, kwargs, func_str) VALUES (?1, ?2, ?3, ?4)",
            params![self.func, self.args, self.kwargs, self.func_str],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get(conn: &Connection, id: i64) -> Result<Option<Task>> {
        conn.query_row(
            "SELECT id, func, args, kwargs, func_str FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                Ok(Task {
                    id: Some(row.get(0)?),
                    func: row.get(1)?,
                    args: row.get(2)?,
                    kwargs: row.get(3)?,
                    func_str: row.get(4)?,
                })
            },
        )
        .optional()
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
}
