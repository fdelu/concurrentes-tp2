use crate::network::Listen;
use crate::packet_dispatcher::messages::{BlockPointsMessage, DiscountMessage};
use crate::packet_dispatcher::TransactionId;
use crate::{Config, PacketDispatcher, ServerId};
use actix::Addr;
use common::log::LogConfig;
use common::packet::{CoffeeMakerId, UserId};
use rand::Rng;
use serial_test::serial;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
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
        add_points_interval_ms: 5000,
        database_dump_path: Some("databases".to_string()),
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

fn parse_db(path: PathBuf) -> HashMap<UserId, u32> {
    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap()
}

// Lee todos los archivos de la carpeta `databases` y realiza un assert de que
// todos los archivos tienen el mismo contenido.
fn assert_databases_are_equal() {
    let mut files = std::fs::read_dir("databases").unwrap();
    let first_file = files.next().unwrap().unwrap();
    let first_file_content = parse_db(first_file.path());

    for file in files {
        let file = file.unwrap();
        let file_content = parse_db(file.path());
        assert_eq!(first_file_content, file_content);
    }
}

// Elimina todos los archivos que comiencen con `database_server` en la carpeta
// `databases`.
// Si la carpeta no existe, no hace nada.
fn clean_database_directory() {
    if let Ok(files) = std::fs::read_dir("databases") {
        for file in files {
            let file = file.unwrap();
            if file
                .file_name()
                .to_str()
                .unwrap()
                .starts_with("database_server")
            {
                std::fs::remove_file(file.path()).unwrap();
            }
        }
    }
}

#[actix_rt::test]
#[serial]
async fn test_two_dispatchers_one_discount() {
    clean_database_directory();

    let cfg_1 = make_config(1, 2);
    let cfg_2 = make_config(2, 2);

    info!("Cfg1: {:#?}", cfg_1);
    info!("Cfg2: {:#?}", cfg_2);

    let dispatcher_1 = make_packet_dispatcher(&cfg_1);
    dispatcher_1.do_send(Listen {});
    let dispatcher_2 = make_packet_dispatcher(&cfg_2);
    dispatcher_2.do_send(Listen {});
    let user_id = 456;
    sleep(Duration::from_millis(100)).await;
    discount(
        make_server_id(1),
        make_coffee_maker_id(123),
        dispatcher_1,
        user_id,
        10,
    )
    .await;
    sleep(Duration::from_millis(100)).await;
    assert_databases_are_equal();
}

#[actix_rt::test]
#[serial]
async fn test_two_dispatchers_two_discounts() {
    clean_database_directory();

    let cfg_1 = make_config(1, 2);
    let cfg_2 = make_config(2, 2);

    info!("Cfg1: {:#?}", cfg_1);
    info!("Cfg2: {:#?}", cfg_2);

    let dispatcher_1 = make_packet_dispatcher(&cfg_1);
    dispatcher_1.do_send(Listen {});
    let dispatcher_2 = make_packet_dispatcher(&cfg_2);
    dispatcher_2.do_send(Listen {});
    let user_id = 456;
    sleep(Duration::from_millis(100)).await;
    discount(
        make_server_id(1),
        make_coffee_maker_id(123),
        dispatcher_1,
        user_id,
        10,
    )
    .await;
    sleep(Duration::from_millis(100)).await;
    discount(
        make_server_id(2),
        make_coffee_maker_id(123),
        dispatcher_2,
        user_id,
        10,
    )
    .await;
    sleep(Duration::from_millis(100)).await;
    assert_databases_are_equal();
}

#[actix_rt::test]
#[serial]
async fn test_three_dispatchers_ten_discounts() {
    clean_database_directory();

    let cfg_1 = make_config(1, 3);
    let cfg_2 = make_config(2, 3);
    let cfg_3 = make_config(3, 3);

    info!("Cfg1: {:#?}", cfg_1);
    info!("Cfg2: {:#?}", cfg_2);
    info!("Cfg3: {:#?}", cfg_3);

    let dispatcher_1 = make_packet_dispatcher(&cfg_1);
    dispatcher_1.do_send(Listen {});
    let dispatcher_2 = make_packet_dispatcher(&cfg_2);
    dispatcher_2.do_send(Listen {});
    let dispatcher_3 = make_packet_dispatcher(&cfg_3);
    dispatcher_3.do_send(Listen {});
    let user_id = 456;
    sleep(Duration::from_millis(100)).await;
    for _ in 0..10 {
        discount(
            make_server_id(1),
            make_coffee_maker_id(123),
            dispatcher_1.clone(),
            user_id,
            3,
        )
        .await;
        discount(
            make_server_id(2),
            make_coffee_maker_id(123),
            dispatcher_2.clone(),
            user_id,
            3,
        )
        .await;
        discount(
            make_server_id(3),
            make_coffee_maker_id(123),
            dispatcher_3.clone(),
            user_id,
            3,
        )
        .await;
    }
    sleep(Duration::from_secs(10)).await;
    assert_databases_are_equal();
}

#[actix_rt::test]
#[serial]
async fn two_dispatchers_two_discounts_different_accounts() {
    clean_database_directory();

    let cfg_1 = make_config(1, 2);
    let cfg_2 = make_config(2, 2);

    info!("Cfg1: {:#?}", cfg_1);
    info!("Cfg2: {:#?}", cfg_2);

    let dispatcher_1 = make_packet_dispatcher(&cfg_1);
    dispatcher_1.do_send(Listen {});
    let dispatcher_2 = make_packet_dispatcher(&cfg_2);
    dispatcher_2.do_send(Listen {});
    let user_id_1 = 456;
    let user_id_2 = 789;
    sleep(Duration::from_millis(100)).await;
    discount(
        make_server_id(1),
        make_coffee_maker_id(123),
        dispatcher_1.clone(),
        user_id_1,
        10,
    )
    .await;
    sleep(Duration::from_millis(100)).await;
    discount(
        make_server_id(2),
        make_coffee_maker_id(123),
        dispatcher_2.clone(),
        user_id_2,
        10,
    )
    .await;
    sleep(Duration::from_millis(100)).await;
    assert_databases_are_equal();
}
