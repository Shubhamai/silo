use std::{ffi::CString, path::Path};

use nix::{
    mount::{self, mount, umount2, MntFlags, MsFlags},
    sched::{clone, unshare, CloneFlags},
    sys::{signal::Signal, stat},
    unistd::{chdir, execvp, mkdir, setgid, sethostname, setresgid, setresuid, setuid, Pid},
};

use crate::mount::setup_rootfs;

const STACK_SIZE: usize = 1024 * 1024;

fn child_func(container_path: &str) -> isize {
    unshare(CloneFlags::CLONE_NEWNS).expect("Failed to unshare");

    sethostname("silo").unwrap();

    setup_rootfs(container_path);

    match mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    ) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to mount procfs: {}", e);
        }
    }

    mount(
        Some("tmpfs"),
        "/dev",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_RELATIME,
        None::<&str>,
    )
    .unwrap();

    ///////////////////////////////////////////////////////////////////////////
    // Change the effective user and group IDs to 0 (root)
    // setresuid(
    //     nix::unistd::Uid::from_raw(0),
    //     nix::unistd::Uid::from_raw(0),
    //     nix::unistd::Uid::from_raw(0),
    // )
    // .unwrap();
    setuid(nix::unistd::Uid::from_raw(0)).unwrap();
    // setresgid(
    //     nix::unistd::Gid::from_raw(0),
    //     nix::unistd::Gid::from_raw(0),
    //     nix::unistd::Gid::from_raw(0),
    // )
    // .unwrap();
    setgid(nix::unistd::Gid::from_raw(0)).unwrap();

    ///////////////////////////////////////////////////////////////////////////

    let shell = CString::new("/bin/nsenter").unwrap();
    execvp(&shell, &[shell.clone()]).expect("Failed to execute shell");

    // let shell = CString::new("/bin/echo").unwrap();
    // execvp(&shell, &[shell.clone(), CString::new("Hello").unwrap()])
    //     .expect("Failed to execute shell");

    mount::umount("proc").unwrap();

    0
}

pub fn create_child(container_path: &str) -> Result<Pid, nix::Error> {
    let mut stack = [0; STACK_SIZE];

    let clone_flags = CloneFlags::CLONE_NEWUSER
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWCGROUP
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWNET
        | CloneFlags::CLONE_NEWUTS;

    unsafe {
        clone(
            Box::new(|| child_func(container_path)),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32),
        )
    }
}
