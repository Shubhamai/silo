use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

mod args;
mod commands;
mod indexer;
mod server;
mod database;

use args::Args;
use commands::{index_image, list_images};
use server::run_tcp_server;
use database::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let output_folder = args.storage.clone();
    let db_path = args.db.clone();

    // Check if the output folder exists
    if !output_folder.exists() {
        std::fs::create_dir_all(&output_folder)?;
    }

    let app_state = AppState::new(db_path, output_folder).await?;

    // Start the TCP server in the background
    let tcp_server = tokio::spawn({
        let app_state = app_state.clone();
        async move {
            if let Err(e) = run_tcp_server(app_state, &args.host, args.port).await {
                error!("TCP server error: {:?}", e);
            }
        }
    });

    // Handle CLI commands
    match args.command {
        Some(args::Commands::Index { image_name }) => {
            info!("Indexing image: {}", image_name);
            index_image(&image_name, &app_state).await?;
        }
        Some(args::Commands::List) => {
            info!("Listing indexed images");
            list_images(&app_state).await?;
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    // Wait for the TCP server to finish (which it never will in this case)
    tcp_server.await?;

    Ok(())
}