#![cfg_attr(test, allow(dead_code))]
use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::task::JoinHandle;

use mockall_double::double;

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
use common::socket::{Packet, SocketEnd, SocketError, SocketSend};
use common::AHandler;

pub struct ConnectionHandler<A: AHandler<ReceivedPacket<P>>, P: Packet> {
    connections: HashMap<SocketAddr, Connection<Self, P>>,
    received_handler: Addr<A>,
    join_listener: Option<JoinHandle<()>>,
    bind_to: SocketAddr,
}

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> ConnectionHandler<A, P> {
    pub fn new(received_handler: Addr<A>, bind_to: SocketAddr) -> Self {
        Self {
            connections: HashMap::new(),
            received_handler,
            join_listener: None,
            bind_to,
        }
    }

    fn get_connection(&mut self, this: Addr<Self>, addr: SocketAddr) -> &mut Connection<Self, P> {
        let bind_to = self.bind_to.ip();
        let connection = self.connections.entry(addr).or_insert_with(move || {
            Connection::new(this.clone(), this, addr, Stream::NewBindedTo(bind_to))
        });
        connection
    }
}

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Actor for ConnectionHandler<A, P> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(join) = self.join_listener.take() {
            join.abort();
        }
    }
}

// Public messages

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Handler<SendPacket<P>> for ConnectionHandler<A, P> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SendPacket<P>, ctx: &mut Context<Self>) -> Self::Result {
        let connection = self.get_connection(ctx.address(), msg.to);
        connection.restart_timeout();
        connection
            .send(SocketSend { data: msg.data })
            .into_actor(self)
            .boxed_local()
    }
}

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Handler<Listen> for ConnectionHandler<A, P> {
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
                this.join_listener = Some(listener?.run(ctx.address()));
                Ok(())
            })
            .boxed_local()
    }
}

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Handler<ReceivedPacket<P>>
    for ConnectionHandler<A, P>
{
    type Result = ();

    fn handle(&mut self, msg: ReceivedPacket<P>, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(conn) = self.connections.get_mut(&msg.addr) {
            conn.restart_timeout();
        }

        self.received_handler.do_send(msg);
    }
}

// Private Messages

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Handler<AddStream> for ConnectionHandler<A, P> {
    type Result = ();

    fn handle(&mut self, msg: AddStream, ctx: &mut Self::Context) -> Self::Result {
        let connection = Connection::new(
            ctx.address(),
            ctx.address(),
            msg.addr,
            Stream::Existing(msg.stream),
        );
        self.connections.insert(msg.addr, connection);
    }
}

impl<A: AHandler<ReceivedPacket<P>>, P: Packet> Handler<SocketEnd> for ConnectionHandler<A, P> {
    type Result = ();

    fn handle(&mut self, msg: SocketEnd, _ctx: &mut Self::Context) {
        self.connections.remove(&msg.addr);
    }
}

#[cfg(test)]
mod tests {
    use actix::{Actor, Context, Handler, System};
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
    use common::socket::{Packet, SocketError, SocketSend};

    pub struct Receiver<P: Packet> {
        pub received: Arc<Mutex<Vec<ReceivedPacket<P>>>>,
    }
    impl<P: Packet> Actor for Receiver<P> {
        type Context = Context<Self>;
    }
    impl<P: Packet> Handler<ReceivedPacket<P>> for Receiver<P> {
        type Result = ();

        fn handle(&mut self, msg: ReceivedPacket<P>, _ctx: &mut Self::Context) {
            self.received.lock().unwrap().push(msg);
        }
    }

    type CH = ConnectionHandler<Receiver<Vec<u8>>, Vec<u8>>;
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
            let mut conn: Connection<CH, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data }))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                )
                .times(1)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let handler = ConnectionHandler::new(r_addr, local!()).start();

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
            let mut conn: Connection<CH, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(2).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data }))
                .returning(|_| Box::pin(async { Ok(()) }));
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data: data_1 }))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                )
                .times(1)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let mut conn: Connection<CH, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(2).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data }))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                )
                .times(1)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let mut conn: Connection<CH, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data }))
                .returning(|_| Box::pin(async { Err(SocketError::new("Mock failed")) }));
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                )
                .times(1)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let conn: Connection<CH, Vec<u8>> = Connection::default();
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::Existing(_))),
                )
                .times(1)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let mut conn: Connection<CH, Vec<u8>> = Connection::default();
            conn.expect_restart_timeout().times(1).return_const(());
            conn.expect_send()
                .times(1)
                .with(eq(SocketSend { data }))
                .returning(|_| Box::pin(async { Ok(()) }));
            let mut conn_1: Connection<CH, Vec<u8>> = Connection::default();
            conn_1.expect_restart_timeout().times(1).return_const(());
            conn_1
                .expect_send()
                .times(1)
                .with(eq(SocketSend { data: data_1 }))
                .returning(|_| Box::pin(async { Ok(()) }));
            let guard = connection_new_context::<CH, Vec<u8>>();
            let handler = ConnectionHandler::new(receiver, local!()).start();
            let handler_c = handler.clone();
            let mut conn_v = vec![conn, conn_1];

            guard
                .ctx
                .expect()
                .with(
                    eq(handler_c.clone()),
                    eq(handler_c),
                    eq(addr),
                    function(|s: &Stream| matches!(s, Stream::NewBindedTo(_))),
                )
                .times(2)
                .returning(move |_, _, _, _| conn_v.remove(0));

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
            let handler = ConnectionHandler::new(receiver, addr).start();
            listener
                .expect_run()
                .times(1)
                .with(eq(handler.clone()))
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
            let handler = ConnectionHandler::new(receiver, addr).start();
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
