mod namespace;
mod mount;

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    thread, time,
};

use clap::{Arg, Command};
use colored::*;
use nix::{
    sched::{unshare, CloneFlags},
    sys::wait::waitpid,
};

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
            let container_path = sub_m.get_one::<String>("CONTAINER").unwrap();

            if !std::path::Path::new(&container_path).exists() {
                println!("{}", format!("{} does not exist", container_path).red());
                return;
            }

            println!("{}", format!("Running {}...", container_path).green());

            let child_pid = namespace::create_child(container_path);

            match child_pid {
                Ok(pid) => {
                    println!(
                        "{}",
                        format!("Container {} is running with PID {}", container_path, pid).green()
                    );

                    //////////////////////////////////////////

                    let raw_pid = pid.as_raw();

                    write_mapping(&format!("/proc/{}/uid_map", raw_pid), 0, 1000, 1)
                        .expect("Failed to write UID mapping");
                    // Allow setting GID mappings by writing to /proc/[pid]/setgroups first
                    let setgroups_path = format!("/proc/{}/setgroups", raw_pid);
                    let mut setgroups_file = OpenOptions::new()
                        .write(true)
                        .open(&setgroups_path)
                        .expect("Failed to open setgroups file");
                    setgroups_file
                        .write_all(b"deny")
                        .expect("Failed to write to setgroups file");

                    write_mapping(&format!("/proc/{}/gid_map", raw_pid), 0, 1000, 1)
                        .expect("Failed to write GID mapping");
                    //////////////////////////////////////////

                    // give child process a chance to boot
                    thread::sleep(time::Duration::from_millis(300));

                    // wait for child process
                    waitpid(pid, None).unwrap();
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("Failed to run container {}: {}", container_path, e).red()
                    );
                }
            }
        }
        _ => {
            println!("{}", "No valid subcommand was used".red());
        }
    }
}

fn write_mapping(path: &str, inside_id: u32, outside_id: u32, length: u32) -> std::io::Result<()> {
    let mapping = format!("{} {} {}\n", inside_id, outside_id, length);
    let mut file = OpenOptions::new().write(true).open(path)?;
    file.write_all(mapping.as_bytes())?;
    Ok(())
}
