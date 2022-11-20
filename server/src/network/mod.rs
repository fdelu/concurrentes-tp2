#![cfg_attr(test, allow(dead_code))]
use std::{collections::HashMap, net::SocketAddr, time::Duration};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Recipient, ResponseActFuture,
    WrapFuture,
};
use actix_rt::task::JoinHandle;

use mockall_double::double;
use tracing::{debug, info};

mod connection;
mod listener;
pub mod messages;

#[double]
use self::connection::Connection;
#[double]
use self::listener::Listener;
pub use self::messages::*;
#[cfg(test)]
use common::socket::test_util::MockStream as Stream;
pub use common::socket::ReceivedPacket;
#[cfg(not(test))]
use common::socket::Stream;
use common::socket::{PacketRecv, PacketSend, SocketEnd, SocketError};

pub trait Packet: PacketSend + PacketRecv {}
impl<T: PacketSend + PacketRecv> Packet for T {}

pub struct ConnectionHandler<S: PacketSend, R: PacketRecv> {
    connections: HashMap<SocketAddr, Connection<S, R>>,
    received_handler: Recipient<ReceivedPacket<R>>,
    join_listener: Option<JoinHandle<()>>,
    bind_to: SocketAddr,
    start_connections: bool,
    timeout: Option<Duration>,
}

impl<S: PacketSend, R: PacketRecv> ConnectionHandler<S, R> {
    pub fn new(
        received_handler: Recipient<ReceivedPacket<R>>,
        bind_to: SocketAddr,
        start_connections: bool,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            received_handler,
            join_listener: None,
            start_connections,
            bind_to,
            timeout,
        }
    }

    fn get_connection(
        &mut self,
        this: Addr<Self>,
        addr: SocketAddr,
    ) -> Option<&mut Connection<S, R>> {
        if !self.start_connections {
            return None;
        }

        let bind_to = addr.ip();
        let timeout = self.timeout;
        let connection = self.connections.entry(addr).or_insert_with(move || {
            Connection::new(
                this.clone().recipient(),
                this.recipient(),
                addr,
                Stream::NewBindedTo(bind_to),
                timeout,
            )
        });
        Some(connection)
    }
}

impl<S: PacketSend, R: PacketRecv> Actor for ConnectionHandler<S, R> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("ConnectionHandler stopped");
        if let Some(join) = self.join_listener.take() {
            join.abort();
        }
    }
}

// Public messages

impl<S: PacketSend, R: PacketRecv> Handler<SendPacket<S>> for ConnectionHandler<S, R> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendPacket<S>, ctx: &mut Context<Self>) -> Self::Result {
        match self.get_connection(ctx.address(), msg.to) {
            Some(connection) => {
                connection.restart_timeout();
                connection.send(msg.data).into_actor(self).boxed_local()
            }
            None => async { Err(SocketError::new("Connection not found")) }
                .into_actor(self)
                .boxed_local(),
        }
    }
}

impl<S: PacketSend, R: PacketRecv> Handler<Listen> for ConnectionHandler<S, R> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, _: Listen, _ctx: &mut Context<Self>) -> Self::Result {
        if self.join_listener.is_some() {
            return async { Err(SocketError::new("Already listening for new connection")) }
                .into_actor(self)
                .boxed_local();
        }
        let bind_to = self.bind_to;
        async move { Listener::bind(bind_to).await }
            .into_actor(self)
            .map(|listener, this, ctx| {
                this.join_listener = Some(listener?.run(ctx.address().recipient()));
                Ok(())
            })
            .boxed_local()
    }
}

impl<S: PacketSend, R: PacketRecv> Handler<ReceivedPacket<R>> for ConnectionHandler<S, R> {
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket<R>, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(conn) = self.connections.get_mut(&msg.addr) {
            conn.restart_timeout();
        }

        self.received_handler.do_send(msg);
    }
}

// Private Messages

impl<S: PacketSend, R: PacketRecv> Handler<AddStream> for ConnectionHandler<S, R> {
    type Result = ();

    fn handle(&mut self, msg: AddStream, ctx: &mut Self::Context) -> Self::Result {
        debug!("Creating connection from stream");
        let connection = Connection::new(
            ctx.address().recipient(),
            ctx.address().recipient(),
            msg.addr,
            Stream::Existing(msg.stream),
            self.timeout,
        );
        self.connections.insert(msg.addr, connection);
    }
}

