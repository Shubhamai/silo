use crate::container::create_container;
use crate::db::{Container, ContainerStatus, Function, Output, Task};
use bincode::{Decode, Encode};
use chrono::Utc;
use colored::*;
use nix::sys::wait::waitpid;
use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};
use tonic::{Request, Response, Status};

pub mod silo {
    tonic::include_proto!("silo");
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

        // save function to the database
        reqwest::Client::new()
            .post(format!("{}/api/functions", self.host_link))
            .json(&Function {
                id: None,
                name: request_data.func.clone(),
                function: request_data.func.clone(),
                function_str: request_data.func_str.clone(),
            })
            .send()
            .await
            .unwrap();

        // send the data to the HTTP server
        let task_id = reqwest::Client::new()
            .post(format!("{}/api/tasks", self.host_link))
            .json(&Task {
                id: None,
                func: request_data.func,
                args: request_data.args,
                kwargs: request_data.kwargs,
            })
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
            .parse::<i64>()
            .unwrap();

        // check if the container path exists
        if !std::path::Path::new(&self.container_path).exists() {
            panic!("Container does not exist");
        }

        println!(
            "{}",
            format!("Running {}...", container_name).bright_yellow()
        );

        let child_pid = create_container(
            &self.container_path,
            &container_name,
            task_id,
            &self.host_link,
        );

        reqwest::Client::new()
            .put(format!("{}/api/containers", self.host_link))
            .json(&Container {
                hostname: container_name.clone(),
                status: ContainerStatus::Starting,
                start_time: Utc::now().timestamp_millis(),
                end_time: Utc::now().timestamp_millis(),
            })
            .send()
            .await
            .unwrap();

        match child_pid {
            Ok(pid) => {
                println!(
                    "{}",
                    format!("Container {} is running with PID {}", container_name, pid).green()
                );

                reqwest::Client::new()
                    .patch(format!(
                        "{}/api/containers/{}",
                        self.host_link, container_name
                    ))
                    .json(&Container {
                        hostname: container_name.clone(),
                        status: ContainerStatus::Running,
                        start_time: Utc::now().timestamp_millis(),
                        end_time: Utc::now().timestamp_millis(),
                    })
                    .send()
                    .await
                    .unwrap();

                waitpid(pid, None).unwrap();
            }
            Err(e) => {
                reqwest::Client::new()
                    .patch(format!(
                        "{}/api/containers/{}",
                        self.host_link, container_name
                    ))
                    .json(&Container {
                        hostname: container_name.clone(),
                        status: ContainerStatus::Failed,
                        start_time: Utc::now().timestamp_millis(),
                        end_time: Utc::now().timestamp_millis(),
                    })
                    .send()
                    .await
                    .unwrap();

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
            .patch(format!(
                "{}/api/containers/{}",
                self.host_link, container_name
            ))
            .json(&Container {
                hostname: container_name.clone(),
                status: ContainerStatus::Completed,
                start_time: Utc::now().timestamp_millis(),
                end_time: Utc::now().timestamp_millis(),
            })
            .send()
            .await
            .unwrap();

        let output = reqwest::Client::new()
            .get(format!("{}/api/results/{}", self.host_link, task_id))
            .body(container_name)
            .send()
            .await
            .unwrap()
            .json::<Output>()
            .await
            .unwrap();

        // delete the container
        std::fs::remove_dir_all(mount_path).unwrap();

        let reply = silo::GetPackageResponse {
            output: output.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
