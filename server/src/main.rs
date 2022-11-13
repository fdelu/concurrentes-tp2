use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::{DistMutex, MutexCreationTrait, ResourceId, ServerId};
use crate::network::Listen;
use crate::packet_dispatcher::messages::add_mutex::AddMutexMessage;
use crate::packet_dispatcher::{PacketDispatcher, SERVERS};
use std::net::SocketAddr;
use std::thread;
use tokio::try_join;

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
    println!("I am server {} (addr: {})", n, addr);

    let addr: SocketAddr = SERVERS[n].into();

    dispatcher
        .try_send(Listen {
            bind_to: addr,
        })
        .unwrap();

    let resource_id_1 = ResourceId::new(1);
    let resource_id_2 = ResourceId::new(2);

    let mutex_addr_1 = dispatcher
        .send(AddMutexMessage::new(resource_id_1))
        .await
        .unwrap();

    let mutex_addr_2 = dispatcher
        .send(AddMutexMessage::new(resource_id_2))
        .await
        .unwrap();

    thread::sleep(std::time::Duration::from_millis(5000));

    println!("Acquiring lock");
    let f1 = mutex_addr_1.send(AcquireMessage::new());
    let f2 = mutex_addr_2.send(AcquireMessage::new());
    try_join!(f1, f2).unwrap();
}
