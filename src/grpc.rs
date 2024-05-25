use bincode::{Decode, Encode};
use nix::sys::wait::waitpid;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};

use colored::*;

use crate::namespace;
use crate::net::{join_veth_to_ns, prepare_net};
pub mod silo {
    tonic::include_proto!("silo");
}

#[derive(Encode, Decode, PartialEq, Debug, Deserialize, Serialize)]
pub struct PythonInput {
    pub func: Vec<u8>,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PythonOutput {
    pub output: Vec<u8>,
}

#[derive(Debug)]
pub struct TheSilo {}

#[tonic::async_trait]
impl Silo for TheSilo {
    async fn get_package(
        &self,
        request: Request<GetPackageRequest>,
    ) -> Result<Response<GetPackageResponse>, Status> {
        // clear the terminal screen and reset the cursor to the top-left position
        print!("\x1B[2J\x1B[1;1H");
        // let container_path = sub_m.get_one::<String>("CONTAINER").unwrap();
        let container_path = "/home/elden/Downloads/python";
        let container_name = &format!("container-{}", rand::random::<u32>());
        let host_link = "http://172.18.0.1:8080";
        let local_container_link = "http://0.0.0.0:8080";
        let container_link = "172.18.0.4";
        let bridge_name = "isobr0";

        let request_data = request.into_inner();

        // send the data to the HTTP server
        reqwest::Client::new()
            .put(format!("{}/data", local_container_link))
            .body(
                bincode::encode_to_vec(
                    PythonInput {
                        func: request_data.func.clone(),
                        args: request_data.args.clone(),
                        kwargs: request_data.kwargs.clone(),
                    },
                    bincode::config::standard(),
                )
                .unwrap(),
            )
            .header("hostname", container_name)
            .send()
            .await
            .unwrap();

        // check if the container path exists
        if !std::path::Path::new(&container_path).exists() {
            println!("{}", format!("{} does not exist", container_path).red());
            panic!("Container does not exist");
        }

        println!(
            "{}",
            format!("Running {} {}...", container_name, container_path).bright_yellow()
        );

        let (_, _, veth2_idx) = prepare_net(
            bridge_name.to_string(),
            host_link
                .split("//")
                .nth(1)
                .unwrap()
                .split(":")
                .nth(0)
                .unwrap(),
            16,
        )
        .await
        .expect("Failed to prepare network");

        let child_pid = namespace::create_child(
            container_path,
            veth2_idx,
            container_name,
            container_link.to_owned(),
            &host_link,
        );

        match child_pid {
            Ok(pid) => {
                println!(
                    "{}",
                    format!("Container {} is running with PID {}", container_path, pid).green()
                );

                join_veth_to_ns(veth2_idx, pid.as_raw() as u32)
                    .await
                    .expect("Failed to join veth to namespace");

                waitpid(pid, None).unwrap();
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to run container {}: {}", container_path, e).red()
                );
            }
        }

        println!(
            "{}",
            format!("Container {} has exited", container_path).bright_red()
        );

        let data: PythonOutput = serde_json::from_slice(
            &reqwest::Client::new()
                .get(format!("{}/output", local_container_link))
                .header("hostname", container_name)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap(),
        )
        .unwrap();

        let reply = silo::GetPackageResponse {
            output: data.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
