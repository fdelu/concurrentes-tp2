use std::future::Future;
use std::time;

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use rand::Rng;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::order_processor::{
    AbortRedemption, AddPoints, CommitRedemption, OrderProcessorTrait, RedeemCoffee,
    TransactionResult,
};
use common::packet::TxId;

mod messages;
mod order;

pub use self::messages::*;
pub use self::order::Coffee;
pub use self::order::Order;

const MAILBOX_CAPACITY: usize = 2048;

///Actor para atender ordenes de cafe y prepararlas
pub struct CoffeeMaker<O: OrderProcessorTrait> {
    order_processor: Addr<O>,
    fail_probability: u8,
}

impl<O: OrderProcessorTrait> Actor for CoffeeMaker<O> {
    type Context = Context<Self>;
}

impl<O: OrderProcessorTrait> CoffeeMaker<O> {
    pub fn new(order_processor: Addr<O>, fail_probability: u8) -> Addr<Self> {
        Self::create(move |ctx| {
            ctx.set_mailbox_capacity(MAILBOX_CAPACITY);
            Self {
                order_processor,
                fail_probability,
            }
        })
    }

    /// Matchea el tipo de orden
    fn process_order(
        &self,
        order: Order,
        ctx: &Context<Self>,
    ) -> impl Future<Output = TransactionResult> {
        let order_processor = self.order_processor.clone();
        let me = ctx.address().recipient();
        async move {
            match order {
                Order::Sale(coffee) => {
                    order_processor
                        .send(RedeemCoffee { coffee, maker: me })
                        .await
                }
                Order::Recharge(user_id, amount) => {
                    order_processor.send(AddPoints { amount, user_id }).await
                }
            }
            .unwrap_or_else(TransactionResult::from)
        }
    }

    /// Prepara el cafe, el tiempo de preparacion es aleatorio y si falla tambien.
    /// La probabilidad de que falle se define en la variable fail_probability
    async fn make_coffee(processor: Addr<O>, coffee: Coffee, tx_id: TxId, fail_probability: u8) {
        let mut rng = rand::thread_rng();
        sleep(time::Duration::from_millis(rng.gen_range(0..100))).await;
        if rng.gen_range(0..100) < fail_probability {
            warn!("Failed preparing coffee: {}", coffee.name);
            processor.do_send(AbortRedemption {
                transaction_id: tx_id,
                coffee,
            });
            return;
        }

        sleep(time::Duration::from_millis(rng.gen_range(0..100))).await;
        info!("Finished making coffee: {}", coffee.name);
        processor.do_send(CommitRedemption {
            transaction_id: tx_id,
            coffee,
        });
    }
}

impl<O: OrderProcessorTrait> Handler<AddOrder> for CoffeeMaker<O> {
    type Result = ResponseActFuture<Self, TransactionResult>;

    /// Hadlea al recibir un mensaje de tipo AddOrder una nueva order.
    fn handle(&mut self, msg: AddOrder, ctx: &mut Self::Context) -> Self::Result {
        self.process_order(msg.order, ctx)
            .into_actor(self)
            .boxed_local()
    }
}

impl<O: OrderProcessorTrait, R: AsyncReadExt + Unpin + 'static> Handler<ReadOrdersFrom<R>>
    for CoffeeMaker<O>
{
    type Result = ResponseActFuture<Self, Vec<(Order, TransactionResult)>>;

    /// Handlea el mensaje de tipo ReadOrdersFrom<R> que le indica como recibir las ordenes
    /// de cafes.
    fn handle(&mut self, msg: ReadOrdersFrom<R>, ctx: &mut Context<Self>) -> Self::Result {
        let reader = BufReader::new(msg.reader);
        let mut lines = reader.lines();
        let this = ctx.address();
        async move {
            let mut futures = vec![];
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => match line.parse::<Order>() {
                        Ok(order) => {
                            let r = this
                                .send(AddOrder {
                                    order: order.clone(),
                                })
                                .await;
                            futures.push((order, r.unwrap_or_else(TransactionResult::from)));
                        }
                        Err(e) => error!("Failed to parse order: {}", e),
                    },
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        error!("I/O error: {}", e);
                        break;
                    }
                }
            }
            futures
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl<O: OrderProcessorTrait> Handler<MakeCoffee> for CoffeeMaker<O> {
    type Result = ResponseActFuture<Self, ()>;

    /// Recibe un mensaje para preparar cafe y lo prepara
    fn handle(&mut self, msg: MakeCoffee, _ctx: &mut Context<Self>) -> Self::Result {
        let addr = self.order_processor.clone();
        let fail_probability = self.fail_probability;
        async move { CoffeeMaker::make_coffee(addr, msg.coffee, msg.tx_id, fail_probability).await }
            .into_actor(self)
            .boxed_local()
    }
}

