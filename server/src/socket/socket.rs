use std::marker::Send;
use std::net::SocketAddr;

use actix::dev::ToEnvelope;
use actix::{Actor, Addr, Context, Handler, Message};
use actix_rt::task::JoinHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::spawn;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub(crate) struct Socket {
    read_handle: JoinHandle<()>,
    write_handle: JoinHandle<()>,
    tx: UnboundedSender<Vec<u8>>,
}

impl Actor for Socket {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.read_handle.abort();
        self.write_handle.abort();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct SocketSend {
    pub(crate) data: Vec<u8>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub(crate) struct SocketReceived {
    pub(crate) data: Vec<u8>,
    pub(crate) from: SocketAddr,
}

async fn write_loop(mut writer: impl AsyncWriteExt + Unpin, mut rx: UnboundedReceiver<Vec<u8>>) {
    while let Some(data) = rx.recv().await {
        if let Err(e) = writer.write_all(&data).await {
            println!("Error writing to socket: {}", e);
            break;
        }
    }
}

async fn read_loop<A>(mut reader: impl AsyncReadExt + Unpin, actor: Addr<A>, from: SocketAddr)
where
    A: Actor + Handler<SocketReceived>,
    A::Context: ToEnvelope<A, SocketReceived>,
{
    let mut buf = vec![];
    while let Ok(n) = reader.read(&mut buf).await {
        if n == 0 {
            break;
        }
        println!("Read: {:?}", buf);
        actor.do_send(SocketReceived { data: buf, from });
        buf = vec![];
    }
}

impl Socket {
    pub fn new<A>(
        read_to: Addr<A>,
        reader: impl AsyncReadExt + Unpin + Send + Sync + 'static,
        writer: impl AsyncWriteExt + Unpin + Send + Sync + 'static,
        addr: SocketAddr,
    ) -> Socket
    where
        A: Actor + Handler<SocketReceived>,
        A::Context: ToEnvelope<A, SocketReceived>,
    {
        let (tx, rx) = unbounded_channel();

        let write_handle = spawn(async move {
            write_loop(writer, rx).await;
        });
        let read_handle = spawn(async move {
            read_loop(reader, read_to, addr).await;
        });

        Socket {
            read_handle,
            write_handle,
            tx,
        }
    }
}

impl Handler<SocketSend> for Socket {
    type Result = ();

    fn handle(&mut self, msg: SocketSend, _ctx: &mut Context<Self>) -> Self::Result {
        self.tx.send(msg.data).unwrap();
    }
}

#[cfg(test)]
mod test {
    use actix::{Actor, Context, Handler, System};
    use std::net::ToSocketAddrs;
    use tokio_test::io::Builder;

    use super::{Socket, SocketReceived};

    struct MockActor {
        expected: Vec<Vec<u8>>,
    }

    impl Actor for MockActor {
        type Context = Context<Self>;
    }

    impl Handler<SocketReceived> for MockActor {
        type Result = ();

        fn handle(&mut self, msg: SocketReceived, _ctx: &mut Context<Self>) -> Self::Result {
            let item = self.expected.remove(0);
            assert!(item == msg.data);
            if self.expected.len() == 0 {
                System::current().stop();
            }
        }
    }

    #[test]
    fn test_read_loop() {
        let system = System::new();
        system.block_on(async {
            let reader = Builder::new().read(b"hello").read(b"world").build();
            let writer = Builder::new().build();

            let mock_actor = MockActor {
                expected: vec![b"hello".to_vec(), b"world".to_vec()],
            }
            .start();
            Socket::new(
                mock_actor,
                reader,
                writer,
                "localhost:1234".to_socket_addrs().unwrap().next().unwrap(),
            )
            .start();
        });

        system.run().unwrap();
    }
}