impl<S: PacketSend, R: PacketRecv> Handler<SocketEnd> for ConnectionHandler<S, R> {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, _ctx: &mut Self::Context) {
        self.connections.remove(&msg.addr);
    }
}

#[cfg(test)]
mod tests {
    use actix::{Actor, Addr, Context, Handler, System};
    use mockall::predicate::{eq, function};
    use std::io;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    use crate::network::{AddStream, Listen, SendPacket};
    use common::socket::{test_util::MockStream as Stream, SocketEnd};

    use super::connection::test::connection_new_context;
    use super::listener::test::listener_new_context;
    use super::Listener;
    use super::{Connection, ConnectionHandler};
    use common::socket::{test_util::MockTcpStream as TcpStream, ReceivedPacket};
    use common::socket::{PacketRecv, SocketError};

    pub struct Receiver<R: PacketRecv> {
        pub received: Arc<Mutex<Vec<ReceivedPacket<R>>>>,
    }
    impl<R: PacketRecv> Actor for Receiver<R> {
        type Context = Context<Self>;
    }
    impl<R: PacketRecv> Handler<ReceivedPacket<R>> for Receiver<R> {
        type Result = ();

        fn handle(&mut self, msg: ReceivedPacket<R>, _ctx: &mut Self::Context) {
            self.received.lock().unwrap().push(msg);
        }
    }

    type CH = ConnectionHandler<Vec<u8>, Vec<u8>>;
    macro_rules! local {
        () => {
            SocketAddr::from(([127, 0, 0, 1], 25000))
        };
    }

    #[test]
    fn test_send_packet() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![1, 2, 3, 4, 5];
        let data_c = data.clone();

