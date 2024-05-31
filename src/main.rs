mod grpc;
mod http;
mod mount;
mod namespace;

use clap::Command;
use colored::*;
use dashmap::DashMap;
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
                ]),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("facility", sub_matches)) => {
            let grpc_port: String = sub_matches.get_one::<String>("gp").unwrap().clone();
            let http_port: String = sub_matches.get_one::<String>("hp").unwrap().clone();

            let grpc_server_addr: &String = &format!("[::1]:{}", grpc_port);
            let http_server_addr = format!("0.0.0.0:{}", &http_port);
            const WEB_URL: &str = "http://localhost:3000";
            const CONTAINER_PATH: &str = "/home/elden/Downloads/python";

            let thread_http_server_address = http_server_addr.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();

                rt.block_on(async {
                    let serv = std::sync::Arc::new(HttpServer {
                        address: thread_http_server_address.to_string(),
                        python_input_data: DashMap::new(),
                        python_result_data: DashMap::new(),
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

            println!(
                "{}",
                format!("gRPC server listening on {}...", grpc_server_addr).blue()
            );
            Server::builder()
                .add_service(SiloServer::new(TheSilo {
                    container_path: CONTAINER_PATH.to_string(),
                    host_link: format!("http://{}", http_server_addr),
                    web_url: WEB_URL.to_string(),
                }))
                .serve(grpc_server_addr.parse().unwrap())
                .await
                .unwrap();
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }
}
