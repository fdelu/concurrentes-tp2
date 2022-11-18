#![cfg_attr(any(test, feature = "socket_test"), allow(dead_code))]
use std::io;
use std::marker::Send;
use std::net::{IpAddr, SocketAddr};

use crate::AHandler;
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
#[cfg(not(test))]
type TcpSocket = actix_rt::net::TcpSocket;
#[cfg(not(test))]
type TcpStream = actix_rt::net::TcpStream;
use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(test)]
use test_util::MockTcpSocket as TcpSocket;
#[cfg(test)]
use test_util::MockTcpStream as TcpStream;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::{select, spawn};

mod error;
mod messages;
mod read;
mod write;

use self::read::ReaderLoop;
use self::write::WriterLoop;
pub use error::SocketError;
pub use messages::*;

pub(crate) type OnRead<T> = Box<dyn Fn(T) + Send + 'static>;

pub(crate) const PACKET_SEP: u8 = b'\n';

pub trait Packet: Serialize + DeserializeOwned + Send + Unpin + 'static {}
impl<T: Serialize + DeserializeOwned + Send + Unpin + 'static> Packet for T {}

pub enum Stream {
    Existing(TcpStream),
    NewBindedTo(IpAddr),
}

pub struct Socket<T: Packet> {
    write_sender: UnboundedSender<WriterSend<T>>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl<T: Packet> Socket<T> {
    pub fn new<A: AHandler<ReceivedPacket<T>>, B: AHandler<SocketEnd>>(
        received_handler: Addr<A>,
        end_handler: Addr<B>,
        socket_addr: SocketAddr,
        stream: Stream,
    ) -> Socket<T> {
        let (write_sender, write_receiver) = unbounded_channel();
        let end_h = end_handler.clone();

        let (stop_tx, stop_rx) = oneshot::channel();
        spawn(async move {
            if Self::run(
                received_handler,
                end_handler,
                stop_rx,
                stream,
                socket_addr,
                write_receiver,
            )
            .await
            .is_err()
            {
                end_h.do_send(SocketEnd { addr: socket_addr });
            }
        });

        Socket {
            write_sender,
            stop_tx: Some(stop_tx),
        }
    }

    async fn new_tcp_stream(bind_to: IpAddr, addr: SocketAddr) -> io::Result<TcpStream> {
        let socket = match bind_to {
            IpAddr::V4(_) => TcpSocket::new_v4(),
            IpAddr::V6(_) => TcpSocket::new_v6(),
        }?;
        socket.bind(SocketAddr::new(bind_to, 0))?; // Bind to any port
        socket.connect(addr).await
    }

    async fn run<A: AHandler<ReceivedPacket<T>>, B: AHandler<SocketEnd>>(
        received_handler: Addr<A>,
        end_handler: Addr<B>,
        stop_rx: oneshot::Receiver<()>,
        stream: Stream,
        my_addr: SocketAddr,
        write_receiver: UnboundedReceiver<WriterSend<T>>,
    ) -> Result<(), SocketError> {
        let stream = match stream {
            Stream::Existing(stream) => stream,
            Stream::NewBindedTo(bind_to) => Self::new_tcp_stream(bind_to, my_addr).await?,
        };
        let (reader, writer) = stream.into_split();

        let on_read = Self::on_read(received_handler, my_addr);
        let receiver = ReaderLoop::new(reader, on_read).run();

        let writer = WriterLoop::new(writer, write_receiver).run();

        // Wait for either the writer or the receiver to end, or a stop signal
        select! {
            () = writer => (),
            _ = receiver => (),
            _ = stop_rx => ()
        }
        Self::on_end(end_handler, my_addr);

        Ok(())
    }

    fn on_end<B: AHandler<SocketEnd>>(actor: Addr<B>, my_addr: SocketAddr) {
        actor.do_send(SocketEnd { addr: my_addr })
    }

    fn on_read<A: AHandler<ReceivedPacket<T>>>(actor: Addr<A>, my_addr: SocketAddr) -> OnRead<T> {
        Box::new(move |data: T| {
            actor.do_send(ReceivedPacket {
                data,
                addr: my_addr,
            });
        })
    }
}

impl<T: Packet> Actor for Socket<T> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.stop_tx.take().and_then(|tx| tx.send(()).ok());
    }
}

impl<T: Packet> Handler<SocketSend<T>> for Socket<T> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SocketSend<T>, _ctx: &mut Context<Self>) -> Self::Result {
        let (result_tx, result_rx) = oneshot::channel();

        if let Err(e) = self.write_sender.send(WriterSend {
            data: msg.data,
            result: Some(result_tx),
        }) {
            Box::pin(async move { Err(SocketError::new(&e.to_string())) }.into_actor(self))
        } else {
            Box::pin(async move { result_rx.await? }.into_actor(self))
        }
    }
}

#[cfg(any(test, feature = "socket_test"))]
pub mod test_util {
    use std::{
        io,
        net::{IpAddr, SocketAddr},
        sync::{Arc, Mutex},
    };

    use actix::{Actor, Addr, Context, Handler};
    use mockall::mock;
    use tokio::net::{tcp::OwnedReadHalf, unix::OwnedWriteHalf, ToSocketAddrs};

    use super::SocketError;
    use super::{Packet, ReceivedPacket, SocketEnd, SocketSend};
    use crate::AHandler;

    mock! {
        pub TcpSocket {
            pub fn new_v4() -> io::Result<Self>;
            pub fn new_v6() -> io::Result<Self>;
            pub fn bind(&self, addr: SocketAddr) -> io::Result<()>;
            pub async fn connect(self, addr: SocketAddr) -> io::Result<MockTcpStream>;
        }
    }
    mock! {
        pub TcpStream {
            pub async fn connect(addr: SocketAddr) -> io::Result<Self>;
            pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf);
        }
    }
    mock! {
        pub TcpListener {
            pub async fn accept(&mut self) -> io::Result<(MockTcpStream, SocketAddr)>;
            pub fn local_addr(&self) -> io::Result<SocketAddr>;
            pub async fn bind<A: ToSocketAddrs + 'static>(addr: A) -> io::Result<Self>;
        }
    }
    pub enum MockStream {
        Existing(MockTcpStream),
        NewBindedTo(IpAddr),
    }

    pub struct MockSocket<T: Packet> {
        pub sent: Arc<Mutex<Vec<SocketSend<T>>>>,
        fail: bool,
    }
    impl<T: Packet> Actor for MockSocket<T> {
        type Context = Context<Self>;
    }
    impl<T: Packet> Handler<SocketSend<T>> for MockSocket<T> {
        type Result = Result<(), SocketError>;

        fn handle(
            &mut self,
            msg: SocketSend<T>,
            _ctx: &mut Context<Self>,
        ) -> Result<(), SocketError> {
            if self.fail {
                return Err(SocketError::new("MockSocket failed"));
            }
            self.sent.lock().unwrap().push(msg);
            Ok(())
        }
    }
    impl<T: Packet> MockSocket<T> {
        pub fn new<A: AHandler<ReceivedPacket<T>>, B: AHandler<SocketEnd>>(
            _: Addr<A>,
            _: Addr<B>,
            _: SocketAddr,
            _: MockStream,
        ) -> Self {
            Self::get(Arc::new(Mutex::new(Vec::new())))
        }

        pub fn get(sent: Arc<Mutex<Vec<SocketSend<T>>>>) -> Self {
            MockSocket { sent, fail: false }
        }

        pub fn get_failing() -> Self {
            MockSocket {
                sent: Arc::new(Mutex::new(Vec::new())),
                fail: true,
            }
        }
    }
}
