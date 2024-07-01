mod db;
mod grpc;
mod http;
mod mount;
mod namespace;

use actix_web::{web, App, HttpResponse, HttpServer};
use clap::Command;
use colored::*;
use db::init_db;
use grpc::{silo::silo_server::SiloServer, PythonInput, TheSilo};
use http::{
    add_container, dashboard, get_input, get_output, index, put_input, put_output,
    update_container, AppState,
};
use tera::Tera;
use tokio::sync::Mutex;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matches = Command::new("silo")
        .bin_name("silo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("[WIP] Build and deploy containers in seconds")
        .subcommand_required(true)
        .subcommand(
            Command::new("facility")
                .about("Run the facility to launch containers")
                .args(&[
                    clap::Arg::new("gp")
                        .long("grpc-port")
                        .help("The port to run the gRPC server on")
                        .default_value("50051"),
                    clap::Arg::new("hp")
                        .long("http-port")
                        .help("The port to run the HTTP server on")
                        .default_value("8000"),
                    clap::Arg::new("container-path")
                        .long("container-path")
                        .help("The path to the container directory")
                        .default_value("/home/elden/Downloads/python"),
                ]),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("facility", sub_matches)) => {
            let grpc_port: String = sub_matches.get_one::<String>("gp").unwrap().clone();
            let http_port: String = sub_matches.get_one::<String>("hp").unwrap().clone();

            let grpc_server_addr: String = format!("0.0.0.0:{}", grpc_port);
            let http_server_addr = format!("0.0.0.0:{}", &http_port);
            let container_path: String = sub_matches
                .get_one::<String>("container-path")
                .unwrap()
                .clone();

            let tera = Tera::new("templates/**/*").unwrap();

            let conn = init_db().unwrap();

            let app_state = web::Data::new(AppState {
                templates: tera,
                db_connection: Mutex::new(conn),
            });

            let http_server = HttpServer::new(move || {
                App::new()
                    .app_data(app_state.clone())
                    .route("/", web::get().to(index))
                    .route("/dashboard", web::get().to(dashboard))
                    .route("/api/inputs", web::put().to(put_input))
                    .route("/api/inputs", web::get().to(get_input))
                    .route("/api/outputs", web::put().to(put_output))
                    .route("/api/outputs", web::get().to(get_output))
                    .route("/api/containers", web::put().to(add_container))
                    .route("/api/containers", web::patch().to(update_container))
                    .service(actix_files::Files::new("/static", ".").show_files_listing())
            })
            .bind(&http_server_addr)?
            .run();

            println!(
                "{}",
                format!("HTTP server listening on {} ...", http_server_addr).blue()
            );

            let grpc_server = Server::builder()
                .add_service(SiloServer::new(TheSilo {
                    container_path,
                    host_link: format!("http://{}", http_server_addr),
                }))
                .serve(grpc_server_addr.parse().unwrap());

            println!(
                "{}",
                format!("gRPC server listening on {} ...", grpc_server_addr).blue()
            );

            tokio::select! {
                _ = http_server => println!("HTTP server exited"),
                _ = grpc_server => println!("gRPC server exited"),
            }
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }

    Ok(())
}
