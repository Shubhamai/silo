use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    #[arg(short, long, default_value = "./data/content")]
    pub storage: PathBuf,

    #[arg(short, long, default_value = "./data/indexer.db")]
    pub db: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[clap(name = "index", about = "Index a podman image")]
    Index { image_name: String },

    #[clap(name = "list", about = "List indexed podman images")]
    List,
}