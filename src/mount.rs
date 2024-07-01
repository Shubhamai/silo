use std::path::{Path, PathBuf};

use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sys::stat,
    unistd::{chdir, mkdir, pivot_root},
};

// From https://github.com/managarm/cbuildrt/blob/main/src/main.rs#L57
fn concat_absolute<L: AsRef<Path>, R: AsRef<Path>>(lhs: L, rhs: R) -> PathBuf {
    lhs.as_ref().join(rhs.as_ref().strip_prefix("/").unwrap())
}

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

    // From https://github.com/managarm/cbuildrt/blob/main/src/main.rs#L57
    let dev_overlays = vec!["tty", "null", "zero", "full", "random", "urandom"];
    for f in dev_overlays {

        // if mount fails, print error message, but continue
        nix::mount::mount(
            Some(&Path::new("/dev/").join(f)),
            &concat_absolute(mount_path, "/dev/").join(f),
            None::<&str>,
            nix::mount::MsFlags::MS_BIND,
            None::<&str>,
        )
        .unwrap_or_else(
            |e| println!("Failed to mount /dev/{}: {}", f, e)
        )
        
    }

    nix::mount::mount(
        Some(&std::fs::canonicalize("/etc/resolv.conf").unwrap()),
        &concat_absolute(mount_path, "/etc/resolv.conf"),
        None::<&str>,
        nix::mount::MsFlags::MS_BIND,
        None::<&str>,
    )
    .expect("failed to mount /etc/resolv.conf");

    nix::mount::mount(
        None::<&str>,
        &concat_absolute(mount_path, "/dev/pts"),
        Some("devpts"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .expect("failed to mount /dev/pts");

    nix::mount::mount(
        None::<&str>,
        &concat_absolute(mount_path, "/dev/shm"),
        Some("tmpfs"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .expect("failed to mount /dev/shm");

    nix::mount::mount(
        None::<&str>,
        &concat_absolute(mount_path, "/run"),
        Some("tmpfs"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .expect("failed to mount /run");

    nix::mount::mount(
        None::<&str>,
        &concat_absolute(mount_path, "/tmp"),
        Some("tmpfs"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .expect("failed to mount /tmp");

    nix::mount::mount(
        None::<&str>,
        &concat_absolute(mount_path, "/proc"),
        Some("proc"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .expect("failed to mount /proc");

    mount(
        Some("tmpfs"),
        "/dev",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_RELATIME,
        None::<&str>,
    )
    .unwrap();

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
