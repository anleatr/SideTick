#![cfg_attr(not(test), windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]

mod model;
mod storage;
mod windows;

fn main() {
    if let Err(error) = windows::run() {
        windows::show_startup_error(&error.to_string());
        std::process::exit(1);
    }
}
