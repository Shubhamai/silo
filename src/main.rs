mod grpc;
mod http;
mod mount;
mod namespace;
mod net;

use clap::Command;
use colored::*;
use grpc::{silo::silo_server::SiloServer, TheSilo};

use http::http_server;
use tonic::transport::Server;

#[tokio::main]
async fn main() {
    let matches = Command::new("silo")
        .bin_name("silo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("[WIP] Build and deploy containers in seconds")
        .subcommand_required(true)
        .subcommand(Command::new("facility").about("Run the facility to launch containers"))
        .get_matches();

    match matches.subcommand() {
        Some(("facility", _)) => {
            http_server("0.0.0.0:8080".to_string());

            Server::builder()
                .add_service(SiloServer::new(TheSilo {}))
                .serve("[::1]:50051".parse().unwrap())
                .await
                .unwrap();
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }
}
