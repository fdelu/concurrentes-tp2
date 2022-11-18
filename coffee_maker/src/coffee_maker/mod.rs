use std::io::{BufRead, BufReader};
use std::time;

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use rand::Rng;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::order_processor::{
    AbortOrder, AddMoney, CommitOrder, OrderProcessorTrait, PrepareOrder,
};
use common::packet::TxId;

mod messages;
mod order;

pub use self::messages::*;
pub use self::order::Coffee;
use self::order::Order;

pub struct CoffeeMaker<O: OrderProcessorTrait> {
    order_processor: Addr<O>,
}

impl<O: OrderProcessorTrait> Actor for CoffeeMaker<O> {
    type Context = Context<Self>;
}

impl<O: OrderProcessorTrait> CoffeeMaker<O> {
    pub fn new(order_processor: Addr<O>) -> Addr<Self> {
        Self { order_processor }.start()
    }

    ///checks the type of order
    fn process_order(&self, order: String, ctx: &Context<Self>) {
        match order.parse::<Order>() {
            Ok(Order::Sale(coffee)) => {
                // TODO
                self.order_processor.do_send(PrepareOrder {
                    coffee,
                    maker: ctx.address().recipient(),
                });
            }
            Ok(Order::Recharge(amount, user_id)) => {
                self.order_processor.do_send(AddMoney { amount, user_id });
            }
            Err(e) => {
                error!("Error parsing order: {}", e);
            }
        }
    }

    /// Prepares coffee, the time it takes is random and theres a chance to fail.
    async fn make_coffee(processor: Addr<O>, coffee: Coffee, tx_id: TxId) {
        let mut rng = rand::thread_rng();
        sleep(time::Duration::from_millis(rng.gen_range(0..100))).await;
        if rng.gen_range(0..100) < 15 {
            warn!("Failed preparing coffee: {}", coffee.name);
            processor.do_send(AbortOrder {
                transaction_id: tx_id,
                coffee,
            });
            return;
        }

        sleep(time::Duration::from_millis(rng.gen_range(0..100))).await;
        info!("Finished making coffee: {}", coffee.name);
        processor.do_send(CommitOrder {
            transaction_id: tx_id,
            coffee,
        });
    }
}

impl<O: OrderProcessorTrait> Handler<ReadOrdersFrom> for CoffeeMaker<O> {
    type Result = ();

    fn handle(&mut self, msg: ReadOrdersFrom, ctx: &mut Context<Self>) -> Self::Result {
        let reader = BufReader::new(msg.reader);

        for line_res in reader.lines() {
            match line_res {
                Ok(line) => self.process_order(line, ctx),
                Err(e) => error!("I/O error: {}", e),
            }
        }
    }
}

impl<O: OrderProcessorTrait> Handler<MakeCoffee> for CoffeeMaker<O> {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: MakeCoffee, _ctx: &mut Context<Self>) -> Self::Result {
        let addr = self.order_processor.clone();
        async move { CoffeeMaker::make_coffee(addr, msg.coffee, msg.tx_id).await }
            .into_actor(self)
            .boxed_local()
    }
}

#[cfg(test)]
mod test {
    use crate::start_coffee_maker;
    use std::io::prelude::*;
    use std::net::TcpListener;
    use std::net::TcpStream;
    use std::{thread, time};
    // use futures::join;

    fn assert_order_ready(stream: &mut TcpStream, id: u8, cost: u8) {
        let mut buf = [0_u8; 3];
        let _ = stream.read(&mut buf).unwrap();
        assert_eq!(buf, ['p' as u8, id, cost]);

        let response = [b'r'];
        let _ = stream.write(&response).unwrap();

        let mut buf2 = [0_u8; 1];
        let _ = stream.read(&mut buf2).unwrap();
        assert!(buf2 == [b'c'] || buf2 == [b'a']);
    }

    #[test]
    fn test_one_order() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34235").unwrap();
            let (mut stream, _) = listener.accept().unwrap();

            assert_order_ready(&mut stream, 3 as u8, 5 as u8);
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34235");

        join_handle.join().unwrap();
    }

    #[test]
    fn test_repeated_order() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34234").unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            for _ in 0..10 {
                assert_order_ready(&mut stream, 3 as u8, 5 as u8);
            }
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/repeated_order.csv", "127.0.0.1:34234");

        join_handle.join().unwrap();
    }

    #[test]
    fn test_one_recharge() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34233").unwrap();
            let (mut stream, _) = listener.accept().unwrap();

            let mut buf = [0_u8; 3];
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 7 as u8, 100 as u8]);

            let response = [b'o'];
            let _ = stream.write(&response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/one_recharge.csv", "127.0.0.1:34233");

        join_handle.join().unwrap();
    }

    #[test]
    fn test_diff_orders() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34232").unwrap();
            let (mut stream, _) = listener.accept().unwrap();

            assert_order_ready(&mut stream, 3 as u8, 5 as u8);

            let mut buf = [0_u8; 3];
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 7 as u8, 100 as u8]);

            let response = [b'o'];
            let _ = stream.write(&response).unwrap();

            assert_order_ready(&mut stream, 100 as u8, 3 as u8);

            assert_order_ready(&mut stream, 5 as u8, 12 as u8);

            let mut buf = [0_u8; 3];
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 12 as u8, 20 as u8]);

            let response = [b'o'];
            let _ = stream.write(&response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/diff_orders.csv", "127.0.0.1:34232");

        join_handle.join().unwrap();
    }

    async fn assert_order_ready_async(stream: &mut TcpStream, id: u8, cost: u8, debug: i32) {
        if debug == 1 {
            tokio::time::sleep(time::Duration::from_millis(100)).await;
        }
        let mut buf = [0_u8; 3];
        let _ = stream.read(&mut buf).unwrap();
        assert_eq!(buf, ['p' as u8, id, cost]);

        let response = [b'r'];
        let _ = stream.write(&response).unwrap();

        let mut buf2 = [0_u8; 1];
        let _ = stream.read(&mut buf2).unwrap();
        assert!(buf2 == [b'c'] || buf2 == [b'a']);
    }

    #[tokio::main]
    async fn split_processing(mut stream: &mut TcpStream, mut stream2: &mut TcpStream) {
        let fut1 = assert_order_ready_async(&mut stream, 3 as u8, 5 as u8, 1);
        let fut2 = assert_order_ready_async(&mut stream2, 3 as u8, 5 as u8, 2);
        tokio::join!(fut1, fut2);
    }

    #[test]
    fn test_two_coffee_makers() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34231").unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            let (mut stream2, _) = listener.accept().unwrap();
            split_processing(&mut stream, &mut stream2);
        });

        thread::sleep(time::Duration::from_millis(100));
        let join_handle2 = thread::spawn(|| {
            start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34231");
        });
        start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34231");

        join_handle.join().unwrap();
        join_handle2.join().unwrap();
    }

    #[test]
    fn test_wrong_message() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34230").unwrap();
            let (mut stream, _) = listener.accept().unwrap();

            let mut buf = [0_u8; 3];
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34230");

        join_handle.join().unwrap();
    }
}
