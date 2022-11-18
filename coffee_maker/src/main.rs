mod coffee_maker;
mod order_processor;
mod log;
use coffee_maker::start_coffee_maker;
use crate::log::init;

fn main() {
    let _guard = init();
    start_coffee_maker("./orders/one_order.csv", "127.0.0.1:34255");
}
