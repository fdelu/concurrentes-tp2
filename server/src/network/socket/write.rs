use tokio::{io::AsyncWriteExt, sync::mpsc::UnboundedReceiver};

use crate::network::error::SocketError;

use super::{OnEnd, WriterSend};

pub(crate) const MAGIC_NUMBER: [u8; 2] = [0x52, 0x56]; // decimal [82, 86] "RV"
pub(crate) const LEN_BYTES: usize = 2; // u16, no cambiar
pub const MAX_MESSAGE_SIZE: usize = 1 << (8 * LEN_BYTES);

pub struct WriterLoop<T: AsyncWriteExt + Unpin> {
    writer: T,
    rx: UnboundedReceiver<WriterSend>,
    on_end: OnEnd,
}

impl<T: AsyncWriteExt + Unpin> WriterLoop<T> {
    pub(crate) fn new(writer: T, rx: UnboundedReceiver<WriterSend>, on_end: OnEnd) -> Self {
        Self { writer, rx, on_end }
    }

    async fn write(&mut self, data: Vec<u8>) -> Result<(), SocketError> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(SocketError::new("Message too big"));
        }
        let size16 = u16::try_from(data.len())?;
        self.writer.write_all(&MAGIC_NUMBER).await?;
        self.writer.write_all(&size16.to_be_bytes()).await?;
        self.writer.write_all(&data).await?;
        Ok(())
    }

    pub async fn run(mut self) {
        while let Some(data) = self.rx.recv().await {
            let mut err = false;
            let result = self.write(data.data).await;
            err |= result.is_err();
            if let Some(tx) = data.result {
                err |= tx.send(result).is_err();
            }
            if err {
                break;
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
        let mut expected = MAGIC_NUMBER.to_vec();
        expected.extend_from_slice(&[0, 4]); // Largo 4
        expected.extend(input);
        let mock_writer = Builder::new().write(&expected).build();
        let (tx, rx) = unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();
        tx.send(WriterSend {
            data: input.to_vec(),
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
        let mut expected_1 = MAGIC_NUMBER.to_vec();
        expected_1.extend_from_slice(&[0, 4]); // Largo 4
        expected_1.extend(input_1);

        let input_2 = [9, 7];
        let mut expected_2 = MAGIC_NUMBER.to_vec();
        expected_2.extend_from_slice(&[0, 2]); // Largo 2
        expected_2.extend(input_2);

        let mock_writer = Builder::new().write(&expected_1).write(&expected_2).build();
        let (tx, rx) = unbounded_channel();
        let (result_tx_1, result_rx_1) = oneshot::channel();
        let (result_tx_2, result_rx_2) = oneshot::channel();
        tx.send(WriterSend {
            data: input_1.to_vec(),
            result: Some(result_tx_1),
        })
        .unwrap();
        tx.send(WriterSend {
            data: input_2.to_vec(),
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

    #[test]
    fn test_write_too_big() {
        let input: Vec<u8> = (0..(MAX_MESSAGE_SIZE + 1))
            .into_iter()
            .map(|x| u8::try_from(x % 256).unwrap())
            .collect();

        let mock_writer = Builder::new().build(); // Expect nothing
        let (tx, rx) = unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();
        tx.send(WriterSend {
            data: input.to_vec(),
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
            result_rx.await.unwrap()
        });

        assert!(ended.load(Relaxed));
        assert!(res.is_err());
    }
}
