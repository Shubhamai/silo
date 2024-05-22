use std::{ffi::CString, fs::File, io::Write, path::Path};

use nix::{
    mount::{self, mount, umount2, MntFlags, MsFlags},
    sched::{clone, unshare, CloneFlags},
    sys::{signal::Signal, stat},
    unistd::{chdir, execve, execvp, mkdir, setgid, sethostname, setresgid, setresuid, setuid, Pid},
};

use crate::mount::setup_rootfs;

const STACK_SIZE: usize = 1024 * 1024;

fn child_func(container_path: &str, func: Vec<u8>, args: Vec<u8>, kwargs: Vec<u8>) -> isize {
    // unshare(CloneFlags::CLONE_NEWUSER).expect("Failed to unshare");

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
    // setuid(nix::unistd::Uid::from_raw(0)).unwrap();
    // setresgid(
    //     nix::unistd::Gid::from_raw(0),
    //     nix::unistd::Gid::from_raw(0),
    //     nix::unistd::Gid::from_raw(0),
    // )
    // .unwrap();
    // setgid(nix::unistd::Gid::from_raw(0)).unwrap();

    ///////////////////////////////////////////////////////////////////////////

    // Write the serialized function to a temporary file
    let func_path = "/tmp/func.pkl";
    let mut file = File::create(func_path).unwrap();
    file.write_all(&func).unwrap();

    // args and kwargs are pickled and saved to /tmp/args.pkl and /tmp/kwargs.pkl
    let args_path = "/tmp/args.pkl";
    let mut args_file = File::create(args_path).unwrap();
    args_file.write_all(&args).unwrap();

    let kwargs_path = "/tmp/kwargs.pkl";
    let mut kwargs_file = File::create(kwargs_path).unwrap();
    kwargs_file.write_all(&kwargs).unwrap();

    
    let script = format!(
        r#"
import cloudpickle
with open("{func_path}", "rb") as f:
    func = cloudpickle.load(f)
# load args
with open("/tmp/args.pkl", "rb") as f:
    args = cloudpickle.load(f)
# load kwargs
with open("/tmp/kwargs.pkl", "rb") as f:
    kwargs = cloudpickle.load(f)
func(*args, **kwargs)"#,
        func_path = func_path
    );

    let script_path = "/tmp/exec_func.py";
    let mut script_file = File::create(script_path).unwrap();
    script_file.write_all(script.as_bytes()).unwrap();

    // Execute the Python script
    let python = CString::new("/opt/bitnami/python/bin/python").unwrap();
    let script_cstr = CString::new(script_path).unwrap();
    execvp(&python, &[python.clone(), script_cstr]).unwrap();

    // run /bin/bash
    // let bash = CString::new("/bin/bash").unwrap();
    // execvp(&bash, &[bash.clone()]).unwrap();
    // execve(path, args, env)

    mount::umount("proc").unwrap();

    0
}

pub fn create_child(
    container_path: &str,
    func: Vec<u8>,
    args: Vec<u8>,
    kwargs: Vec<u8>,
) -> Result<Pid, nix::Error> {
    let mut stack = [0; STACK_SIZE];

    

    let clone_flags = 
    // CloneFlags::CLONE_NEWUSER
    //     |
         CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWCGROUP
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWNET
        | CloneFlags::CLONE_NEWUTS;

    unsafe {
        clone(
            Box::new(|| child_func(container_path, func.clone(), args.clone(), kwargs.clone())),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32),
        )
    }
}
