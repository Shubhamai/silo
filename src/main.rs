mod container;
mod db;
mod filesystem;
mod grpc;
mod http;

use std::thread;

use actix_web::{web, App, HttpServer};
use clap::Command;
use colored::*;
use db::init_db;
use filesystem::SiloFS;
use grpc::{silo::silo_server::SiloServer, TheSilo};
use http::{configure_routes, AppState};
use tera::Tera;
use tokio::sync::Mutex;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let matches = Command::new("silo")
        .bin_name("silo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("[WIP] Build and deploy containers in seconds")
        .subcommand_required(true)
        .subcommand(
            Command::new("serve")
                .about("Start the Silo server to serve received requests, launch containers, and return responses")
                .args(&[
                    clap::Arg::new("gp")
                        .long("grpc-port")
                        .help("The port to run the gRPC server on")
                        .default_value("50051"),
                    clap::Arg::new("hp")
                        .long("http-port")
                        .help("The port to run the HTTP server on")
                        .default_value("8000"),
                    // clap::Arg::new("container-path")
                    //     .long("container-path")
                    //     .help("The path to the container directory")
                    //     .default_value("/home/elden/Downloads/python"),
                ]),
        )
        .subcommand(
            Command::new("fs").about("Index a container")
            // .args(&[
                // clap::Arg::new("name")
                //     .long("name")
                //     .help("The name of the container to index")
                //     .required(true),
                // clap::Arg::new("image")
                //     .long("image")
                //     .help("The image to index")
                //     .required(true),
            // ]),
        )
        // .subcommand(
        //     Command::new("filesystem").about("Run the FUSE filesystem").args(&[
        //         clap::Arg::new("addr")
        //             .long("addr")
        //             .help("The TCP address to listen on")
        //             .required(true),
        //         clap::Arg::new("mountpoint")
        //             .long("mountpoint")
        //             .help("The path to mount the filesystem on")
        //             .required(true),
        //     ]),
        // )
        .get_matches();

    match matches.subcommand() {
        Some(("serve", sub_matches)) => {
            let grpc_port: String = sub_matches.get_one::<String>("gp").unwrap().clone();
            let http_port: String = sub_matches.get_one::<String>("hp").unwrap().clone();

            let grpc_server_addr: String = format!("0.0.0.0:{}", grpc_port);
            let http_server_addr = format!("0.0.0.0:{}", &http_port);
            // let container_path: String = sub_matches
            //     .get_one::<String>("container-path")
            //     .unwrap()
            //     .clone();

            let tera = Tera::new("templates/**/*").unwrap();

            let conn = init_db().unwrap();

            let app_state = web::Data::new(AppState {
                templates: tera,
                db_connection: Mutex::new(conn),
            });

            let http_server = HttpServer::new(move || {
                App::new()
                    .app_data(app_state.clone())
                    .service(configure_routes())
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
