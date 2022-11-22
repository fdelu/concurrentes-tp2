use crate::network::Listen;
use crate::packet_dispatcher::messages::{BlockPointsMessage, DiscountMessage};
use crate::packet_dispatcher::TransactionId;
use crate::{Config, PacketDispatcher, ServerId};
use actix::Addr;
use common::log::LogConfig;
use common::packet::{CoffeeMakerId, UserId};
use rand::Rng;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

fn random() -> u32 {
    rand::thread_rng().gen_range(0..1000000)
}

fn make_server_id(id: u32) -> ServerId {
    if id == 0 {
        panic!("Server id cannot be 0");
    }
    ServerId::new(IpAddr::from_str(&format!("127.0.0.{}", id)).unwrap())
}

fn make_coffee_maker_id(id: u16) -> CoffeeMakerId {
    if id == 0 {
        panic!("Coffee maker id cannot be 0");
    }
    CoffeeMakerId::new(IpAddr::from_str(&format!("127.0.0.{}", id)).unwrap(), 1234)
}

fn make_config(server_number: u32, server_amount: u32) -> Config {
    let servers = (0..server_amount).map(|i| make_server_id(i + 1)).collect();

    Config {
        server_ip: make_server_id(server_number).ip,
        servers,
        server_port: 5555,
        client_port: Rng::gen_range(&mut rand::thread_rng(), 10000..20000),
        logs: LogConfig {
            stdout_log_level: Level::TRACE,
            file_log_level: Level::DEBUG,
            files_directory: "logs_test".to_string(),
        },
    }
}

fn make_packet_dispatcher(cfg: &Config) -> Addr<PacketDispatcher> {
    PacketDispatcher::new(cfg)
}

async fn discount(
    server_id: ServerId,
    coffee_maker_id: CoffeeMakerId,
    dispatcher: Addr<PacketDispatcher>,
    user_id: UserId,
    amount: u32,
) {
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
    let cfg_1 = make_config(1, 2);
    let cfg_2 = make_config(2, 2);
    //let _g = init_logger(&cfg_1.logs);

    info!("Cfg1: {:#?}", cfg_1);
    info!("Cfg2: {:#?}", cfg_2);

    let dispatcher_1 = make_packet_dispatcher(&cfg_1);
    dispatcher_1.do_send(Listen {});
    sleep(Duration::from_secs(10)).await;
    let dispatcher_2 = make_packet_dispatcher(&cfg_2);
    dispatcher_2.do_send(Listen {});
    sleep(Duration::from_secs(10)).await;
    let user_id = 456;
    discount(
        make_server_id(1),
        make_coffee_maker_id(123),
        dispatcher_1,
        user_id,
        10,
    )
    .await;
    sleep(Duration::from_secs(100)).await;
    assert_eq!(true, false);
}
