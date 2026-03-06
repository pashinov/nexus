use std::process::ExitCode;

use clap::Parser;

use crate::cli::App;

mod cli;
mod config;
mod kafka;
mod mqtt;
mod service;
mod storage;

fn main() -> ExitCode {
    if std::env::var("RUST_BACKTRACE").is_err() {
        // SAFETY: There is only a single thread at the moment.
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        // SAFETY: There is only a single thread at the moment.
        unsafe { std::env::set_var("RUST_LIB_BACKTRACE", "0") };
    }

    match App::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:?}");
            ExitCode::FAILURE
        }
    }
}
