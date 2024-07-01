use bincode::{Decode, Encode};
use chrono::Utc;
use nix::sys::wait::waitpid;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};

use colored::*;

use crate::db::{Container, ContainerStatus};
use crate::namespace;
pub mod silo {
    tonic::include_proto!("silo");
}

#[derive(Encode, Decode, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct PythonInput {
    pub func: Vec<u8>,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
    pub hostname: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PythonOutput {
    pub output: Vec<u8>,
    pub hostname: String,
}

#[derive(Debug)]
pub struct TheSilo {
    pub container_path: String,
    pub host_link: String,
}

#[tonic::async_trait]
impl Silo for TheSilo {
    async fn get_package(
        &self,
        request: Request<GetPackageRequest>,
    ) -> Result<Response<GetPackageResponse>, Status> {
        // clear the terminal screen and reset the cursor to the top-left position
        print!("\x1B[2J\x1B[1;1H");

        let container_name = format!("container-{}", rand::random::<u32>());
        let mount_path = &format!("/tmp/{}", container_name);
        let start_time = std::time::Instant::now();

        std::fs::create_dir_all(mount_path).unwrap();

        let request_data = request.into_inner();

        // send the data to the HTTP server
        reqwest::Client::new()
            .put(format!("{}/api/inputs", self.host_link))
            .json(
                // bincode::encode_to_vec(
                &PythonInput {
                    func: request_data.func.clone(),
                    args: request_data.args.clone(),
                    kwargs: request_data.kwargs.clone(),
                    hostname: container_name.clone(),
                },
                //     bincode::config::standard(),
                // )
                // .unwrap(),
            )
            // .header("hostname", container_name.clone())
            .send()
            .await
            .unwrap();

        // check if the container path exists
        if !std::path::Path::new(&self.container_path).exists() {
            panic!("Container does not exist");
        }

        println!(
            "{}",
            format!("Running {}...", container_name).bright_yellow()
        );

        let child_pid = namespace::create_child(
            &self.container_path.clone(),
            mount_path,
            container_name.clone(),
            self.host_link.clone(),
        );

        match child_pid {
            Ok(pid) => {
                println!(
                    "{}",
                    format!("Container {} is running with PID {}", container_name, pid).green()
                );

                reqwest::Client::new()
                    .put(format!("{}/api/containers", self.host_link))
                    .json(&Container {
                        hostname: container_name.clone(),
                        status: ContainerStatus::Running,
                        start_time: Utc::now().timestamp(),
                        end_time: 00000,
                    })
                    .send()
                    .await
                    .unwrap();

                waitpid(pid, None).unwrap();
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Failed to run container {}: {}", container_name, e).red()
                );
            }
        }

        println!(
            "{}",
            format!(
                "Container {} has exited in {:?}ms",
                container_name,
                start_time.elapsed().as_millis()
            )
            .bright_yellow()
        );

        reqwest::Client::new()
            .patch(format!("{}/api/containers", self.host_link))
            .json(&Container {
                hostname: container_name.clone(),
                status: ContainerStatus::Stopped,
                start_time: 00000,
                end_time: Utc::now().timestamp(),
            })
            .send()
            .await
            .unwrap();

        let data: PythonOutput = serde_json::from_slice(
            &reqwest::Client::new()
                .get(format!("{}/api/outputs", self.host_link))
                // .header("hostname", container_name)
                .body(container_name)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap(),
        )
        .unwrap();

        // delete the container
        std::fs::remove_dir_all(mount_path).unwrap();

        let reply = silo::GetPackageResponse {
            output: data.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
