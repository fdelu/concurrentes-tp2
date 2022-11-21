use actix::Supervisor;
use common::error::{CoffeeError, FlattenResult};
use common::log::init_logger;
use tokio::io::{AsyncReadExt, stdin};
use tracing::info;

use crate::client_connections::ClientConnections;
use crate::config::Config;
use server_id::ServerId;
use crate::network::Listen;
use crate::packet_dispatcher::PacketDispatcher;

/// Módulo de conexiones con clientes.
pub mod client_connections;
/// Módulo de configuración.
mod config;
/// Módulo de mutex distribuido.
pub mod dist_mutex;
/// Módulo de conexiones (otros servidores y clientes).
mod network;
/// Módulo de envío de paquetes.
pub mod packet_dispatcher;
/// Módulo de commits en 2 fases.
pub mod two_phase_commit;
/// Modulo de identificación de servidores.
pub mod server_id;

#[actix_rt::main]
async fn main() {
    let config_path = std::env::args().nth(1).expect("No config file provided");
    let cfg = Config::from_file(&config_path);
    let _g = init_logger(&cfg.logs);

    let cfg_c = cfg.clone();
    let dispatcher = Supervisor::start(move |ctx| PacketDispatcher::new_with_context(&cfg_c, ctx));
    let clients = ClientConnections::new(&cfg, dispatcher);

    (clients.send(Listen {}).await.flatten() as Result<(), CoffeeError>)
        .expect("Failed to initialize server listener");

    info!("Press [ENTER] to stop execution");
    let mut buf = [0u8; 1];
    stdin().read_exact(&mut buf).await.ok();
}
