use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;

#[derive(Clone, Debug, Deserialize)]
pub struct SendConfig {
    address: IpAddr,
    port: u16,
}

impl SendConfig {
    fn sock_addr(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MulticastConfig {
    interface: Option<Ipv4Addr>,
    join: Option<bool>,
    loopback: Option<bool>,
}

impl Default for MulticastConfig {
    fn default() -> Self {
        Self {
            interface: None,
            join: Some(false),
            loopback: Some(false),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub inputs: Vec<String>,
    send: SendConfig,
    multicast: Option<MulticastConfig>,
}

impl Config {
    async fn connect(&self, sock: UdpSocket) -> Result<UdpSocket> {
        let destination = self.send.sock_addr();
        tracing::info!(?destination, "connect udp socket");
        sock.connect(destination)
            .await
            .context("connect() failed")?;
        Ok(sock)
    }

    fn set_outgoing_interface(&self, sock: socket2::Socket) -> Result<socket2::Socket> {
        if let Some(multicast) = &self.multicast {
            if let Some(interface) = multicast.interface {
                tracing::debug!(?interface, "set outgoing multicast interface");
                sock.set_multicast_if_v4(&interface)
                    .with_context(|| format!("failed to set multicast interface ({interface})"))?;
            }
        }
        Ok(sock)
    }

    fn join_group(&self, sock: socket2::Socket) -> Result<socket2::Socket> {
        if let Some(multicast) = &self.multicast {
            if multicast.join.unwrap_or(false) {
                let dest = self.send.sock_addr().ip();
                if !dest.is_multicast() {
                    Err(anyhow!("cannot join multicast group because destination address ({}) is non-multicast", dest))?;
                }
                match dest {
                    IpAddr::V6(_) => Err(anyhow!("join ipv6 multicast not supported"))?,
                    IpAddr::V4(group) => {
                        let interface = match multicast.interface {
                            None => Ipv4Addr::new(0, 0, 0, 0),
                            Some(interface) => interface,
                        };
                        tracing::info!(?group, ?interface, "join multicast group");
                        sock.join_multicast_v4(&group, &interface)
                            .with_context(|| {
                                format!(
                                    "failed to join multicast group {group} on interface {interface}"
                                )
                            })?;
                    }
                }
            }
        }
        Ok(sock)
    }

    fn set_loopback(&self, sock: socket2::Socket) -> Result<socket2::Socket> {
        if let Some(multicast) = &self.multicast {
            if let Some(value) = multicast.loopback {
                tracing::debug!(value, "set multicast loopback option");
                sock.set_multicast_loop_v4(value)
                    .context("failed to set multicast loopback socket option")?;
            }
        }
        Ok(sock)
    }

    pub async fn socket(&self) -> Result<UdpSocket> {
        self.connect(
            UdpSocket::from_std(
                self.join_group(
                    UdpSocket::bind("0.0.0.0:0")
                        .await?
                        .into_std()
                        .context("failed to extract underlying socket")?
                        .into(),
                )
                .and_then(|socket| self.set_outgoing_interface(socket))
                .and_then(|socket| self.set_loopback(socket))
                .context("failed to create udp socket")?
                .into(),
            )
            .context("failed to create tokio udp socket")?,
        )
        .await
    }
}
