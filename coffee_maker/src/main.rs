mod coffee_maker;
mod config;
mod order_processor;

use tokio::{
    fs::File,
    io::{stdin, AsyncReadExt},
};
use tracing::info;

use crate::{
    coffee_maker::{CoffeeMaker, ReadOrdersFrom},
    config::Config,
    order_processor::OrderProcessor,
};
use common::log::init_logger;

pub async fn start_coffee_maker(cfg: &Config) {
    info!("Initializing...");
    let order_actor = OrderProcessor::new(cfg.server_ip);
    let maker_actor = CoffeeMaker::new(order_actor, 15);

    info!("Reading orders from {}", cfg.order_from);

    if cfg.order_from == "stdin" {
        maker_actor.send(ReadOrdersFrom { reader: stdin() }).await
    } else {
        let file = File::open(cfg.order_from.clone())
            .await
            .expect("Failed to open orders file");
        maker_actor.send(ReadOrdersFrom { reader: file }).await
    }
    .expect("Failed to send file to CoffeMaker");
}

#[actix_rt::main]
async fn main() {
    let config_path = std::env::args().nth(1).expect("No config file provided");
    let cfg = Config::from_file(&config_path);
    let _guard = init_logger(&cfg.logs);

    start_coffee_maker(&cfg).await;

    info!("Finished adding orders. Press [ENTER] to stop execution");
    let mut buf = [0u8; 1];
    stdin().read_exact(&mut buf).await.ok();
}
