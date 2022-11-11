use crate::dist_mutex::messages::public::acquire::AcquireMessage;
use crate::dist_mutex::{DistMutex, MutexCreationTrait, ResourceId, ServerId};
use crate::network::Listen;
use crate::packet_dispatcher::messages::add_mutex::AddMutexMessage;
use crate::packet_dispatcher::PacketDispatcher;
use std::collections::HashSet;
use std::thread;
use tokio::{try_join};

pub mod dist_mutex;
mod network;
pub mod packet_dispatcher;

const ADDRESS: &str = "127.0.0.1";
const PORTS: [u16; 3] = [8000, 8001, 8002];

#[actix_rt::main]
async fn main() {
    let n: usize = std::env::var("N")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap();
    let mut servers = HashSet::new();
    for (i, port) in PORTS.iter().enumerate() {
        if i == n {
            continue;
        }
        let server_id = ServerId::new(*port);
        servers.insert(server_id);
    }
    let dispatcher = PacketDispatcher::new(servers);

    println!("Listening on port {}", PORTS[n]);
    dispatcher
        .try_send(Listen {
            bind_to: (ADDRESS, PORTS[n]),
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
