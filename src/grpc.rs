use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;
use std::{thread, time};

use nix::sys::wait::waitpid;
use tonic::{transport::Server, Request, Response, Status};

use silo::silo_server::{Silo, SiloServer};
use silo::{GetPackageRequest, GetPackageResponse};

use colored::*;

use crate::namespace;

pub mod silo {
    tonic::include_proto!("silo"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct TheSilo {}

#[tonic::async_trait]
impl Silo for TheSilo {
    async fn get_package(
        &self,
        request: Request<GetPackageRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<GetPackageResponse>, Status> {
        let request_data = request.into_inner();

        // let container_path = sub_m.get_one::<String>("CONTAINER").unwrap();
        let container_path = "/home/elden/Downloads/python";

        if !std::path::Path::new(&container_path).exists() {
            println!("{}", format!("{} does not exist", container_path).red());
            panic!("Container does not exist");
        }

        println!("{}", format!("Running {}...", container_path).green());

        let child_pid = namespace::create_child(
            container_path,
            request_data.func,
            request_data.args,
            request_data.kwargs,
        );

        match child_pid {
            Ok(pid) => {
                println!(
                    "{}",
                    format!("Container {} is running with PID {}", container_path, pid).green()
                );

                //////////////////////////////////////////

                // let raw_pid = pid.as_raw();

                // write_mapping(&format!("/proc/{}/uid_map", raw_pid), 0, 1000, 1)
                //     .expect("Failed to write UID mapping");
                // // Allow setting GID mappings by writing to /proc/[pid]/setgroups first
                // let setgroups_path = format!("/proc/{}/setgroups", raw_pid);
                // let mut setgroups_file = OpenOptions::new()
                //     .write(true)
                //     .open(&setgroups_path)
                //     .expect("Failed to open setgroups file");
                // setgroups_file
                //     .write_all(b"deny")
                //     .expect("Failed to write to setgroups file");

                // write_mapping(&format!("/proc/{}/gid_map", raw_pid), 0, 1000, 1)
                //     .expect("Failed to write GID mapping");
                //////////////////////////////////////////

                // give child process a chance to boot
                // thread::sleep(time::Duration::from_millis(300));

                // wait for child process
                waitpid(pid, None).unwrap();
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to run container {}: {}", container_path, e).red()
                );
            }
        }

        println!("{}", format!("Container {} has exited", container_path).green());

        let reply = silo::GetPackageResponse {
            name: 434, // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

fn write_mapping(path: &str, inside_id: u32, outside_id: u32, length: u32) -> std::io::Result<()> {
    let mapping = format!("{} {} {}\n", inside_id, outside_id, length);
    let mut file = OpenOptions::new().write(true).open(path)?;
    file.write_all(mapping.as_bytes())?;
    Ok(())
}
