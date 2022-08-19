use clap::Parser;
use config::{CmdLine, Config};
use ctl::ControlProcessor;

pub mod config;
pub mod ctl;
pub mod engine;
pub mod error;
pub mod packet;
pub mod processor;
pub mod signal;
pub mod util;

async fn async_main(cfg: Config) {
    match ControlProcessor::try_new(&cfg).await {
        Ok(k) => k.join().await.unwrap(),
        Err(e) => {
            tracing::error!("startup failed: {:?}", e);
        }
    };
}

fn main() {
    let cmd_line = CmdLine::parse();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(cmd_line.log_level.unwrap_or(tracing::Level::ERROR))
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    match Config::load(&cmd_line) {
        Ok(cfg) => {
            let num_threads = 2usize;
            let rt = match num_threads {
                1 => tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build(),
                num => tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(num)
                    .enable_all()
                    .build(),
            }
            .unwrap();
            rt.block_on(async_main(cfg))
        }
        Err(e) => {
            tracing::error!("failed to start: {:?}", e)
        }
    }
}
