use std::{collections::HashSet, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use common::{
    packet::{TxId, UserId},
    socket::{ReceivedPacket, SocketEnd, SocketError, SocketSend, Stream},
};
use tracing::{error, info, warn};

use super::processor_messages::{AbortOrder, AddMoney, CommitOrder, PrepareOrder};
use common::{
    packet::{ClientPacket, ServerPacket},
    socket::Socket,
};

///Actor para procesar ordenes de cafe
pub(crate) struct OrderProcessor {
    server_socket: Addr<Socket<ClientPacket, ServerPacket>>,
    user_id: UserId,
    active_transactions: HashSet<TxId>,
}

impl Actor for OrderProcessor {
    type Context = Context<Self>;
}

impl OrderProcessor {
    ///constructor de OrderProcessor, recive la direccion del servidor y los milisegundos de timeout
    pub(crate) fn new(server_addr: SocketAddr, read_timeout: u64) -> Addr<Self> {
        Self::create(move |ctx| Self {
            server_socket: Socket::new(ctx.address(), ctx.address(), server_addr, Stream::New())
                .start(),
            user_id: 123, // TODO
            active_transactions: HashSet::new(),
        })
    }

    fn add_tx_id(&mut self) -> TxId {
        let mut tx_id = rand::random();
        while self.active_transactions.contains(&tx_id) {
            tx_id = rand::random();
        }
        self.active_transactions.insert(tx_id);
        tx_id
    }

    fn remove_tx_id(&self, tx_id: TxId) -> bool {
        if !self.active_transactions.remove(&tx_id) {
            warn!(
                "Got message from the server for inactive transaction: {}",
                tx_id
            );
            return false;
        }
        true
    }

    fn handle_ready(&mut self, tx_id: TxId) {
        if self.remove_tx_id(tx_id) {
            // proceed
        }
    }

    fn handle_insufficient(&mut self, tx_id: TxId) {
        if self.remove_tx_id(tx_id) {
            info!("Transaction {} canceled: insufficient points", tx_id);
        }
    }

    fn handle_server_error(&mut self, tx_id: TxId, error: SocketError) {
        if self.remove_tx_id(tx_id) {
            error!("Transaction {} canceled: server error ({})", tx_id, error);
        }
    }
}

impl Handler<PrepareOrder> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: PrepareOrder, _ctx: &mut Self::Context) {
        let transaction_id = self.add_tx_id();
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::PrepareOrder(self.user_id, msg.cost, transaction_id),
                })
                .await;

            if let Err(e) = res.map_err(SocketError::from).and_then(|x| x) {
                error!("Error sending PrepareOrder: {}", e);
            } else {
                info!("Sent PrepareOrder with transaction id {}", transaction_id);
            }
        }
        .into_actor(self)
        .boxed_local();
    }
}

impl Handler<CommitOrder> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: CommitOrder, _ctx: &mut Self::Context) {
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::CommitOrder(msg.transaction_id),
                })
                .await;

            if let Err(e) = res.map_err(SocketError::from).and_then(|x| x) {
                error!("Error sending CommitOrder: {}", e);
            } else {
                info!(
                    "Sent CommitOrder with transaction id {}",
                    msg.transaction_id
                )
            }
        }
        .into_actor(self)
        .boxed_local();
    }
}

impl Handler<AbortOrder> for OrderProcessor {
    type Result = ();

    fn handle(&mut self, msg: AbortOrder, _ctx: &mut Self::Context) {
        warn!("Aborting order with transaction id {}", msg.transaction_id)
    }
}

impl Handler<AddMoney> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: AddMoney, _ctx: &mut Self::Context) {
        let transaction_id = self.add_tx_id();
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::AddPoints(self.user_id, msg.amount, transaction_id),
                })
                .await;

            if let Err(e) = res.map_err(SocketError::from).and_then(|x| x) {
                error!("Error sending AddMoney: {}", e);
            } else {
                info!("Sent AddMoney with transaction id {}", transaction_id)
            }
        }
        .into_actor(self)
        .boxed_local();
    }
}

impl Handler<SocketEnd> for OrderProcessor {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

impl Handler<ReceivedPacket<ServerPacket>> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        msg: ReceivedPacket<ServerPacket>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg.data {
            ServerPacket::Ready(tx_id) => self.handle_ready(tx_id),
            ServerPacket::Insufficient(tx_id) => self.handle_insufficient(tx_id),
            ServerPacket::ServerErrror(tx_id, e) => self.handle_server_error(tx_id, e),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::order_processor::processor::OrderProcessor;
    use std::io::prelude::*;
    use std::net::TcpListener;
    use std::{thread, time};
    extern crate actix;
    use crate::order_processor::processor_messages::{
        AbortOrder, AddMoney, CommitOrder, PrepareOrder,
    };

    #[actix_rt::test]
    async fn test_prepare_ready() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34244").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['r' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34244", 10000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("ready"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_timeout() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34254").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(1500));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['r' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(1000));

        let a = OrderProcessor::new("127.0.0.1:34254", 100).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("timeout"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_insufficient() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34243").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['i' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34243", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("insufficient"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_abort() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34242").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['a' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34242", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("abort"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_unknown() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34241").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['c' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34241", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("unknown"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_commit_sent() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34240").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['c' as u8]);
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34240", 1000).unwrap();
        let a_addr = a.start();

        let order = CommitOrder {};

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("sent"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_abort_sent() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34239").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['a' as u8]);
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34239", 1000).unwrap();
        let a_addr = a.start();

        let order = AbortOrder {};

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("sent"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_ok() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34238").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);

            let mut response = ['o' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34238", 10000).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("Ok"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_unknown() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34237").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);

            let mut response = ['t' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34237", 10000).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("unknown"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_timeout() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34236").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(1500));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);
        });

        thread::sleep(time::Duration::from_millis(1000));

        let a = OrderProcessor::new("127.0.0.1:34236", 100).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("timeout"));
        join_handle.join().unwrap();
    }
}
