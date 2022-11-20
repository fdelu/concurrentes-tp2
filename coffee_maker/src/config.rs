use std::{fs::File, io::Read, net::SocketAddr};

use common::log::LogConfig;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub order_from: String,
    pub server_ip: SocketAddr,
    pub logs: LogConfig,
}

impl Config {
    pub fn new<R: Read>(reader: R) -> Self {
        serde_json::from_reader(reader).expect("Invalid config file")
    }

    pub fn from_file(path: &str) -> Self {
        let file = File::open(path).expect("Could not open config file");
        Self::new(file)
    }
}
