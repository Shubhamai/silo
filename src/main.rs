use clap::{Arg, Command};
use colored::*;

fn main() {
    let matches = Command::new("silo")
        .bin_name("silo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("[WIP] Build and deploy containers in seconds")
        .subcommand_required(true)
        .subcommand(
            Command::new("run").about("Run a container").arg(
                Arg::new("CONTAINER")
                    .help("The path of container to run")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("run", sub_m)) => {
            let script = sub_m.get_one::<String>("CONTAINER").unwrap();
            println!("{}", format!("Running {}...", script).green());
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }
}
