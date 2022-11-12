use std::marker::Send;
use std::net::SocketAddr;

#[cfg(test)]
use super::messages::tests::MockTcpStream as TcpStream;
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
#[cfg(not(test))]
use actix_rt::net::TcpStream;
use common::AHandler;
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
pub(crate) type OnRead = Box<dyn Fn(Vec<u8>) + Send + 'static>;

pub struct Socket {
    write_sender: UnboundedSender<WriterSend>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl Socket {
    pub fn new<A: AHandler<ReceivedPacket>, B: AHandler<SocketEnd>>(
        received_handler: Addr<A>,
        end_handler: Addr<B>,
        socket_addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> Socket {
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

    async fn run<A: AHandler<ReceivedPacket>, B: AHandler<SocketEnd>>(
        received_handler: Addr<A>,
        end_handler: Addr<B>,
        stop_rx: oneshot::Receiver<()>,
        stream: Option<TcpStream>,
        my_addr: SocketAddr,
        write_receiver: UnboundedReceiver<WriterSend>,
    ) -> Result<(), SocketError> {
        let stream = match stream {
            Some(stream) => stream,
            None => TcpStream::connect(my_addr).await?,
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

    fn on_read<A: AHandler<ReceivedPacket>>(actor: Addr<A>, my_addr: SocketAddr) -> OnRead {
        Box::new(move |data: Vec<u8>| {
            actor.do_send(ReceivedPacket {
                data,
                addr: my_addr,
            });
        })
    }
}

impl Actor for Socket {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.stop_tx.take().and_then(|tx| tx.send(()).ok());
    }
}

impl Handler<SocketSend> for Socket {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: SocketSend, _ctx: &mut Context<Self>) -> Self::Result {
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
        net::SocketAddr,
        sync::{Arc, Mutex},
    };

    use actix::{Actor, Addr, Context, Handler};
    use common::AHandler;

    use crate::network::error::SocketError;

    use super::{ReceivedPacket, SocketEnd, SocketSend, TcpStream};

    pub struct MockSocket {
        pub sent: Arc<Mutex<Vec<SocketSend>>>,
        fail: bool,
    }
    impl Actor for MockSocket {
        type Context = Context<Self>;
    }
    impl Handler<SocketSend> for MockSocket {
        type Result = Result<(), SocketError>;

        fn handle(&mut self, msg: SocketSend, _ctx: &mut Context<Self>) -> Result<(), SocketError> {
            if self.fail {
                return Err(SocketError::new("MockSocket failed"));
            }
            self.sent.lock().unwrap().push(msg);
            Ok(())
        }
    }
    impl MockSocket {
        pub fn new<A: AHandler<ReceivedPacket>, B: AHandler<SocketEnd>>(
            _: Addr<A>,
            _: Addr<B>,
            _: SocketAddr,
            _: Option<TcpStream>,
        ) -> Self {
            Self::get(Arc::new(Mutex::new(Vec::new())))
        }

        pub fn get(sent: Arc<Mutex<Vec<SocketSend>>>) -> Self {
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
