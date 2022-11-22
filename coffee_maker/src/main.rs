use coffee_maker::Order;
use order_processor::TransactionResult;
use tokio::{fs::File, io::stdin};
use tracing::info;

/// Módulo de la cafetera
mod coffee_maker;
/// Módulo de configuración
mod config;
/// Módulo de procesamiento de pedidos
mod order_processor;

use crate::{
    coffee_maker::{CoffeeMaker, ReadOrdersFrom},
    config::Config,
    order_processor::OrderProcessor,
};
use common::log::init_logger;

/// Inicializa la cafetera y el procesador de pedidos, y luego
/// lee los pedidos según la configuración dada.
pub async fn start_coffee_maker(cfg: &Config) -> Vec<(Order, TransactionResult)> {
    info!("Initializing...");
    let order_actor = OrderProcessor::new(cfg.server_ip);
    let maker_actor = CoffeeMaker::new(order_actor, cfg.fail_probability);

    info!("Reading orders from {}", cfg.order_from);

    return if cfg.order_from == "stdin" {
        maker_actor.send(ReadOrdersFrom { reader: stdin() }).await
    } else {
        let file = File::open(cfg.order_from.clone())
            .await
            .expect("Failed to open orders file");
        maker_actor.send(ReadOrdersFrom { reader: file }).await
    }
    .expect("Failed to send file to CoffeMaker");
}

fn print_results(results: Vec<(Order, TransactionResult)>) {
    info!("Finished processing orders. Results:");
    for (order, result) in results {
        info!("{}: {}", order, result);
    }
}

#[actix_rt::main]
async fn main() {
    let config_path = std::env::args().nth(1).expect("No config file provided");
    let cfg = Config::from_file(&config_path).await;
    let _guard = init_logger(&cfg.logs);

    print_results(start_coffee_maker(&cfg).await);
}
