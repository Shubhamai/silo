use bincode::{Decode, Encode};
use nix::sys::wait::waitpid;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use silo::silo_server::Silo;
use silo::{GetPackageRequest, GetPackageResponse};

use colored::*;

use crate::namespace;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedData {
    pub data: String,
    pub key: String,
}

#[derive(Debug)]
pub struct TheSilo {
    pub container_path: String,
    pub host_link: String,
    pub web_url: String,
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
        std::fs::create_dir_all(mount_path).unwrap();

        let request_data = request.into_inner();

        // curl data from request_data.cid
        let encrypted_data = reqwest::Client::new()
            .get(format!(
                "https://gateway.lighthouse.storage/ipfs/{}",
                request_data.cid
            ))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let decrypted_data = reqwest::Client::new()
            .patch(format!("{}/api/upload", self.web_url))
            .body(
                serde_json::to_string(&EncryptedData {
                    data: encrypted_data,
                    key: request_data.key,
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap();

        let data: PythonInput =
            serde_json::from_slice(&decrypted_data.bytes().await.unwrap()).unwrap();
        // send the data to the HTTP server
        reqwest::Client::new()
            .put(format!("{}/data", self.host_link))
            .body(bincode::encode_to_vec(data, bincode::config::standard()).unwrap())
            .header("hostname", container_name.clone())
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
            format!("Container {} has exited", container_name).bright_red()
        );

        let data: PythonOutput = serde_json::from_slice(
            &reqwest::Client::new()
                .get(format!("{}/output", self.host_link))
                .header("hostname", container_name)
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
            output: data.output, //vec![1], //data.output,
            errors: "errors".to_string(),
        };
        Ok(Response::new(reply))
    }
}
