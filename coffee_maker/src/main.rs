mod coffee_maker;
mod order_processor;
use coffee_maker::start_coffee_maker;

fn main() {
    start_coffee_maker("./orders/one_order.csv", "127.0.0.1:34255");
}
