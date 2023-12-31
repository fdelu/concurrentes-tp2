#![cfg_attr(any(test, feature = "socket_test"), allow(dead_code))]
use std::io;
use std::marker::{PhantomData, Send};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use actix::{Actor, Context, Handler, Recipient, ResponseActFuture, WrapFuture};
#[cfg(not(test))]
use actix_rt::net::{TcpSocket, TcpStream};
use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(test)]
use test_util::{MockTcpSocket as TcpSocket, MockTcpStream as TcpStream};
use tokio::{select, spawn};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    time::timeout,
};
use tracing::{trace, warn};

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
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub trait PacketSend: Serialize + Send + Sync + Unpin + 'static {}
impl<T: Serialize + Send + Sync + Unpin + 'static> PacketSend for T {}
pub trait PacketRecv: DeserializeOwned + Send + Sync + Unpin + 'static {}
impl<T: DeserializeOwned + Send + Sync + Unpin + 'static> PacketRecv for T {}

/// Opciones para configurar el [TcpStream] que va a usar el socket.
/// Se pasa al [Socket] al momento de instanciarlo.
/// [Stream::Existing]: Usa un [TcpStream] preexistente.
/// [Stream::New]: Crea un nuevo [TcpStream].
/// [Stream::NewBindedTo]: Crea un nuevo [TcpStream], luego de
/// bindearlo a la dirección IP dada. Esto permite que el otro
/// extremo de la conexión obtenga los paquetes desde esta dirección IP.
pub enum Stream {
    Existing(TcpStream),
    NewBindedTo(IpAddr),
    New,
}

/// Un Actor que se encarga de enviar y recibir paquetes a través de un socket.
pub struct Socket<S: PacketSend, R: PacketRecv> {
    // Sends packets to the WriterLoop
    write_tx: UnboundedSender<WriterSend<S>>,
    // Sends a stop signal
    stop_tx: Option<oneshot::Sender<()>>,
    // Type marker for the received packets
    _receiver_type: PhantomData<R>,
}

// Internal helper struct for the socket
struct SocketRunner<R, S>
where
    R: PacketRecv,
    S: PacketSend,
{
    // Actor that receives packets from the socket
    received_handler: Recipient<ReceivedPacket<R>>,
    // Receives the stop signal
    stop_rx: oneshot::Receiver<()>,
    // Contains/creates the TcpStream
    stream: Stream,
    // Where this socket is connected to
    socket_addr: SocketAddr,
    // Receiver for the WriterLoop
    write_rx: UnboundedReceiver<WriterSend<S>>,
    // Type marker for the received packets
    _receiver_type: PhantomData<R>,
}

impl<R, S> SocketRunner<R, S>
where
    R: PacketRecv,
    S: PacketSend,
{
    // Run the ReaderLoop and WriterLoop
    async fn run(self) -> Result<(), SocketError> {
        let on_read = self.on_read();
        let stream = match self.stream {
            Stream::Existing(stream) => stream,
            Stream::NewBindedTo(bind_to) => Self::connect(Some(bind_to), self.socket_addr).await?,
            Stream::New => Self::connect(None, self.socket_addr).await?,
        };
        let (reader, writer) = stream.into_split();

        let receiver = ReaderLoop::new(reader, on_read).run();
        let writer = WriterLoop::new(writer, self.write_rx).run();

        // Wait for either the writer or the receiver to end, or a stop signal
        select! {
            () = writer => (),
            _ = receiver => (),
            _ = self.stop_rx => ()
        }

        Ok(())
    }

    // What to do when a packet is received
    fn on_read(&self) -> OnRead<R> {
        let actor = self.received_handler.clone();
        let addr = self.socket_addr;
        Box::new(move |data: R| {
            actor.do_send(ReceivedPacket { data, addr });
        })
    }

    // Connects a TcpStream to the specified address
    async fn connect(bind_to: Option<IpAddr>, addr: SocketAddr) -> io::Result<TcpStream> {
        trace!("Socket connecting to {}...", addr);
        let stream = timeout(CONNECT_TIMEOUT, async move {
            if let Some(bind_to) = bind_to {
                let socket = match bind_to {
                    IpAddr::V4(_) => TcpSocket::new_v4(),
                    IpAddr::V6(_) => TcpSocket::new_v6(),
                }?;
                socket.bind(SocketAddr::new(bind_to, 0))?; // Bind to any port
                trace!("Binded socket to {} before connecting to {}", bind_to, addr);
                socket.connect(addr).await
            } else {
                TcpStream::connect(addr).await
            }
        })
        .await
        .map_err(|_| {
            SocketError::new(&format!(
                "Connection to {} timed out: failed to connect after {}ms",
                addr,
                CONNECT_TIMEOUT.as_millis()
            ))
        })?;
        trace!("Socket connected to {}", addr);
        stream
    }
}

