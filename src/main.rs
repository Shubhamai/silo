mod grpc;
mod mount;
mod namespace;

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    thread, time,
};

use clap::{Arg, Command};
use colored::*;
use grpc::{silo::silo_server::SiloServer, TheSilo};
use nix::{
    sched::{unshare, CloneFlags},
    sys::wait::waitpid,
};
use tonic::transport::Server;

#[tokio::main]
async fn main() {
    let matches = Command::new("silo")
        .bin_name("silo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("[WIP] Build and deploy containers in seconds")
        .subcommand_required(true)
        .subcommand(
            Command::new("facility").about("Run the facility to launch containers"), // .arg(
                                                                                     //     Arg::new("CONTAINER")
                                                                                     //         .help("The path of container to run")
                                                                                     //         .required(true)
                                                                                     //         .index(1),
                                                                                     // ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("facility", sub_m)) => {
            let addr = "[::1]:50051".parse().unwrap();
            let silo = TheSilo::default();

            Server::builder()
                .add_service(SiloServer::new(silo))
                .serve(addr)
                .await
                .unwrap();
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }
}
