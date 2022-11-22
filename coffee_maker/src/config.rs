use std::net::SocketAddr;

use common::log::LogConfig;
use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt};

/// Tipo de dato para la configuracion de la cafetera.
#[derive(Deserialize)]
pub struct Config {
    pub order_from: String,
    pub server_ip: SocketAddr,
    pub logs: LogConfig,
    pub fail_probability: u8,
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
