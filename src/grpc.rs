use bincode::{Decode, Encode};
use nix::sys::wait::waitpid;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};

use colored::*;

use crate::http;
use crate::namespace;

pub mod silo {
    tonic::include_proto!("silo");
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
        let container_name = format!("container-{}", rand::random::<u32>());
        let mount_path = &format!("/home/elden/Downloads/{}", container_name);
        std::fs::create_dir_all(mount_path).unwrap();
        let host_link = "http://0.0.0.0:8081".to_owned();

        let request_data = request.into_inner();
        let request_id = rand::random::<i32>();

        // send the data to the HTTP server
        let to_launch = reqwest::Client::new()
            .put(format!("{}/tasks", host_link))
            .body(
                bincode::encode_to_vec(
                    http::PythonInput {
                        request_id,
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
        match to_launch.text().await.unwrap().as_str() {
            "launch" => {
                // check if the container path exists
                if !std::path::Path::new(&container_path).exists() {
                    panic!("Container does not exist");
                }

                println!(
                    "{}",
                    format!("Running {}...", container_name).bright_yellow()
                );

                let child_pid = namespace::create_child(
                    container_path,
                    mount_path,
                    container_name.clone(),
                    host_link.clone(),
                );

                match child_pid {
                    Ok(pid) => {
                        // println!(
                        //     "{}",
                        //     format!("Container {} is running with PID {}", container_name, pid)
                        //         .green()
                        // );

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
            }
            _ => {
                println!("Using Existing Container");
            }
        }

        let output_response_data = reqwest::Client::new()
            .get(format!("{}/output", host_link))
            .header("hostname", container_name)
            .header("request_id", request_id)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let data: http::PythonOutput = serde_json::from_slice(&output_response_data).unwrap();

        // delete the container
        std::fs::remove_dir_all(mount_path).unwrap();

        let reply = silo::GetPackageResponse {
            output: data.output, //vec![1], //data.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
