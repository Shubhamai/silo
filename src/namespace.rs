use std::{ffi::CString, fs::File, io::Write,  };

use nix::{
    mount::{ mount, MsFlags}, sched::{clone, CloneFlags}, sys::signal::Signal, unistd::{ execvp, sethostname,  Pid}
};

use crate::mount::setup_rootfs;

const STACK_SIZE: usize = 1024 * 1024;

 fn child_func(container_path: &str, 
mount_path: &str,
container_name: String,
host_link: String,
) -> isize {

    sethostname(&container_name).unwrap();
    // unshare(CloneFlags::CLONE_NEWUSER).expect("Failed to unshare");

    setup_rootfs(container_path, mount_path, &container_name);
    
    // requests, cloudpickle, grpcio, google, 
    let script = format!(
        r#"
import requests
import cloudpickle
import time
import socket

start = time.perf_counter()

url = "{host_link}/data"
response = requests.get(url, headers={{"hostname": socket.gethostname()}})

if response.status_code == 200:
    res = response.json()

    func = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["func"]))
    )
    args = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["args"]))
    )
    kwargs = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["kwargs"]))
    )
    output = func(*args, **kwargs)

    result = cloudpickle.dumps(output)

    data = {{'output': list(result)}}

    response = requests.put("{host_link}/output", json=data, headers={{"hostname": socket.gethostname()}})    
    end = time.perf_counter() - start
    print(f"Python time taken: {{end * 1000:.2f}}ms")


else:
    print("Failed with status code:", response.status_code)    
"#,
host_link = host_link
    );

    std::env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin");

    let script_path = "/tmp/exec_func.py";
    let mut script_file = File::create(script_path).unwrap();
    script_file.write_all(script.as_bytes()).unwrap();

    // let python = CString::new("/opt/bitnami/python/bin/python").unwrap();
    // let script_cstr = CString::new(script_path).unwrap();
    // execvp(&python, &[python.clone(), script_cstr]).unwrap();


    ///////////////////////////////////////////////////////////////////////////

    // run /bin/bash
    let bash = CString::new("/bin/bash").unwrap();
    execvp(&bash, &[bash.clone()]).unwrap();

    // mount::umount("proc").unwrap();

    0
}

pub fn create_child(
    container_path: &str,
    mount_path: &str,
    container_name: String,
    host_link: String,
) -> Result<Pid, nix::Error> {
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .unwrap();

    let mut stack = [0; STACK_SIZE];

    let clone_flags = 
    // CloneFlags::CLONE_NEWUSER
    //     |
         CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWCGROUP
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC
        // | CloneFlags::CLONE_NEWNET
        | CloneFlags::CLONE_NEWUTS
        ;

    unsafe {
        clone(
            Box::new(   move  || child_func(container_path, 
                mount_path,
            container_name.clone(),
            host_link.clone(),
            )),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32),
        )
    }
}
