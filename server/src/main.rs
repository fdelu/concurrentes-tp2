use std::net::SocketAddr;

use tokio::time::sleep;
use tokio::try_join;

use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::server_id::ServerId;
use crate::network::Listen;
use crate::packet_dispatcher::messages::public::block_points::BlockPointsMessage;
use crate::packet_dispatcher::{PacketDispatcher, SERVERS};

pub mod dist_mutex;
mod network;
pub mod packet_dispatcher;
pub mod two_phase_commit;

#[actix_rt::main]
async fn main() {
    let n: usize = std::env::var("N")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap();
    let dispatcher = PacketDispatcher::new(SERVERS[n]);

    let addr: SocketAddr = SERVERS[n].into();
    println!(
        "I am server {} (addr: {}, {})",
        n,
        addr,
        ServerId::from(addr)
    );

    dispatcher.try_send(Listen {}).unwrap();

    sleep(std::time::Duration::from_millis(5000)).await;

    dispatcher
        .send(BlockPointsMessage {
            client_id: 1,
            amount: 10,
        })
        .await
        .unwrap();

    dispatcher
        .send(BlockPointsMessage {
            client_id: 3,
            amount: 150,
        })
        .await
        .unwrap();

    dispatcher
        .send(BlockPointsMessage {
            client_id: 1,
            amount: 20,
        })
        .await
        .unwrap();

    dispatcher
        .send(BlockPointsMessage {
            client_id: 1,
            amount: 90,
        })
        .await
        .unwrap();

    dispatcher
        .send(BlockPointsMessage {
            client_id: 4,
            amount: 100,
        })
        .await
        .unwrap();

    println!("[main] Done transaction");

    sleep(std::time::Duration::from_secs(600)).await;
}
