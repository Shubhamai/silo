use actix_web::{web, HttpResponse};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use rusqlite::{params, Connection};
use tera::Tera;
use tokio::sync::Mutex;

use crate::{
    db::{
        get_all_containers_db, get_all_tasks, get_python_input_db, get_python_output_db, Container,
        ContainerStatus,
    },
    grpc::PythonInput,
};

pub struct AppState {
    pub templates: Tera,
    pub db_connection: Mutex<Connection>,
}

pub async fn index(data: web::Data<AppState>) -> HttpResponse {
    let rendered = data
        .templates
        .render("base.html", &tera::Context::new())
        .unwrap();

    HttpResponse::Ok().body(rendered)
}

pub async fn dashboard(data: web::Data<AppState>) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    let inputs = get_all_tasks(&conn).unwrap();
    let containers = get_all_containers_db(&conn).unwrap();

    let mut context = tera::Context::new();
    context.insert("inputs", &inputs);
    context.insert("containers", &containers);
    let rendered = data.templates.render("dashboard.html", &context).unwrap();
    HttpResponse::Ok().body(rendered)
}

pub async fn put_input(data: web::Data<AppState>, input: web::Json<PythonInput>) -> HttpResponse {
    // Implementation to add a new input
    let conn = data.db_connection.lock().await;

    match conn.execute(
        "INSERT INTO tasks (hostname, func, args, kwargs) VALUES (?1, ?2, ?3, ?4)",
        params![
            &input.hostname,
            general_purpose::STANDARD.encode(&input.func),
            general_purpose::STANDARD.encode(&input.args),
            general_purpose::STANDARD.encode(&input.kwargs)
        ],
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!("Error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn get_input(data: web::Data<AppState>, hostname: String) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    match get_python_input_db(&conn, hostname) {
        Ok(input) => HttpResponse::Ok().json(input),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn put_output(
    data: web::Data<AppState>,
    output: web::Json<crate::grpc::PythonOutput>,
) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    match conn.execute(
        "INSERT INTO results (hostname, output) VALUES (?1, ?2)",
        params![
            &output.hostname,
            general_purpose::STANDARD.encode(&output.output)
        ],
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!("Error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn get_output(data: web::Data<AppState>, hostname: String) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    match get_python_output_db(&conn, hostname) {
        Ok(output) => match output {
            Some(output) => HttpResponse::Ok().json(output),
            None => HttpResponse::NotFound().finish(),
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn add_container(
    data: web::Data<AppState>,
    container: web::Json<Container>,
) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    match conn.execute(
        "INSERT INTO containers (hostname, status, start_time, end_time) VALUES (?1, ?2, ?3, ?4)",
        params![
            container.hostname,
            format!("{:?}", container.status),
            container.start_time,
            container.end_time
        ], // Utc::now().timestamp()
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!("Error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn update_container(
    data: web::Data<AppState>,
    container: web::Json<Container>,
) -> HttpResponse {
    let conn = data.db_connection.lock().await;

    match conn.execute(
        "UPDATE containers SET status = ?1, end_time = ?2 WHERE hostname = ?3",
        params![
            format!("{:?}", container.status),
            container.end_time,
            container.hostname
        ],
    ) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            eprintln!("Error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
