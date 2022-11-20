mod coffee_maker;
mod config;
mod log;
mod order_processor;
use std::{fs::File, io::Read};

use tracing::info;

use crate::{
    coffee_maker::{CoffeeMaker, ReadOrdersFrom},
    config::Config,
    log::init,
    order_processor::OrderProcessor,
};

pub async fn start_coffee_maker(cfg: &Config) {
    let file = File::open(&cfg.order_from).unwrap();
    // let _guard = init();
    info!("started coffee making");
    let server_addr = cfg.server_ip.parse().expect("Invalid server ip");
    let order_actor = OrderProcessor::new(server_addr);
    let maker_actor = CoffeeMaker::new(order_actor, 15);

    maker_actor
        .send(ReadOrdersFrom {
            reader: Box::new(file),
        })
        .await
        .expect("Failed to send file to CoffeMaker");
}

#[actix_rt::main]
async fn main() {
    let config_path = std::env::args().nth(1).expect("No config file provided");
    let cfg = Config::from_file(&config_path);
    let _log_guard = init(&cfg);

    start_coffee_maker(&cfg).await;

    info!("Presione [ENTER] para detener la ejecución");
    let mut buf = [0u8; 1];
    std::io::stdin().read_exact(&mut buf).unwrap_or(());
}