#[cfg(test)]
mod test {
    use super::CoffeeMaker;
    use super::ReadOrdersFrom;
    use crate::order_processor::OrderProcessor;
    use actix::Actor;
    use actix::Addr;
    use actix::Context;
    use actix::Handler;
    use actix_rt::net::TcpListener;
    use common::packet::{ClientPacket, ServerPacket};
    use common::socket::ReceivedPacket;
    use common::socket::Socket;
    use common::socket::SocketEnd;
    use common::socket::SocketSend;
    use common::socket::Stream;
    use std::net::SocketAddr;
    use tokio::fs::File;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::sync::mpsc::UnboundedReceiver;
    use tokio::sync::mpsc::UnboundedSender;

    /// Estructura para corroborar que los mensajes recibidos
    /// fueron los correctos.
    struct RequiredMessages {
        prepare_expected: u32,
        commit_expected: u32,
        add_points_expected: u32,
        prepare_real: u32,
        commit_real: u32,
        add_points_real: u32,
    }

    impl RequiredMessages {
        /// Inicializacion, recibe los valores que esperar.
        fn new(prepare: u32, commit: u32, add_points: u32) -> RequiredMessages {
            RequiredMessages {
                prepare_expected: prepare,
                commit_expected: commit,
                add_points_expected: add_points,
                prepare_real: 0,
                commit_real: 0,
                add_points_real: 0,
            }
        }

        fn add_prepare(&mut self) {
            self.prepare_real += 1;
        }

        fn add_commit(&mut self) {
            self.commit_real += 1;
        }

        fn add_add_points(&mut self) {
            self.add_points_real += 1;
        }

        /// Corrobora si los valores esperados son iguales a los reales.
        fn check(&self) {
            assert_eq!(self.prepare_expected, self.prepare_real);
            assert_eq!(self.commit_expected, self.commit_real);
            assert_eq!(self.add_points_expected, self.add_points_real);
        }
    }

    /// Procesa los mensajes de la cafetera, corrobora que sea lo esperado
    /// y le responde.
    async fn order_ready_server(
        rx: &mut UnboundedReceiver<ClientPacket>,
        socket: &Addr<Socket<ServerPacket, ClientPacket>>,
        id: u32,
        cost: u32,
        orders: &mut Vec<u32>,
        required_messages: &mut RequiredMessages,
    ) {
        println!("reading");
        match rx.recv().await.unwrap() {
            ClientPacket::RedeemCoffee(user_id, amount, tx_id) => {
                println!("recieved prepare");
                if id != 0 {
                    assert_eq!(user_id, id);
                    assert_eq!(amount, cost);
                }
                let packet = ServerPacket::Ready(tx_id);
                socket.do_send(SocketSend { data: packet });
                orders.push(tx_id);
                required_messages.add_prepare();
            }
            ClientPacket::CommitRedemption(tx_id) => {
                let index = orders
                    .iter()
                    .position(|x| *x == tx_id)
                    .expect(&format!("commit for non existent prepare id: {}", tx_id)[..]);
                orders.remove(index);

                println!("recieved commit");
                required_messages.add_commit();
            }
            ClientPacket::AddPoints(user_id, amount, tx_id) => {
                println!("recieved recharge");
                if id != 0 {
                    assert_eq!(user_id, id);
                    assert_eq!(amount, cost);
                }
                assert!(!orders.contains(&tx_id));
                required_messages.add_add_points();
            }
        };
        println!("done")
    }

