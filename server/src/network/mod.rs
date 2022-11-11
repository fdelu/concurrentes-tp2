#![cfg_attr(test, allow(dead_code))]
use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::task::JoinHandle;
use tokio::net::ToSocketAddrs;

use mockall_double::double;

mod connection;
mod error;
mod listener;
mod messages;
mod socket;

#[double]
use self::connection::Connection;
#[double]
use self::listener::Listener;
pub use self::messages::*;
use self::{
    error::SocketError,
    socket::{ReceivedPacket, SocketEnd, SocketSend},
};
use common::AHandler;

pub struct ConnectionHandler<A: AHandler<ReceivedPacket>> {
    connections: HashMap<SocketAddr, Connection<Self>>,
    received_handler: Addr<A>,
    join_listener: Option<JoinHandle<()>>,
}

impl<A: AHandler<ReceivedPacket>> ConnectionHandler<A> {
    fn new(received_handler: Addr<A>) -> Self {
        Self {
            connections: HashMap::new(),
            received_handler,
            join_listener: None,
        }
    }

    fn get_connection(&mut self, this: Addr<Self>, addr: SocketAddr) -> &mut Connection<Self> {
        let connection = self
            .connections
            .entry(addr)
            .or_insert_with(move || Connection::new(this.clone(), this, addr, None));
        connection
    }
}

impl<A: AHandler<ReceivedPacket>> Actor for ConnectionHandler<A> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(join) = self.join_listener.take() {
            join.abort();
        }
    }
}

// Public messages

impl<A: AHandler<ReceivedPacket>> Handler<SendPacket> for ConnectionHandler<A> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendPacket, ctx: &mut Context<Self>) -> Self::Result {
        let connection = self.get_connection(ctx.address(), msg.to);
        connection.restart_timeout();
        let socket = connection.get_socket();

        async move { socket.send(SocketSend { data: msg.data }).await? }
            .into_actor(self)
            .boxed_local()
    }
}

impl<A: AHandler<ReceivedPacket>, T: ToSocketAddrs + 'static> Handler<Listen<T>>
    for ConnectionHandler<A>
{
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen<T>, _ctx: &mut Context<Self>) -> Self::Result {
        if self.join_listener.is_some() {
            return async { Err(SocketError::new("Already listening for new connection")) }
                .into_actor(self)
                .boxed_local();
        }

        async move { Listener::bind(msg.bind_to).await }
            .into_actor(self)
            .map(|listener, this, ctx| {
                this.join_listener = Some(listener?.run(ctx.address()));
                Ok(())
            })
            .boxed_local()
    }
}

impl<A: AHandler<ReceivedPacket>> Handler<ReceivedPacket> for ConnectionHandler<A> {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(conn) = self.connections.get_mut(&msg.addr) {
            conn.restart_timeout();
        }

        self.received_handler.do_send(msg);
    }
}

// Private Messages

impl<A: AHandler<ReceivedPacket>> Handler<AddStream> for ConnectionHandler<A> {
    type Result = ();

    fn handle(&mut self, msg: AddStream, ctx: &mut Self::Context) -> Self::Result {
        let connection = Connection::new(
            ctx.address(),
            self.received_handler.clone(),
            msg.addr,
            Some(msg.stream),
        );
        self.connections.insert(msg.addr, connection);
    }
}

impl<A: AHandler<ReceivedPacket>> Handler<SocketEnd> for ConnectionHandler<A> {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, _ctx: &mut Self::Context) {
        self.connections.remove(&msg.addr);
    }
}

#[cfg(test)]
mod tests {
    use actix::{Actor, Context, Handler, System};
    use std::sync::{Arc, Mutex};

    use crate::network::SendPacket;

    use super::connection::test::init_connections;
    use super::socket::{tests::MockSocket as Socket, ReceivedPacket};
    use super::{Connection, ConnectionHandler};

    pub struct Receiver {
        pub received: Arc<Mutex<Vec<ReceivedPacket>>>,
    }
    impl Actor for Receiver {
        type Context = Context<Self>;
    }
    impl Handler<ReceivedPacket> for Receiver {
        type Result = ();

        fn handle(&mut self, msg: ReceivedPacket, _ctx: &mut Self::Context) {
            self.received.lock().unwrap().push(msg);
        }
    }

    type CH = ConnectionHandler<Receiver>;

    #[test]
    fn test_send_packet() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let sent = Arc::new(Mutex::new(Vec::new()));
        let socket = Socket { sent: sent.clone() };

        let data = vec![1, 2, 3, 4, 5];
        let data_c = data.clone();

        sys.block_on(async move {
            let s_addr = socket.start();
            let r_addr = receiver.start();
            let mut conn: Connection<CH> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_get_socket().times(1).return_const(s_addr);
            let _g = init_connections::<CH, CH>(vec![conn]);
            let handler = ConnectionHandler::new(r_addr).start();

            handler
                .send(SendPacket {
                    to: ([127, 0, 0, 1], 1234).into(),
                    data: data_c,
                })
                .await
                .unwrap()
                .unwrap();
        });

        assert_eq!(sent.lock().unwrap().len(), 1);
        assert_eq!(sent.lock().unwrap()[0].data, data);
        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_receive_packet() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![1, 2, 3, 4, 5];
        let data_c = data.clone();

        sys.block_on(async move {
            let r_addr = receiver.start();
            let handler = ConnectionHandler::new(r_addr).start();
            handler
                .send(ReceivedPacket {
                    addr: ([127, 0, 0, 1], 1234).into(),
                    data: data_c,
                })
                .await
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 1);
        assert_eq!(received.lock().unwrap()[0].data, data);
    }
}
