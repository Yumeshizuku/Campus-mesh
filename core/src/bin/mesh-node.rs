use campus_mesh::{ffi::*, network::now};
use std::{
    ffi::{CStr, CString},
    io::{self, BufRead},
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("mesh-node STATE_PATH NICKNAME TCP_PORT [--discover]\nJSON commands on stdin: state/contact/send/peer/relay");
        std::process::exit(2);
    }
    unsafe {
        let path = CString::new(args[1].clone()).unwrap();
        let nick = CString::new(args[2].clone()).unwrap();
        let h = cm_open(
            path.as_ptr(),
            nick.as_ptr(),
            args[3].parse().expect("port"),
            args.iter().any(|x| x == "--discover"),
        );
        if h.is_null() {
            eprintln!("startup failed: check state file and TCP port");
            std::process::exit(1);
        }
        eprintln!("Campus Mesh running at {}", now());
        for line in io::stdin().lock().lines() {
            let line = line.unwrap();
            if line == "quit" {
                break;
            }
            if let Ok(c) = CString::new(line) {
                let out = cm_call(h, c.as_ptr());
                println!("{}", CStr::from_ptr(out).to_string_lossy());
                cm_free(out);
            }
        }
        cm_close(h);
    }
}
