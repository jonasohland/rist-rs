use std::{collections::HashMap, fs};

use serde::Deserialize;

use crate::processor::{delay, drop, rx, splitter, tx};
use anyhow::{Context, Result};
use clap;

#[derive(clap::Parser)]
pub struct CmdLine {
    #[clap(short, long)]
    pub config: String,

    #[clap(long, short, rename_all = "lowercase")]
    pub log_level: Option<tracing::Level>,

    #[clap(short, long)]
    pub threads: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProcessorConfigs {
    Drop(drop::Config),
    Splitter(splitter::Config),
    Tx(tx::Config),
    Rx(rx::Config),
    Delay(delay::Config),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub threads: Option<usize>,
    pub proc: HashMap<String, ProcessorConfigs>,
}

impl Config {
    pub fn load(cmd_line: &CmdLine) -> Result<Self> {
        let mut cfg: Config =
            toml::from_str(&fs::read_to_string(&cmd_line.config).with_context(|| {
                format!("failed to read config file from: {}", cmd_line.config)
            })?)
            .with_context(|| format!("failed to parse config file from: {}", cmd_line.config))?;
        if let Some(t) = cmd_line.threads {
            cfg.threads = Some(t)
        }
        Ok(cfg)
    }
}
