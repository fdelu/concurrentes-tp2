use std::sync::mpsc::Sender;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::{spawn, JoinHandle},
};

struct SocketRead<T: AsyncReadExt + Unpin> {
    reader: T,
    tx: Sender<Vec<u8>>,
}

impl<T: AsyncReadExt + Unpin> SocketRead<T> {
    fn new(reader: T, tx: Sender<Vec<u8>>) -> Self {
        Self { reader, tx }
    }

    async fn run(mut self) {
        let mut buf = vec![];
        while let Ok(size) = self.reader.read(&mut buf).await {
            if size == 0 {
                break;
            }

            self.tx.send(buf).unwrap();
            buf = vec![];
        }
    }
}

struct SocketWrite<T: AsyncWriteExt + Unpin> {
    writer: T,
    rx: UnboundedReceiver<Vec<u8>>,
}

impl<T: AsyncWriteExt + Unpin> SocketWrite<T> {
    fn new(writer: T, rx: UnboundedReceiver<Vec<u8>>) -> Self {
        Self { writer, rx }
    }

    async fn run(mut self) {
        while let Some(data) = self.rx.recv().await {
            self.writer.write_all(&data).await.unwrap();
        }
    }
}

pub struct Socket {
    writer_handle: JoinHandle<()>,
    reader_handle: JoinHandle<()>,
    write_from: UnboundedSender<Vec<u8>>,
}

impl Socket {
    pub fn new(stream: TcpStream, read_to: Sender<Vec<u8>>) -> Socket {
        let (reader, writer) = stream.into_split();
        let (write_from, rx) = tokio::sync::mpsc::unbounded_channel();
        let writer = SocketWrite::new(writer, rx);
        let reader = SocketRead::new(reader, read_to);

        let writer_handle = spawn(async {
            writer.run().await;
        });
        let reader_handle = spawn(async {
            reader.run().await;
        });

        Socket {
            writer_handle,
            reader_handle,
            write_from,
        }
    }

    pub fn send(&self, data: Vec<u8>) {
        self.write_from.send(data).unwrap();
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.writer_handle.abort();
        self.reader_handle.abort();
    }
}
