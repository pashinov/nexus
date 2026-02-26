use std::process::ExitCode;

use clap::Parser;

use crate::cli::App;

mod api;
mod cli;
mod config;
mod redis;
mod sqlx;
mod utils;

fn main() -> ExitCode {
    if std::env::var("RUST_BACKTRACE").is_err() {
        // Enable backtraces on panics by default.
        // SAFETY: There is only a single thread at the moment.
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        // Disable backtraces in libraries by default
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
