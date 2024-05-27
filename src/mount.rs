use std::path::Path;

use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sys::stat,
    unistd::{chdir, mkdir, pivot_root},
};
pub fn setup_rootfs(container_path: &str, mount_path: &str, container_name: &str) {
    match mount(
        Some(container_path),
        mount_path,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    ) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to bind mount rootfs: {}", e);
        }
    }

    let prev_rootfs = Path::new(mount_path).join(format!(".oldroot{}", container_name));
    // match std::fs::remove_dir_all(&prev_rootfs) {
    //     Ok(_) => {}
    //     Err(e) => {
    //         println!("Failed to remove old rootfs: {}", e);
    //     }
    // }
    mkdir(
        &prev_rootfs,
        stat::Mode::S_IRWXU | stat::Mode::S_IRWXG | stat::Mode::S_IRWXO,
    )
    .unwrap();

    match pivot_root(mount_path, &prev_rootfs) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to pivot root: {}", e);
        }
    };

    chdir("/").unwrap();

    let new_rootfs_path: &str = &format!(".oldroot{}", container_name);
    match umount2(new_rootfs_path, MntFlags::MNT_DETACH) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to unmount old rootfs: {}", e);
        }
    };
}
