use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nexus_utils as utils;
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::service;

#[derive(Parser)]
#[clap(name = "kafka-producer")]
#[clap(version = version_string())]
#[clap(subcommand_required = true, arg_required_else_help = true)]
pub struct App {
    #[clap(subcommand)]
    cmd: Cmd,
}

impl App {
    pub fn run(self) -> Result<()> {
        self.cmd.run()
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Start service.
    Run(CmdRun),
}

impl Cmd {
    fn run(self) -> Result<()> {
        match self {
            Cmd::Run(cmd) => cmd.run(),
        }
    }
}

#[derive(Parser)]
struct CmdRun {
    /// Path to the service config.
    #[clap(short, long)]
    config: Option<PathBuf>,

    /// Path to the logger config.
    #[clap(short, long)]
    logger_config: Option<PathBuf>,
}

impl CmdRun {
    fn run(self) -> Result<()> {
        let config: AppConfig = match self.config.as_ref() {
            Some(path) => {
                utils::serde::load_json_from_file(path).context("failed to load config")?
            }
            None => AppConfig::default(),
        };

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(utils::signal::run_with_shutdown(|token| {
                self.run_impl(config, token)
            }))
    }

    async fn run_impl(self, config: AppConfig, token: CancellationToken) -> Result<()> {
        utils::logger::init_logger(&config.logger, self.logger_config)?;
        utils::logger::set_abort_with_tracing();

        service::run_service(config, token).await
    }
}

fn version_string() -> &'static str {
    static STRING: OnceLock<String> = OnceLock::new();
    STRING.get_or_init(|| {
        format!("(release {KAFKA_PRODUCER_VERSION}) (rustc {KAFKA_PRODUCER_RUSTC_VERSION})")
    })
}

static KAFKA_PRODUCER_VERSION: &str = env!("KAFKA_PRODUCER_VERSION");
static KAFKA_PRODUCER_RUSTC_VERSION: &str = env!("KAFKA_PRODUCER_RUSTC_VERSION");
