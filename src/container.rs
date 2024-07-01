use nix::{
    mount::{mount, MsFlags},
    sched::{clone, CloneFlags},
    sys::signal::Signal,
    unistd::{execvp, sethostname, Pid},
};
use nix::{
    mount::{umount2, MntFlags},
    sys::stat,
    unistd::{chdir, mkdir, pivot_root},
};
use std::path::{Path, PathBuf};
use std::{ffi::CString, fs::File, io::Write};

const STACK_SIZE: usize = 1024 * 1024;

fn child_func(
    container_path: &str,
    mount_path: &str,
    container_name: &str,
    task_id: i64,
    host_link: &str,
) -> isize {
    sethostname(container_name).unwrap();

    // Setup rootfs (implement this function based on your requirements)
    setup_rootfs(container_path, mount_path, container_name);

    let script = format!(
        r#"
import requests
import cloudpickle
import time
import socket
import base64

start = time.perf_counter()

url = "{host_link}/api/tasks/{task_id}"
response = requests.get(url)

if response.status_code == 200:
    task = response.json()
    
    func = cloudpickle.loads(base64.b64decode(task["func"]))
    args = cloudpickle.loads(base64.b64decode(task["args"]))
    kwargs = cloudpickle.loads(base64.b64decode(task["kwargs"]))
    output = func(*args, **kwargs)

    result = cloudpickle.dumps(output)

    requests.post("{host_link}/api/results/{task_id}", data=base64.b64encode(result))

    end = time.perf_counter() - start
    print(f"Python time taken: {{end * 1000:.2f}}ms")
else:
    print("Failed with status code:", response.status_code)
"#,
        host_link = host_link,
        task_id = task_id
    );

    std::env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin");

    let script_path = "/tmp/exec_func.py";
    let mut script_file = File::create(script_path).unwrap();
    script_file.write_all(script.as_bytes()).unwrap();

    let python = CString::new("/opt/bitnami/python/bin/python").unwrap();
    let script_cstr = CString::new(script_path).unwrap();
    execvp(&python, &[python.clone(), script_cstr]).unwrap();

    0
}

pub fn create_container(
    container_path: &str,
    container_name: &str,
    task_id: i64,
    host_link: &str,
) -> Result<Pid, nix::Error> {
    let mount_path = format!("/tmp/{}", container_name);
    std::fs::create_dir_all(&mount_path).unwrap();

    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .unwrap();

    let mut stack = [0; STACK_SIZE];

    let clone_flags = CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWCGROUP
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWUTS;

    unsafe {
        clone(
            Box::new(move || {
                child_func(
                    container_path,
                    &mount_path,
                    container_name,
                    task_id,
                    host_link,
                )
            }),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32),
        )
    }
}

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
        .unwrap_or_else(|e| println!("Failed to mount /dev/{}: {}", f, e))
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
