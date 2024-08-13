use crate::db::{Output, Task};
use actix_web::{web, HttpResponse, Scope};
use rusqlite::Connection;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Template rendering error: {0}")]
    TemplateError(#[from] tera::Error),

    #[error("Not found")]
    NotFound,
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound => HttpResponse::NotFound().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

pub struct AppState {
    pub db_connection: Mutex<Connection>,
}

pub async fn add_task(
    data: web::Data<AppState>,
    task: web::Json<Task>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let task_id = task.insert(conn)?;
    Ok(HttpResponse::Ok().body(task_id.to_string()))
}

pub async fn get_task(
    data: web::Data<AppState>,
    task_id: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let task = Task::get(conn, task_id.into_inner())?.ok_or(AppError::NotFound)?;
    Ok(HttpResponse::Ok().json(task))
}

pub async fn add_result(
    data: web::Data<AppState>,
    task_id: web::Path<i64>,
    output: String,
) -> Result<HttpResponse, AppError> {
    let output = Output {
        task_id: task_id.into_inner(),
        output,
    };
    let conn = &data.db_connection.lock().await;
    output.insert(conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_result(
    data: web::Data<AppState>,
    task_id: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let output = Output::get(conn, task_id.into_inner())?.ok_or(AppError::NotFound)?;
    Ok(HttpResponse::Ok().json(output))
}

pub fn configure_routes() -> Scope {
    web::scope("/api")
        .route("/tasks", web::post().to(add_task))
        .route("/tasks/{task_id}", web::get().to(get_task))
        .route("/results/{task_id}", web::post().to(add_result))
        .route("/results/{task_id}", web::get().to(get_result))
}