impl<S: PacketSend, R: PacketRecv> Socket<S, R> {
    /// Crea un nuevo [Socket]. Parámetros:
    /// * `stream`: [Stream] con el que se va a crear el [TcpStream].
    /// * `socket_addr`: Dirección a la que se va a conectar el socket.
    /// * `received_handler`: Actor que va a recibir los paquetes que lleguen.
    /// * `end_handler`: Actor que va a recibir un mensaje cuando el socket se cierre.
    pub fn new(
        received_handler: Recipient<ReceivedPacket<R>>,
        end_handler: Recipient<SocketEnd>,
        socket_addr: SocketAddr,
        stream: Stream,
    ) -> Socket<S, R> {
        let (write_tx, write_rx) = unbounded_channel();

        let (stop_tx, stop_rx) = oneshot::channel();
        spawn(async move {
            if let Err(err) = (SocketRunner {
                received_handler,
                stop_rx,
                stream,
                socket_addr,
                write_rx,
                _receiver_type: PhantomData::default(),
            }
            .run())
            .await
            {
                warn!("Internal socket error: {}", err);
            }
            end_handler.do_send(SocketEnd { addr: socket_addr });
        });

        Socket {
            write_tx,
            stop_tx: Some(stop_tx),
            _receiver_type: PhantomData::default(),
        }
    }
}

impl<S: PacketSend, R: PacketRecv> Actor for Socket<S, R> {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Send a stop signal to the socket
        self.stop_tx.take().and_then(|tx| tx.send(()).ok());
    }
}

impl<S: PacketSend, R: PacketRecv> Handler<SocketSend<S>> for Socket<S, R> {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SocketSend<S>, _ctx: &mut Context<Self>) -> Self::Result {
        let (result_tx, result_rx) = oneshot::channel();

        // Send it over to the WriterLoop
        if let Err(e) = self.write_tx.send(WriterSend {
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
        marker::PhantomData,
        net::{IpAddr, SocketAddr},
        sync::{Arc, Mutex},
    };

    use actix::{Actor, Context, Handler, Recipient};
    use mockall::mock;
    use tokio::net::{tcp::OwnedReadHalf, unix::OwnedWriteHalf, ToSocketAddrs};

    use super::{PacketRecv, PacketSend, SocketError};
    use super::{ReceivedPacket, SocketEnd, SocketSend};

    mock! {
        pub TcpSocket {
            pub fn new_v4() -> io::Result<Self>;
            pub fn new_v6() -> io::Result<Self>;
            pub fn bind(&self, addr: SocketAddr) -> io::Result<()>;
            pub async fn connect(self, addr: SocketAddr) -> io::Result<MockTcpStream>;
            pub fn local_addr(&self) -> io::Result<SocketAddr>;
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
        New,
    }

    pub struct MockSocket<S: PacketSend, R: PacketRecv> {
        pub sent: Arc<Mutex<Vec<SocketSend<S>>>>,
        fail: bool,
        _type: PhantomData<R>,
    }
    impl<S: PacketSend, R: PacketRecv> Actor for MockSocket<S, R> {
        type Context = Context<Self>;
    }
    impl<S: PacketSend, R: PacketRecv> Handler<SocketSend<S>> for MockSocket<S, R> {
        type Result = Result<(), SocketError>;

        fn handle(
            &mut self,
            msg: SocketSend<S>,
            _ctx: &mut Context<Self>,
        ) -> Result<(), SocketError> {
            if self.fail {
                return Err(SocketError::new("MockSocket failed"));
            }
            self.sent.lock().unwrap().push(msg);
            Ok(())
        }
    }
    impl<S: PacketSend, R: PacketRecv> MockSocket<S, R> {
        pub fn new(
            _: Recipient<ReceivedPacket<R>>,
            _: Recipient<SocketEnd>,
            _: SocketAddr,
            _: MockStream,
        ) -> Self {
            Self::get(Arc::new(Mutex::new(Vec::new())))
        }

        pub fn get(sent: Arc<Mutex<Vec<SocketSend<S>>>>) -> Self {
            MockSocket {
                sent,
                fail: false,
                _type: PhantomData::default(),
            }
        }

        pub fn get_failing() -> Self {
            MockSocket {
                sent: Arc::new(Mutex::new(Vec::new())),
                fail: true,
                _type: PhantomData::default(),
            }
        }
    }
}
