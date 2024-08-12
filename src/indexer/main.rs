use anyhow::Result;
use clap::Parser;
use log::{error, info};
use tokio::io::{self, AsyncBufReadExt};
mod args;
mod commands;
mod database;
mod indexer;
mod server;
use args::Args;
use commands::{index_image, list_images};
use database::AppState;
use server::run_tcp_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse arguments
    let args = Args::parse();
    let output_folder = args.storage.clone();
    let db_path = args.db.clone();

    // Ensure output folder exists
    if !output_folder.exists() {
        std::fs::create_dir_all(&output_folder)?;
    }

    // Initialize application state
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

    // Spawn a task to handle CLI commands concurrently
    let command_handler = tokio::spawn(handle_commands(app_state));

    // Wait for both the TCP server and command handler to finish
    let _ = tokio::try_join!(tcp_server, command_handler)?;

    Ok(())
}

async fn handle_commands(app_state: AppState) {
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Enter a command (type 'help' for available commands):");

    while let Some(line) = lines.next_line().await.unwrap_or(None) {
        let trimmed_line = line.trim();

        if trimmed_line.is_empty() {
            continue; // Ignore empty lines
        }

        match parse_command(trimmed_line) {
            Command::List => {
                info!("Listing indexed images");
                if let Err(e) = list_images(&app_state).await {
                    error!("Error listing images: {:?}", e);
                }
            }
            Command::Index(image_name) => {
                info!("Indexing image: {}", image_name);
                println!("Indexing image: {}", image_name);
                if let Err(e) = index_image(&image_name, &app_state).await {
                    error!("Error indexing image: {:?}", e);
                }
            }
            Command::Help => {
                print_help();
            }
            Command::Unknown(cmd) => {
                println!(
                    "Unknown command: '{}'. Type 'help' for a list of available commands.",
                    cmd
                );
            }
        }

        println!("\nEnter a command:");
    }
}

enum Command {
    List,
    Index(String),
    Help,
    Unknown(String),
}

fn parse_command(input: &str) -> Command {
    let mut parts = input.split_whitespace();
    match parts.next() {
        Some("ls") => Command::List,
        Some("index") => {
            let image_name = parts.collect::<Vec<&str>>().join(" ");
            if image_name.is_empty() {
                println!("Usage: index <image_name>");
                Command::Unknown("index".to_string())
            } else {
                Command::Index(image_name)
            }
        }
        Some("help") => Command::Help,
        Some(cmd) => Command::Unknown(cmd.to_string()),
        None => Command::Unknown(String::new()),
    }
}

fn print_help() {
    println!(
        "\nAvailable commands:
    ls              - List indexed images
    index <name>    - Index an image by name
    help            - Show this help message\n"
    );
}
