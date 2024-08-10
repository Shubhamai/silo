use std::process::Command;
use std::{fs::File, io::Write};

pub fn run_podman_container(
    // container_path: &str,
    container_name: &str,
    task_id: i64,
    host_link: &str,
    mount_path: &str,
) -> std::io::Result<()> {
    // Create the Python script
    let script = r#"
import requests
import cloudpickle
import time
import socket
import base64
import os

start = time.perf_counter()

host_link = os.environ.get("HOST_LINK")
task_id = os.environ.get("TASK_ID")

url = f"{host_link}/api/tasks/{task_id}"
response = requests.get(url)

if response.status_code == 200:
    task = response.json()
    
    func = cloudpickle.loads(base64.b64decode(task["func"]))
    args = cloudpickle.loads(base64.b64decode(task["args"]))
    kwargs = cloudpickle.loads(base64.b64decode(task["kwargs"]))
    
    output = func(*args, **kwargs)

    result = cloudpickle.dumps(output)

    requests.post(f"{host_link}/api/results/{task_id}", data=base64.b64encode(result))

    end = time.perf_counter() - start
    print(f"Python time taken: {end * 1000:.2f}ms")
else:
    print("Failed with status code:", response.status_code)
"#;

    let script_path = format!("/tmp/silo.py");
    let container_script_path = "/silo.py";
    let mut script_file = File::create(script_path.clone())?;
    script_file.write_all(script.as_bytes())?;

    // Run the Podman command
    let status = Command::new("podman")
        .args(&[
            "run",
            "--name",
            container_name,
            "-v",
            &format!("{}:{}", script_path, container_script_path),
            "-e",
            &format!("HOST_LINK={}", host_link),
            "-e",
            &format!("TASK_ID={}", task_id),
            "--hostname",
            container_name,
            "--network",
            "host",
            "--rootfs",
            mount_path,
            // "python:3.10",
            "/bin/bash",
            "-c",
            &format!(
                "pip install requests cloudpickle && python3 {}",
                container_script_path
            ),
        ])
        // .output()?;
        .status()?;

    if status.success() {
        println!("Container executed successfully");
    } else {
        eprintln!("Container execution failed");
    }

    // println!("Container executed successfully {:?}", status);

    Ok(())
}
