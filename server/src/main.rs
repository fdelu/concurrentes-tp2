use std::net::SocketAddr;
use tokio::time::sleep;
use tokio::try_join;

use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::packets::ResourceId;
use crate::dist_mutex::server_id::ServerId;
use crate::dist_mutex::{DistMutex, MutexCreationTrait};
use crate::network::Listen;
use crate::packet_dispatcher::messages::add_mutex::AddMutexMessage;
use crate::packet_dispatcher::{PacketDispatcher, SERVERS};

pub mod dist_mutex;
mod network;
pub mod packet_dispatcher;

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

    let resource_id_1 = 1;
    let resource_id_2 = 2;

    let mutex_addr_1 = dispatcher
        .send(AddMutexMessage::new(resource_id_1))
        .await
        .unwrap();

    let mutex_addr_2 = dispatcher
        .send(AddMutexMessage::new(resource_id_2))
        .await
        .unwrap();

    sleep(std::time::Duration::from_millis(5000)).await;

    println!("Acquiring lock");
    let f1 = mutex_addr_1.send(AcquireMessage::new());
    let f2 = mutex_addr_2.send(AcquireMessage::new());
    if try_join!(f1, f2).is_err() {
        println!("Try join failed");
    }

    sleep(std::time::Duration::from_millis(5000)).await;

    println!("Releasing lock");

    let f1 = mutex_addr_1.send(ReleaseMessage {});
    let f2 = mutex_addr_2.send(ReleaseMessage {});

    if try_join!(f1, f2).is_err() {
        println!("Try join failed");
    }

    println!("Done");

    sleep(std::time::Duration::from_millis(50000)).await;
}
