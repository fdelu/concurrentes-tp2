use std::net::SocketAddr;
use std::str::FromStr;
use actix::Addr;
use tracing::Level;
use common::log::LogConfig;
use common::packet::{CoffeeMakerId, UserId};
use crate::{Config, PacketDispatcher, ServerId};
use crate::packet_dispatcher::messages::{BlockPointsMessage, DiscountMessage};
use rand::Rng;
use crate::packet_dispatcher::TransactionId;

fn random() -> u32 {
    rand::thread_rng().gen_range(0..1000000)
}

fn make_server_id(id: u32) -> ServerId {
    if id == 0 {
        panic!("Server id cannot be 0");
    }
    println!("{}", format!("127.0.0.{}", id));
    ServerId::new(SocketAddr::from_str(&format!("127.0.0.{}", id)).unwrap().ip())
}

fn make_coffee_maker_id(id: u16) -> CoffeeMakerId {
    if id == 0 {
        panic!("Coffee maker id cannot be 0");
    }
    CoffeeMakerId::new(SocketAddr::from_str(&format!("127.0.0.{}", id)).unwrap().ip(), 1234)
}

fn make_packet_dispatcher(server_id: ServerId, server_amount: u32) -> Addr<PacketDispatcher> {
    let servers = (0..server_amount)
        .map(|i| {
            make_server_id(i + 1)
        })
        .collect();

    let cfg = Config {
        server_ip: server_id.get_socket_addr(5555).ip(),
        servers,
        server_port: 5555,
        client_port: 8080,
        logs: LogConfig {
            stdout_log_level: Level::TRACE,
            file_log_level: Level::DEBUG,
            files_directory: "logs_test".to_string(),
        },
    };

    PacketDispatcher::new(&cfg)
}

async fn discount(server_id: ServerId, coffee_maker_id: CoffeeMakerId, dispatcher: Addr<PacketDispatcher>, user_id: UserId, amount: u32) {
    let transaction_id = TransactionId::Discount(server_id, coffee_maker_id, random());
    dispatcher
        .send(BlockPointsMessage {
            user_id,
            amount,
            transaction_id,
        })
        .await
        .unwrap()
        .unwrap();

    dispatcher
        .send(DiscountMessage {
            user_id,
            transaction_id,
        })
        .await
        .unwrap()
        .unwrap();
}

#[actix_rt::test]
async fn test_two_dispatchers_one_discount() {
    let server_id_1 = make_server_id(1);
    let server_id_2 = make_server_id(2);

    let dispatcher_1 = make_packet_dispatcher(server_id_1, 2);
    let dispatcher_2 = make_packet_dispatcher(server_id_2, 2);

    let user_id = random();
    discount(server_id_1, make_coffee_maker_id(123), dispatcher_1, user_id, 10).await;
}

