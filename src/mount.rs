use std::path::Path;

use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sys::stat,
    unistd::{chdir, mkdir, pivot_root as nix_pivot_root},
};
pub fn setup_rootfs(container_path: &str) {
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .unwrap();

    match mount(
        Some(container_path),
        container_path,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    ) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to bind mount rootfs: {}", e);
        }
    }

    let prev_rootfs = Path::new(container_path).join(".oldroot");
    match std::fs::remove_dir_all(&prev_rootfs) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to remove old rootfs: {}", e);
        }
    }
    mkdir(
        &prev_rootfs,
        stat::Mode::S_IRWXU | stat::Mode::S_IRWXG | stat::Mode::S_IRWXO,
    )
    .unwrap();

    match nix_pivot_root(container_path, &prev_rootfs) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to pivot root: {}", e);
        }
    };

    chdir("/").unwrap();

    match umount2(".oldroot", MntFlags::MNT_DETACH) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to unmount old rootfs: {}", e);
        }
    };
}
