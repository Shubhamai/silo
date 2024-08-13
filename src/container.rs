use std::process::Command;

pub async fn run_podman_container(
    task_id: i64,
    host_link: &str,
    mount_path: &str,
) -> std::io::Result<std::process::Output> {
    let script_path = "silo.py";

    // Run the Podman command
    let status = Command::new("podman")
        .args(&[
            "run",
            "-e",
            &format!("HOST_LINK={}", host_link),
            "-e",
            &format!("TASK_ID={}", task_id),
            "--network",
            "host",
            "--rootfs",
            mount_path,
            "python3",
            script_path,
        ])
        .output()?;

    println!("Container exited with status: {:?}", status);

    Ok(status)
}
