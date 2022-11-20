use std::io::Read;

use crate::client_connections::ClientConnections;
use crate::config::Config;
use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::server_id::ServerId;
use crate::packet_dispatcher::PacketDispatcher;

pub mod client_connections;
mod config;
pub mod dist_mutex;
mod network;
pub mod packet_dispatcher;
pub mod two_phase_commit;

#[actix_rt::main]
async fn main() {
    let config_path = std::env::args().nth(1).expect("No config file provided");
    let cfg = Config::from_file(&config_path);

    let dispatcher = PacketDispatcher::new(&cfg);
    let _clients = ClientConnections::new(&cfg, dispatcher);

    println!("Presione [ENTER] para detener la ejecución");
    let mut buf = [0u8; 1];
    std::io::stdin().read_exact(&mut buf).unwrap_or(());
}
