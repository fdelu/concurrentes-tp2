use serde::Serialize;
use tokio::{io::AsyncWriteExt, sync::mpsc::UnboundedReceiver};

use super::SocketError;

use super::{WriterSend, PACKET_SEP};

pub struct WriterLoop<T: AsyncWriteExt + Unpin, P: Serialize> {
    writer: T,
    rx: UnboundedReceiver<WriterSend<P>>,
}

impl<T: AsyncWriteExt + Unpin, P: Serialize> WriterLoop<T, P> {
    pub(crate) fn new(writer: T, rx: UnboundedReceiver<WriterSend<P>>) -> Self {
        Self { writer, rx }
    }

    async fn write(&mut self, data: P) -> Result<(), SocketError> {
        let mut to_send = serde_json::to_vec(&data)?;
        to_send.push(PACKET_SEP);
        self.writer.write_all(&to_send).await?;
        Ok(())
    }

    async fn process_message(&mut self) -> Result<bool, SocketError> {
        match self.rx.recv().await {
            Some(data) => {
                let result = self.write(data.data).await;
                if let Some(result_tx) = data.result {
                    result_tx
                        .send(result.clone())
                        .map_err(|_| SocketError::new("Failed to send WriterLoop result"))?;
                }
                result.map(|_| true)
            }
            None => Ok(false),
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.process_message().await {
                Err(e) => eprintln!("Error in WriterLoop from stream: {}", e),
                Ok(false) => break,
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{mpsc::unbounded_channel, oneshot};
    use tokio_test::{block_on, io::Builder};

    #[test]
    fn test_write_basic() {
        let input = [1, 2, 2, 1];
        let mut expected = serde_json::to_vec(&input).unwrap();
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

        let socket_read = WriterLoop::new(mock_writer, rx);

        let res = block_on(async move {
            socket_read.run().await;
            result_rx.await
        })
        .unwrap();

        assert!(res.is_ok());
    }

    #[test]
    fn test_write_twice() {
        let input_1 = vec![1, 2, 2, 1];
        let mut expected_1 = serde_json::to_vec(&input_1).unwrap();
        expected_1.push(PACKET_SEP);

        let input_2 = vec![9, 7];
        let mut expected_2 = serde_json::to_vec(&input_2).unwrap();
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

        let socket_read = WriterLoop::new(mock_writer, rx);

        let (res_1, res_2) = block_on(async move {
            socket_read.run().await;
            (result_rx_1.await.unwrap(), result_rx_2.await.unwrap())
        });

        assert!(res_1.is_ok());
        assert!(res_2.is_ok());
    }
}
