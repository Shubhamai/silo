use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::{Container, ContainerStatus, Function, Output, Task};
use actix_web::{web, HttpResponse, Scope};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tera::Tera;
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

    #[error("Internal server error")]
    InternalServerError,
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
    pub templates: Tera,
    pub db_connection: Mutex<Connection>,
}

pub async fn index(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rendered = data.templates.render("base.html", &tera::Context::new())?;
    Ok(HttpResponse::Ok().body(rendered))
}

#[derive(Serialize, Deserialize)]
struct ContainerWithUptime {
    hostname: String,
    status: ContainerStatus,
    uptime: i64,
}

pub async fn dashboard(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let conn = data.db_connection.lock().await;
    let tasks = Task::get_all(&conn)?;
    let containers = Container::get_all(&conn)?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let containers_with_uptime: Vec<ContainerWithUptime> = containers
        .into_iter()
        .map(|container| {
            let uptime = if container.status == ContainerStatus::Running {
                current_time - container.start_time
            } else {
                container.end_time - container.start_time
            };
            ContainerWithUptime {
                hostname: container.hostname,
                status: container.status,
                uptime,
            }
        })
        .collect();

    let mut context = tera::Context::new();
    context.insert("tasks", &tasks);
    context.insert("containers", &containers_with_uptime);
    let rendered = data.templates.render("dashboard.html", &context)?;
    Ok(HttpResponse::Ok().body(rendered))
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

pub async fn add_function(
    data: web::Data<AppState>,
    function: web::Json<Function>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    function.insert(conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_function(
    data: web::Data<AppState>,
    function_id: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let function = Function::get(conn, function_id.into_inner())?.ok_or(AppError::NotFound)?;
    Ok(HttpResponse::Ok().json(function))
}

pub async fn get_functions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let functions = Function::get_all(conn)?;
    Ok(HttpResponse::Ok().json(functions))
}

pub async fn add_container(
    data: web::Data<AppState>,
    container: web::Json<Container>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    container.insert(conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn update_container(
    data: web::Data<AppState>,
    container: web::Json<Container>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    container.update_status_and_endtime(conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_container(
    data: web::Data<AppState>,
    hostname: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let conn = &data.db_connection.lock().await;
    let container = Container::get(conn, &hostname)?.ok_or(AppError::NotFound)?;
    Ok(HttpResponse::Ok().json(container))
}

pub async fn add_result(
    data: web::Data<AppState>,
    task_id: web::Path<i64>,
    output: String,
) -> Result<HttpResponse, AppError> {
    let output = Output {
        task_id: task_id.into_inner(),
        output,
        stdout: Some("null".to_string()),
        stderr: Some("null".to_string()),
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
    web::scope("")
        .route("/", web::get().to(index))
        .route("/dashboard", web::get().to(dashboard))
        .route("/api/functions", web::post().to(add_function))
        .route("/api/functions/{function_id}", web::get().to(get_function))
        .route("/api/functions", web::get().to(get_functions))
        .route("/api/tasks", web::post().to(add_task))
        .route("/api/tasks/{task_id}", web::get().to(get_task))
        .route("/api/containers", web::put().to(add_container))
        .route(
            "/api/containers/{hostname}",
            web::patch().to(update_container),
        )
        .route("/api/containers/{hostname}", web::get().to(get_container))
        .route("/api/results/{task_id}", web::post().to(add_result))
        .route("/api/results/{task_id}", web::get().to(get_result))
}
