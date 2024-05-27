use bincode::{Decode, Encode};
use ipc_channel::ipc;
use nix::sys::wait::waitpid;
use redis::Commands;
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

#[derive(Encode, Decode, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct PythonInput {
    pub func: Vec<u8>,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        // println!("{}", "Received a request".bright_green());

        // let container_path = sub_m.get_one::<String>("CONTAINER").unwrap();
        let container_path = "/home/elden/Downloads/python";
        let container_name = format!("container-{}", rand::random::<u32>());
        let mount_path = &format!("/home/elden/Downloads/{}", container_name);
        std::fs::create_dir_all(mount_path).unwrap();

        // let host_link = format!(
        //     "http://172.18.{}.{}:8081",
        //     rand::random::<u8>(),
        //     rand::random::<u8>()
        // );
        let host_link = "http://172.18.0.1:8081".to_owned();
        // println!("{}", format!("Host link: {}", host_link).bright_yellow());
        // let redis_link = "redis://0.0.0.0:8080";
        let local_container_link = "http://0.0.0.0:8081";
        // let container_link = "172.18.0.4";
        let container_link = format!("172.18.{}.{}", rand::random::<u8>(), rand::random::<u8>());
        // let bridge_name = "isobr0";
        let bridge_name = format!("isobr{}", "0");
        // println!("{}", format!("Bridge name: {}", bridge_name).bright_yellow());
        let (tx, rx) = ipc::channel::<bool>().unwrap();

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
            .header("hostname", container_name.clone())
            .send()
            .await
            .unwrap();

        // check if the container path exists
        if !std::path::Path::new(&container_path).exists() {
            // println!("{}", format!("{} does not exist", container_path).red());
            panic!("Container does not exist");
        }

        // println!(
        //     "{}",
        //     format!("Running {}...", container_name).bright_yellow()
        // );

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
            mount_path,
            veth2_idx,
            container_name.clone(),
            container_link.to_owned(),
            host_link.clone(),
            &rx,
        );

        match child_pid {
            Ok(pid) => {
                // println!(
                //     "{}",
                //     format!("Container {} is running with PID {}", container_name, pid).green()
                // );
                // std::thread::sleep(std::time::Duration::from_secs(5));
                join_veth_to_ns(veth2_idx, pid.as_raw() as u32)
                    .await
                    .expect("Failed to join veth to namespace");
                // add the info that network setup is done
                // reqwest::Client::new()
                //     .put(format!("{}/network", local_container_link))
                //     .header("hostname", container_name.clone())
                //     .send()
                //     .await
                //     .unwrap();
                tx.send(true).unwrap();

                waitpid(pid, None).unwrap();
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to run container {}: {}", container_name, e).red()
                );
            }
        }

        // println!(
        //     "{}",
        //     format!("Container {} has exited", container_name).bright_red()
        // );

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

        // let data: PythonOutput = bincode::deserialize(
        //     &con.get(format!("output-{}", container_name))
        //         .unwrap()
        //         .as_bytes(),
        // );

        // delete the container
        std::fs::remove_dir_all(mount_path).unwrap();

        let reply = silo::GetPackageResponse {
            output: data.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
