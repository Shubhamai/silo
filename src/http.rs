use crate::db::{Container, ContainerStatus, Function, Output, Task};
use actix_web::{web, HttpResponse, Scope};
use clap::builder::Str;
use rusqlite::Connection;
use tera::Tera;
use tokio::sync::Mutex;

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
    let tasks = Task::get_all(&conn).unwrap();
    let containers = Container::get_all(&conn).unwrap();

    let mut context = tera::Context::new();
    context.insert("tasks", &tasks);
    context.insert("containers", &containers);
    let rendered = data.templates.render("dashboard.html", &context).unwrap();
    HttpResponse::Ok().body(rendered)
}

pub async fn add_task(data: web::Data<AppState>, task: web::Json<Task>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;

    match task.insert(conn) {
        Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
        Err(e) => {
            eprintln!("Error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn get_task(data: web::Data<AppState>, task_id: web::Path<i64>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match Task::get(conn, task_id.into_inner()) {
        Ok(task) => HttpResponse::Ok().json(task.unwrap()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

pub async fn add_function(
    data: web::Data<AppState>,
    function: web::Json<Function>,
) -> HttpResponse {
    let conn = &data.db_connection.lock().await;

    match function.insert(conn) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_function(data: web::Data<AppState>, function_id: web::Path<i64>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match Function::get(conn, function_id.into_inner()) {
        Ok(function) => HttpResponse::Ok().json(function.unwrap()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

pub async fn get_functions(data: web::Data<AppState>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match Function::get_all(conn) {
        Ok(functions) => HttpResponse::Ok().json(functions),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn add_container(
    data: web::Data<AppState>,
    container: web::Json<Container>,
) -> HttpResponse {
    let conn = &data.db_connection.lock().await;

    match container.insert(conn) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn updt_cntr(data: web::Data<AppState>, container: web::Json<Container>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match container.update_status_and_endtime(conn) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_container(data: web::Data<AppState>, hostname: web::Path<String>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match Container::get(conn, &hostname) {
        Ok(container) => HttpResponse::Ok().json(container.unwrap()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

// pub async fn get_random_task(data: web::Data<AppState>) -> HttpResponse {
//     let conn = &data.db_connection.lock().await;
//     match Task::get_random(conn) {
//         Ok(Some(task)) => HttpResponse::Ok().json(task),
//         Ok(None) => HttpResponse::NotFound().finish(),
//         Err(_) => HttpResponse::InternalServerError().finish(),
//     }
// }

pub async fn add_result(
    data: web::Data<AppState>,
    task_id: web::Path<i64>,
    output: String,
) -> HttpResponse {
    let output = Output {
        task_id: task_id.into_inner(),
        output,
    };

    let conn = &data.db_connection.lock().await;

    match output.insert(conn) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_result(data: web::Data<AppState>, task_id: web::Path<i64>) -> HttpResponse {
    let conn = &data.db_connection.lock().await;
    match Output::get(conn, task_id.into_inner()) {
        Ok(output) => HttpResponse::Ok().json(output.unwrap()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
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
        // .route("/api/tasks/random", web::get().to(get_random_task))
        .route("/api/containers", web::put().to(add_container))
        .route("/api/containers/{hostname}", web::patch().to(updt_cntr))
        .route("/api/containers/{hostname}", web::get().to(get_container))
        .route("/api/results/{task_id}", web::post().to(add_result))
        .route("/api/results/{task_id}", web::get().to(get_result))
}
