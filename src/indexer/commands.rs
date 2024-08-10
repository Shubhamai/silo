use anyhow::Result;
use tracing::{info, error};

use crate::database::AppState;
use crate::indexer::ContentIndexer;

pub async fn index_image(image_name: &str, state: &AppState) -> Result<()> {
    let start_time = std::time::Instant::now();

    info!("Pulling image: {}", image_name);
    pull_image(image_name)?;

    info!("Running container: {}", image_name);
    let container_id = run_container(image_name)?;

    info!("Mounting container: {} ({})", image_name, container_id);
    let mount_path = mount_container(&container_id)?;

    let conn = state.db.lock().await;
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