use anyhow::Result;
use log::{error, info};

use crate::database::AppState;
use crate::indexer::ContentIndexer;

pub async fn index_image(image_name: &str, state: &AppState) -> Result<()> {
    let start_time = std::time::Instant::now();

    info!("Pulling image: {}", image_name);
    pull_image(image_name)?;

    info!("Running container: {}", image_name);
    let container_id = run_container(image_name)?;

    info!("Adding custom file to container: {}", container_id);
    add_silo_script_to_container(&container_id)?;

    info!("Installing Python libraries in container: {}", container_id);
    install_python_libraries(&container_id)?;

    info!("Mounting container: {} ({})", image_name, container_id);
    let mount_path = mount_container(&container_id)?;

    let last_saved_inode = state.load_next_inode().await?;

    let mut fs = ContentIndexer::new(image_name, last_saved_inode, state.output_folder.clone());

    fs.total_files = walkdir::WalkDir::new(&mount_path).into_iter().count();
    info!("Total files to process: {}", fs.total_files);

    // Process files
    fs.save_directory(&mount_path, 0)?;

    let elapsed = start_time.elapsed().as_secs_f64();

    info!(
        "Saving data for image: {}, elapsed: {:.2}s",
        image_name, elapsed
    );

    state.save_to_sqlite(&fs).await?;
    state.save_next_inode(fs.next_inode).await?;

    info!("Image indexed successfully!");

    Ok(())
}

pub async fn list_images(state: &AppState) -> Result<()> {
    let image_names = state.get_indexed_images().await?;

    if image_names.is_empty() {
        println!("No images indexed yet.");
    } else {
        println!("Indexed images:");
        for (i, name) in image_names.iter().enumerate() {
            println!("{}. {}", i + 1, name);
        }
    }

    Ok(())
}

fn pull_image(image_name: &str) -> Result<()> {
    let output = std::process::Command::new("sudo")
        .args(&["podman", "pull", image_name])
        .output()?;

    if !output.status.success() {
        error!("Failed to pull image {}", image_name);
        anyhow::bail!("Failed to pull image {}", image_name);
    }
    Ok(())
}

fn run_container(image_name: &str) -> Result<String> {
    let output = std::process::Command::new("sudo")
        .args(&["podman", "run", "-dt", image_name])
        .output()?;

    if !output.status.success() {
        error!("Failed to run container for image {}", image_name);
        anyhow::bail!("Failed to run container");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn add_silo_script_to_container(container_id: &str) -> Result<()> {
    let silo_file_path = "./common/silo.py";
    let container_file_path = "/silo.py";

    // Copy file to container
    let output = std::process::Command::new("sudo")
        .args(&[
            "podman",
            "cp",
            silo_file_path,
            &format!("{}:{}", container_id, container_file_path),
        ])
        .output()?;

    if !output.status.success() {
        error!("Failed to copy custom file to container {}", container_id);
        anyhow::bail!("Failed to copy custom file to container");
    }

    info!("Successfully added custom file to container");
    Ok(())
}

fn install_python_libraries(container_id: &str) -> Result<()> {
    let libraries = ["requests", "cloudpickle"];

    for lib in libraries.iter() {
        let output = std::process::Command::new("sudo")
            .args(&["podman", "exec", container_id, "pip", "install", lib])
            .output()?;

        if !output.status.success() {
            error!(
                "Failed to install Python library {} in container {}",
                lib, container_id
            );
            anyhow::bail!("Failed to install Python library {}", lib);
        }
        info!("Successfully installed Python library: {}", lib);
    }
    Ok(())
}

fn mount_container(container_id: &str) -> Result<std::path::PathBuf> {
    let output = std::process::Command::new("sudo")
        .args(&["podman", "mount", container_id])
        .output()?;

    if !output.status.success() {
        error!("Failed to mount container {}", container_id);
        anyhow::bail!("Failed to mount container");
    }
    Ok(std::path::PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}
