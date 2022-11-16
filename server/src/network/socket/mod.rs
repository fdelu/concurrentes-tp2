use std::io;
use std::marker::Send;
use std::net::{IpAddr, SocketAddr};

#[cfg(test)]
use super::messages::tests::MockTcpStream as TcpStream;
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
#[cfg(not(test))]
use actix_rt::net::TcpSocket;
#[cfg(not(test))]
use actix_rt::net::TcpStream;
use common::AHandler;
use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(test)]
use tests::MockTcpSocket as TcpSocket;
use tokio::spawn;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use super::error::SocketError;

mod messages;
mod read;
mod write;

use self::read::ReaderLoop;
use self::write::WriterLoop;

pub use self::write::MAX_MESSAGE_SIZE;
pub use messages::*;

pub(crate) type OnEnd = Box<dyn Fn() + Send + 'static>;
pub(crate) type OnRead<T: DeserializeOwned> = Box<dyn Fn(T) + Send + 'static>;

pub(crate) const PACKET_SEP: u8 = b"\n";

pub trait Packet: Serialize + DeserializeOwned {}
impl<T: Serialize + DeserializeOwned> Packet for T {}

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

        let on_end = Self::on_end(end_handler.clone(), my_addr);
        let write_handle = spawn(async move {
            WriterLoop::new(writer, write_receiver, on_end).run().await;
        });

        let on_end = Self::on_end(end_handler.clone(), my_addr);
        let on_read = Self::on_read(received_handler, my_addr);
        let read_handle = spawn(async move {
            ReaderLoop::new(reader, on_read, on_end).run().await;
        });

        stop_rx.await.ok();
        write_handle.abort();
        read_handle.abort();

        Ok(())
    }

    fn on_end<B: AHandler<SocketEnd>>(actor: Addr<B>, my_addr: SocketAddr) -> OnEnd {
        Box::new(move || {
            actor.do_send(SocketEnd { addr: my_addr });
        })
    }

    fn on_read<A: AHandler<ReceivedPacket<T>>>(actor: Addr<A>, my_addr: SocketAddr) -> OnRead<T> {
        Box::new(move |data: Vec<u8>| {
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
            Box::pin(async move { Err(e.into()) }.into_actor(self))
        } else {
            Box::pin(async move { result_rx.await? }.into_actor(self))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        io,
        net::SocketAddr,
        sync::{Arc, Mutex},
    };

    use actix::{Actor, Addr, Context, Handler};
    use mockall::mock;

    use super::{Packet, ReceivedPacket, SocketEnd, SocketSend, Stream, TcpStream};
    use crate::network::error::SocketError;
    use common::AHandler;

    mock! {
        pub TcpSocket {
            pub fn new_v4() -> io::Result<Self>;
            pub fn new_v6() -> io::Result<Self>;
            pub fn bind(&self, addr: SocketAddr) -> io::Result<()>;
            pub async fn connect(self, addr: SocketAddr) -> io::Result<TcpStream>;
        }
    }

    pub struct MockSocket<T: Packet> {
        pub sent: Arc<Mutex<Vec<T>>>,
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
            _: Stream,
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
