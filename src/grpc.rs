use crate::container::run_podman_container;
use crate::db::{Output, Task};
use crate::filesystem::silofs::SiloFS;
use colored::*;
use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};
use tonic::{Request, Response, Status};

pub mod silo {
    tonic::include_proto!("silo");
}

pub struct TheSilo {
    pub host_link: String,
    pub filesystem: SiloFS,
}

#[tonic::async_trait]
impl Silo for TheSilo {
    async fn get_package(
        &self,
        request: Request<GetPackageRequest>,
    ) -> Result<Response<GetPackageResponse>, Status> {
        // clear the terminal screen and reset the cursor to the top-left position
        print!("\x1B[2J\x1B[1;1H");
        let start_time = std::time::Instant::now();

        let request_data = request.into_inner();

        let container_name = format!("container-{}", rand::random::<u32>());
        let mount_path = &format!("/tmp/{}", container_name);

        println!(
            "{}",
            format!("Creating container {}...", container_name).bright_yellow()
        );

        std::fs::create_dir_all(mount_path).unwrap();

        self.filesystem
            .mount(&request_data.image_name, mount_path)
            .unwrap();

        // send the data to the HTTP server
        let task_id = reqwest::Client::new()
            .post(format!("{}/api/tasks", self.host_link))
            .json(&Task {
                id: None,
                func: request_data.func,
                args: request_data.args,
                kwargs: request_data.kwargs,
                func_str: request_data.func_str,
            })
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
            .parse::<i64>()
            .unwrap();

        println!(
            "{}",
            format!("Running {}...", container_name).bright_yellow()
        );

        let container_result = run_podman_container(task_id, &self.host_link, mount_path)
            .await
            .unwrap();

        println!(
            "{}",
            format!(
                "Container {} has exited in {:?}ms",
                container_name,
                start_time.elapsed().as_millis()
            )
            .bright_yellow()
        );

        let python_result = reqwest::Client::new()
            .get(format!("{}/api/results/{}", self.host_link, task_id))
            .body(container_name)
            .send()
            .await
            .unwrap()
            .json::<Output>()
            .await
            .unwrap();

        let reply = silo::GetPackageResponse {
            result: python_result.output,
            stdout: String::from_utf8_lossy(&container_result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&container_result.stderr).to_string(),
        };
        Ok(Response::new(reply))
    }
}
