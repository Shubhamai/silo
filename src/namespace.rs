use std::{ffi::CString, fs::File, io::Write,  };

use nix::{
    mount::{self, mount, MsFlags}, sched::{clone, CloneFlags}, sys::signal::Signal, unistd::{ execvp, sethostname,  Pid}
};

use crate::{mount::setup_rootfs, net::setup_veth_peer};

const STACK_SIZE: usize = 1024 * 1024;

 fn child_func(container_path: &str, 
veth2_idx : u32,
container_name: &str,
container_link: String,
host_link: &str,
) -> isize {

    sethostname(container_name).unwrap();
    setup_rootfs(container_path);
    // unshare(CloneFlags::CLONE_NEWUSER).expect("Failed to unshare");

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



std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let container_link = container_link.clone();

    rt.block_on(async {
        setup_veth_peer(veth2_idx, &container_link, 16).await.unwrap();
    });
}).join().unwrap();
    
    let script = format!(
        r#"
import requests
import cloudpickle
import time

start = time.perf_counter()

url = "{host_link}/data"
response = requests.get(url, headers={{"hostname": "{container_name}"}})

if response.status_code == 200:
    res = response.json()

    func = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["func"]))
    )
    print(bytes.fromhex("".join(format(x, "02x") for x in res["args"])))

    args = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["args"]))
    )
    kwargs = cloudpickle.loads(
        bytes.fromhex("".join(format(x, "02x") for x in res["kwargs"]))
    )
    output = func(*args, **kwargs)

    result = cloudpickle.dumps(output)

    data = {{'output': list(result)}}

    # send the result back
    response = requests.put("{host_link}/output", json=data, headers={{"hostname": "{container_name}"}})

    if response.status_code == 200:
        print("Success!")


else:
    print("Failed with status code:", response.status_code)

end = time.perf_counter() - start
# print ms
print(f"Python time taken: {{end * 1000:.2f}}ms")
"#,
host_link = host_link
    );

    let script_path = "/tmp/exec_func.py";
    let mut script_file = File::create(script_path).unwrap();
    script_file.write_all(script.as_bytes()).unwrap();

    let python = CString::new("/opt/bitnami/python/bin/python").unwrap();
    let script_cstr = CString::new(script_path).unwrap();
    execvp(&python, &[python.clone(), script_cstr]).unwrap();


    ///////////////////////////////////////////////////////////////////////////

    // run /bin/bash
    // let bash = CString::new("/bin/bash").unwrap();
    // execvp(&bash, &[bash.clone()]).unwrap();
    // execve(path, args, env)

    mount::umount("proc").unwrap();

    0
}

pub fn create_child(
    container_path: &str,
    veth2_idx : u32,
    container_name: &str,
    container_link: String,
    host_link: &str,
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
            Box::new(   move  || child_func(container_path, 
            veth2_idx,
            container_name,
            container_link.clone(),
            host_link,
            )),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32),
        )
    }
}
