use std::net::IpAddr;

use common::log::LogConfig;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::server_id::ServerId;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server_ip: IpAddr,
    pub servers: Vec<ServerId>,
    pub server_port: u16,
    pub client_port: u16,
    pub logs: LogConfig,
    pub add_points_interval_ms: u64,
}

impl Config {
    pub async fn new<R: AsyncReadExt + Unpin>(mut reader: R) -> Self {
        let mut string = String::new();
        reader
            .read_to_string(&mut string)
            .await
            .expect("Failed to read config file");
        serde_json::from_str(&string).expect("Invalid config file")
    }

    pub async fn from_file(path: &str) -> Self {
        let file = File::open(path).await.expect("Could not open config file");
        Self::new(file).await
    }
}
