use std::{fs::File, io::Read, net::IpAddr};

use common::log::LogConfig;
use serde::Deserialize;

use crate::server_id::ServerId;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server_ip: IpAddr,
    pub servers: Vec<ServerId>,
    pub server_port: u16,
    pub client_port: u16,
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
