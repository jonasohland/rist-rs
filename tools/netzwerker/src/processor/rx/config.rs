use std::net::IpAddr;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SocketOption {
    ReusePort,
    ReuseAddress,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BindConfig {
    pub port: u16,
    pub address: Option<IpAddr>,
}

#[derive(Debug, Deserialize)]
pub struct JoinConfig {
    pub interfaces: Vec<IpAddr>,
    pub group: IpAddr,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bind: BindConfig,
    pub join: Option<JoinConfig>,
    _options: Option<Vec<SocketOption>>,
}