    /// Mock del servidor
    struct ServerMock {
        sender: UnboundedSender<ClientPacket>,
    }
    impl Actor for ServerMock {
        type Context = Context<Self>;
    }
    impl Handler<ReceivedPacket<ClientPacket>> for ServerMock {
        type Result = ();

        fn handle(&mut self, packet: ReceivedPacket<ClientPacket>, _ctx: &mut Context<Self>) {
            self.sender.send(packet.data).unwrap();
        }
    }
    impl Handler<SocketEnd> for ServerMock {
        type Result = ();

        fn handle(&mut self, _packet: SocketEnd, _ctx: &mut Context<Self>) {}
    }

    #[actix_rt::test]
    async fn test_one_order() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(1, 1, 0);
            for _ in 0..2 {
                order_ready_server(&mut rx, &socket, 3, 5, &mut orders, &mut required_messages)
                    .await;
            }
            if !orders.is_empty() {
                panic!("a transaction did not commit");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/one_order.csv").await.unwrap();
        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 0);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom { reader: file })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }

    #[actix_rt::test]
    async fn test_repeated_order() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(10, 10, 0);
            for i in 0..20 {
                println!("iteration: {}", i);
                order_ready_server(&mut rx, &socket, 3, 5, &mut orders, &mut required_messages)
                    .await;
            }
            if !orders.is_empty() {
                panic!("a transaction did not commit");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/repeated_order.csv").await.unwrap();

        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 0);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom {
                reader: Box::new(file),
            })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }

    #[actix_rt::test]
    async fn test_one_recharge() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(0, 0, 1);
            for _ in 0..1 {
                order_ready_server(
                    &mut rx,
                    &socket,
                    7,
                    100,
                    &mut orders,
                    &mut required_messages,
                )
                .await;
            }
            if !orders.is_empty() {
                panic!("a transaction did not commit");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/one_recharge.csv").await.unwrap();
        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 0);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom { reader: file })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }

    #[actix_rt::test]
    async fn test_diff_orders() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(3, 3, 2);
            for _ in 0..8 {
                order_ready_server(&mut rx, &socket, 0, 7, &mut orders, &mut required_messages)
                    .await;
            }
            if !orders.is_empty() {
                panic!("a transaction did not commit");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/diff_orders.csv").await.unwrap();
        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 0);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom { reader: file })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }

    #[actix_rt::test]
    async fn test_one_order_failing() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(1, 0, 0);
            for _ in 0..1 {
                order_ready_server(&mut rx, &socket, 3, 5, &mut orders, &mut required_messages)
                    .await;
            }
            if orders.len() != 1 {
                panic!("a transaction committed");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/one_order.csv").await.unwrap();
        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 0);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom { reader: file })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }

    #[actix_rt::test]
    async fn test_repeated_order_failing() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = unbounded_channel();
        let server_mock = ServerMock { sender: tx }.start();

        let join_handle = actix_rt::spawn(async move {
            let socket = Socket::new(
                server_mock.clone().recipient(),
                server_mock.recipient(),
                server_addr,
                Stream::Existing(listener.accept().await.unwrap().0),
            )
            .start();

            let mut orders: Vec<u32> = Vec::new();
            let mut required_messages = RequiredMessages::new(10, 0, 0);
            for i in 0..10 {
                println!("iteration: {}", i);
                order_ready_server(&mut rx, &socket, 3, 5, &mut orders, &mut required_messages)
                    .await;
            }
            if orders.len() != 10 {
                panic!("a transaction committed");
            }
            required_messages.check();
        });

        let file = File::open("./src/orders/repeated_order.csv").await.unwrap();

        let order_actor = OrderProcessor::new(server_addr);
        let maker_actor = CoffeeMaker::new(order_actor, 100);

        println!("sending message");
        maker_actor
            .send(ReadOrdersFrom {
                reader: Box::new(file),
            })
            .await
            .expect("Failed to send file to CoffeMaker");

        join_handle.await.unwrap();
    }
}
