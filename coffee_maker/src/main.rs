mod coffee_maker;
mod log;
mod order_processor;
use std::{fs::File, io::Read, net::SocketAddr};

use tracing::info;

use crate::{
    coffee_maker::{CoffeeMaker, ReadOrdersFrom},
    log::init,
    order_processor::OrderProcessor,
};

#[actix_rt::main]
pub async fn start_coffee_maker(path: &str) {
    let _log_guard = init();
    let file = File::open(path).unwrap();
    // let _guard = init();
    info!("started coffee making");
    let server_addr = SocketAddr::from(([127, 0, 0, 1], 34255));
    let order_actor = OrderProcessor::new(server_addr);
    let maker_actor = CoffeeMaker::new(order_actor);

    maker_actor
        .send(ReadOrdersFrom {
            reader: Box::new(file),
        })
        .await
        .expect("Failed to send file to CoffeMaker");

    info!("all orders taken");
}

fn main() {
    start_coffee_maker("./orders/one_order.csv");

    info!("Presione [ENTER] para detener la ejecución");

    let mut buf = [0u8; 1];
    std::io::stdin().read_exact(&mut buf).unwrap_or(());
}