        let addr = SocketAddr::from(([127, 0, 0, 1], 1234));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(data))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context();
            let handler =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                    eq(None),
                )
                .times(1)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            handler
                .send(SendPacket {
                    to: addr,
                    data: data_c,
                })
                .await
                .unwrap()
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_receive_packet() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data: Vec<u8> = (0..100).collect();
        let data_c = data.clone();

        let addr = SocketAddr::from(([1, 1, 1, 1], 80));

        sys.block_on(async move {
            let r_addr = receiver.start();
            let handler: Addr<ConnectionHandler<Vec<u8>, Vec<u8>>> =
                ConnectionHandler::new(r_addr.recipient(), local!(), true, None).start();

            handler
                .send(ReceivedPacket { addr, data: data_c })
                .await
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 1);
        assert_eq!(received.lock().unwrap()[0].data, data);
        assert_eq!(received.lock().unwrap()[0].addr, addr);
    }

    #[test]
    fn test_timeout_restarts_when_sending_twice() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![1, 2, 3, 4, 5];
        let data_1: Vec<u8> = (3..211).collect();
        let data_c = data.clone();
        let data_1_c: Vec<u8> = data_1.clone();

        let addr = SocketAddr::from(([170, 123, 200, 15], 5000));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(2).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(data))
                .returning(|_| Box::pin(async { Ok(()) }));
            conn.expect_send()
                .times(1)
                .with(eq(data_1))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context();
            let handler: Addr<ConnectionHandler<Vec<u8>, Vec<u8>>> =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                    eq(None),
                )
                .times(1)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            // First send should create Connection and restart timeout
            handler
                .send(SendPacket {
                    to: addr,
                    data: data_c,
                })
                .await
                .unwrap()
                .unwrap();

            // Second send should get existing connection and restart timeout
            handler
                .send(SendPacket {
                    to: addr,
                    data: data_1_c,
                })
                .await
                .unwrap()
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_timeout_restarts_when_receiving() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![10, 3, 2];
        let data_1: Vec<u8> = (5..20).collect();
        let data_c = data.clone();
        let data_1_c: Vec<u8> = data_1.clone();

        let addr = SocketAddr::from(([34, 54, 12, 65], 1883));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(2).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(data))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context();
            let handler =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                    eq(None),
                )
                .times(1)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            // First send should create Connection and restart timeout
            handler
                .send(SendPacket {
                    to: addr,
                    data: data_c,
                })
                .await
                .unwrap()
                .unwrap();

            // Receiving from that connection should restart it again
            handler
                .send(ReceivedPacket {
                    addr,
                    data: data_1_c,
                })
                .await
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 1);
        assert_eq!(received.lock().unwrap()[0].data, data_1);
    }

    #[test]
    fn test_socket_fail() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![23, 43, 12, 5];
        let data_c = data.clone();

        let addr = SocketAddr::from(([23, 43, 12, 5], 25565));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(data))
                .returning(|_| Box::pin(async { Err(SocketError::new("Mock failed")) }));
            let guard = connection_new_context();
            let handler =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                    eq(None),
                )
                .times(1)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            assert!(handler
                .send(SendPacket {
                    to: addr,
                    data: data_c
                })
                .await
                .unwrap()
                .is_err());
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_add_stream() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let addr = SocketAddr::from(([1, 2, 6, 24], 120));

        sys.block_on(async move {
            let receiver = receiver.start();
            let conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            let guard = connection_new_context();
            let handler: Addr<ConnectionHandler<Vec<u8>, Vec<u8>>> =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::Existing(_))),
                    eq(None),
                )
                .times(1)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            handler
                .send(AddStream {
                    stream: TcpStream::default(),
                    addr,
                })
                .await
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_socket_end() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver = Receiver {
            received: received.clone(),
        };

        let data = vec![1, 2, 3, 4, 5];
        let data_1: Vec<u8> = (3..211).collect();
        let data_c = data.clone();
        let data_1_c: Vec<u8> = data_1.clone();

        let addr = SocketAddr::from(([170, 123, 200, 15], 5000));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut conn: Connection<Vec<u8>, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(data))
                .returning(|_| Box::pin(async { Ok(()) }));
            let mut conn_1 = Connection::default();
            conn_1.expect_restart_timeout().times(1).return_const(());
            conn_1
                .expect_send()
                .times(1)
                .with(eq(data_1))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context();
            let handler =
                ConnectionHandler::new(receiver.recipient(), local!(), true, None).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn, conn_1];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone().recipient()),
                    eq(handler_c.recipient()),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                    eq(None),
                )
                .times(2)
                .returning(move |_, _, _, _, _| conn_v.remove(0));

            // First send should create Connection and restart timeout
            handler
                .send(SendPacket {
                    to: addr,
                    data: data_c,
                })
                .await
                .unwrap()
                .unwrap();

            // Connection should be deleted
            handler.send(SocketEnd { addr }).await.unwrap();

            // Second send should create connection again and restart timeout
            handler
                .send(SendPacket {
                    to: addr,
                    data: data_1_c,
                })
                .await
                .unwrap()
                .unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_bind_listener() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver: Receiver<Vec<u8>> = Receiver {
            received: received.clone(),
        };

        let addr = SocketAddr::from(([127, 0, 0, 1], 1234));

        sys.block_on(async move {
            let receiver = receiver.start();
            let mut listener: Listener = Listener::default();
            let handler: Addr<ConnectionHandler<Vec<u8>, Vec<u8>>> =
                ConnectionHandler::new(receiver.recipient(), addr, true, None).start();
            listener
                .expect_run()
                .times(1)
                .with(eq(handler.clone().recipient()))
                .returning(|_| actix_rt::spawn(async {}));
            let guard = listener_new_context();
            let mut listener_v = vec![listener];

            guard
                .ctx
                .expect()
                .with(eq(addr))
                .times(1)
                .returning(move |_| Ok(listener_v.pop().unwrap()));

            handler.send(Listen {}).await.unwrap().unwrap();
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_listener_failed() {
        let sys = System::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let receiver: Receiver<Vec<u8>> = Receiver {
            received: received.clone(),
        };

        let addr = SocketAddr::from(([127, 0, 0, 1], 1234));

        sys.block_on(async move {
            let receiver = receiver.start();
            let handler: Addr<ConnectionHandler<Vec<u8>, Vec<u8>>> =
                ConnectionHandler::new(receiver.recipient(), addr, true, None).start();
            let guard = listener_new_context();

            guard
                .ctx
                .expect()
                .with(eq(addr))
                .times(1)
                .returning(move |_| Err(io::Error::new(io::ErrorKind::Other, "test")));

            assert!(handler.send(Listen {}).await.unwrap().is_err());
        });

        assert_eq!(received.lock().unwrap().len(), 0);
    }
}
