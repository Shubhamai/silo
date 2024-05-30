mod grpc;
mod http;
mod mount;
mod namespace;

use clap::Command;
use colored::*;
use dashmap::{DashMap, DashSet};
use grpc::{silo::silo_server::SiloServer, TheSilo};

use http::HttpServer;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
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
            std::thread::spawn(move || {
                const HTTP_SERVER_ADDRESS: &str = "0.0.0.0:8081";

                let rt = tokio::runtime::Runtime::new().unwrap();

                rt.block_on(async {
                    let serv = std::sync::Arc::new(HttpServer {
                        address: HTTP_SERVER_ADDRESS.to_string(),
                        containers: DashMap::new(),
                        tasks: DashSet::new(),
                        results: DashMap::new(),
                    });

                    let listener = TcpListener::bind(serv.address.clone()).await.unwrap();
                    println!(
                        "{}",
                        format!("HTTP server listening on {}...", serv.address).blue()
                    );

                    loop {
                        let (stream, _) = listener.accept().await.unwrap();
                        let io = TokioIo::new(stream);

                        let serv = serv.clone();
                        let make_service = service_fn(move |r| {
                            let serv = serv.clone();

                            async move { serv.handle(r).await }
                        });

                        tokio::task::spawn(async move {
                            if let Err(err) = http1::Builder::new()
                                .serve_connection(io, make_service)
                                .await
                            {
                                println!(
                                    "{}",
                                    format!("Error serving connection: {:?}", err).red()
                                );
                            }
                        });
                    }
                })
            });

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
