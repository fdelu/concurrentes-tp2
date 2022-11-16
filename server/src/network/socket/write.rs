use serde::Serialize;
use tokio::{io::AsyncWriteExt, sync::mpsc::UnboundedReceiver};

use crate::network::error::SocketError;

use super::{OnEnd, WriterSend, PACKET_SEP};

pub(crate) const MAGIC_NUMBER: [u8; 2] = [0x52, 0x56]; // decimal [82, 86] "RV"
pub(crate) const LEN_BYTES: usize = 2; // u16, no cambiar
pub const MAX_MESSAGE_SIZE: usize = 1 << (8 * LEN_BYTES);

pub struct WriterLoop<T: AsyncWriteExt + Unpin, P: Serialize> {
    writer: T,
    rx: UnboundedReceiver<WriterSend<P>>,
    on_end: OnEnd,
}

impl<T: AsyncWriteExt + Unpin, P: Serialize> WriterLoop<T, P> {
    pub(crate) fn new(writer: T, rx: UnboundedReceiver<WriterSend<P>>, on_end: OnEnd) -> Self {
        Self { writer, rx, on_end }
    }

    async fn write(&mut self, data: P) -> Result<(), SocketError> {
        let to_send = serde_json::to_vec(&data)?;
        to_send.push(PACKET_SEP);
        self.writer.write_all(&to_send).await?;
        Ok(())
    }

    async fn process_message(&mut self) -> Result<(), SocketError> {
        self.write(self.rx.recv().await?).await
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.process_message() {
                eprintln!("Error in WriterLoop from stream: {}", e);
            }
        }
        (self.on_end)();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    };

    use super::*;
    use tokio::sync::{mpsc::unbounded_channel, oneshot};
    use tokio_test::{block_on, io::Builder};

    #[test]
    fn test_write_basic() {
        let input = [1, 2, 2, 1];
        let mut expected = input.to_vec();
        expected.push(PACKET_SEP);

        let mock_writer = Builder::new().write(&expected).build();
        let (tx, rx) = unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();
        tx.send(WriterSend {
            data: input,
            result: Some(result_tx),
        })
        .unwrap();
        drop(tx);

        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));
        let socket_read = WriterLoop::new(mock_writer, rx, on_end);

        let res = block_on(async move {
            socket_read.run().await;
            result_rx.await
        })
        .unwrap();

        assert!(ended.load(Relaxed));
        assert!(res.is_ok());
    }

    #[test]
    fn test_write_twice() {
        let input_1 = [1, 2, 2, 1];
        let mut expected_1 = input_1.to_vec();
        expected_1.push(PACKET_SEP);

        let input_2 = [9, 7];
        let mut expected_2 = input_2.to_vec();
        expected_2.push(PACKET_SEP);

        let mock_writer = Builder::new().write(&expected_1).write(&expected_2).build();
        let (tx, rx) = unbounded_channel();
        let (result_tx_1, result_rx_1) = oneshot::channel();
        let (result_tx_2, result_rx_2) = oneshot::channel();
        tx.send(WriterSend {
            data: input_1,
            result: Some(result_tx_1),
        })
        .unwrap();
        tx.send(WriterSend {
            data: input_2,
            result: Some(result_tx_2),
        })
        .unwrap();
        drop(tx);

        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));
        let socket_read = WriterLoop::new(mock_writer, rx, on_end);

        let (res_1, res_2) = block_on(async move {
            socket_read.run().await;
            (result_rx_1.await.unwrap(), result_rx_2.await.unwrap())
        });

        assert!(ended.load(Relaxed));
        assert!(res_1.is_ok());
        assert!(res_2.is_ok());
    }
}
