use std::marker::Send;
use std::net::SocketAddr;

use actix::dev::ToEnvelope;
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
use actix_rt::net::TcpStream;
use tokio::spawn;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

mod messages;
mod read;
mod write;

pub use messages::*;

use self::read::SocketRead;
use self::write::SocketWrite;

use super::error::SocketError;

pub(crate) type OnEnd = Box<dyn Fn() + Send + 'static>;
pub(crate) type OnRead = Box<dyn Fn(Vec<u8>) + Send + 'static>;

pub use self::write::MAX_MESSAGE_SIZE;

pub struct Socket {
    write_sender: UnboundedSender<WriterSend>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl Actor for Socket {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.stop_tx.take().and_then(|tx| tx.send(()).ok());
    }
}

impl Socket {
    pub fn new<A>(
        actor_read_to: Addr<A>,
        socket_addr: SocketAddr,
        stream: Option<TcpStream>,
    ) -> Socket
    where
        A: Actor + Handler<SocketReceived> + Handler<SocketEnd>,
        A::Context: ToEnvelope<A, SocketReceived> + ToEnvelope<A, SocketEnd>,
    {
        let (write_sender, write_receiver) = unbounded_channel();
        let actor_end = actor_read_to.clone();

        let (stop_tx, stop_rx) = oneshot::channel();
        spawn(async move {
            if Self::setup_runners(actor_read_to, stream, socket_addr, write_receiver, stop_rx)
                .await
                .is_err()
            {
                actor_end.do_send(SocketEnd { addr: socket_addr });
            }
        });

        Socket {
            write_sender,
            stop_tx: Some(stop_tx),
        }
    }

    async fn setup_runners<A>(
        actor: Addr<A>,
        stream: Option<TcpStream>,
        my_addr: SocketAddr,
        write_receiver: UnboundedReceiver<WriterSend>,
        stop_rx: oneshot::Receiver<()>,
    ) -> Result<(), SocketError>
    where
        A: Actor + Handler<SocketReceived> + Handler<SocketEnd>,
        A::Context: ToEnvelope<A, SocketReceived> + ToEnvelope<A, SocketEnd>,
    {
        let stream = match stream {
            Some(stream) => stream,
            None => TcpStream::connect(my_addr).await?,
        };
        let (reader, writer) = stream.into_split();

        let on_end = Self::on_end(actor.clone(), my_addr);
        let write_handle = spawn(async move {
            SocketWrite::new(writer, write_receiver, on_end).run().await;
        });

        let on_end = Self::on_end(actor.clone(), my_addr);
        let on_read = Self::on_read(actor, my_addr);
        let read_handle = spawn(async move {
            SocketRead::new(reader, on_read, on_end).run().await;
        });

        stop_rx.await.ok();
        write_handle.abort();
        read_handle.abort();

        Ok(())
    }

    fn on_end<A>(actor: Addr<A>, my_addr: SocketAddr) -> OnEnd
    where
        A: Actor + Handler<SocketReceived> + Handler<SocketEnd>,
        A::Context: ToEnvelope<A, SocketReceived> + ToEnvelope<A, SocketEnd>,
    {
        Box::new(move || {
            actor.do_send(SocketEnd { addr: my_addr });
        })
    }

    fn on_read<A>(actor: Addr<A>, my_addr: SocketAddr) -> OnRead
    where
        A: Actor + Handler<SocketReceived> + Handler<SocketEnd>,
        A::Context: ToEnvelope<A, SocketReceived> + ToEnvelope<A, SocketEnd>,
    {
        Box::new(move |data: Vec<u8>| {
            actor.do_send(SocketReceived {
                data,
                addr: my_addr,
            });
        })
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
