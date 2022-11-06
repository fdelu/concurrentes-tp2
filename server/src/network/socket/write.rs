use tokio::{io::AsyncWriteExt, sync::mpsc::UnboundedReceiver};

use crate::network::error::SocketError;

use super::OnEnd;

pub(crate) const MAGIC_NUMBER: [u8; 2] = [0x52, 0x56];
pub(crate) const LEN_BYTES: usize = 2; // u16, no cambiar
const MAX_MESSAGE_SIZE: usize = 1 << (8 * LEN_BYTES);

pub struct SocketWrite<T: AsyncWriteExt + Unpin> {
    writer: T,
    rx: UnboundedReceiver<Vec<u8>>,
    on_end: OnEnd,
}

impl<T: AsyncWriteExt + Unpin> SocketWrite<T> {
    pub fn new(writer: T, rx: UnboundedReceiver<Vec<u8>>, on_end: OnEnd) -> Self {
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
            if let Err(_e) = self.write(data).await {
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
    use tokio::sync::mpsc::unbounded_channel;
    use tokio_test::{block_on, io::Builder};

    #[test]
    fn test_write_basic() {
        let input = [1, 2, 2, 1];
        let mut expected = MAGIC_NUMBER.to_vec();
        expected.extend_from_slice(&[0, 4]); // Largo 4
        expected.extend(&input);
        let mock_writer = Builder::new().write(&expected).build();
        let (tx, rx) = unbounded_channel();
        tx.send(input.to_vec()).unwrap();
        drop(tx);

        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));
        let socket_read = SocketWrite::new(mock_writer, rx, on_end);

        block_on(socket_read.run());

        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_write_twice() {
        let input_1 = [1, 2, 2, 1];
        let mut expected_1 = MAGIC_NUMBER.to_vec();
        expected_1.extend_from_slice(&[0, 4]); // Largo 4
        expected_1.extend(&input_1);

        let input_2 = [9, 7];
        let mut expected_2 = MAGIC_NUMBER.to_vec();
        expected_2.extend_from_slice(&[0, 2]); // Largo 2
        expected_2.extend(&input_2);

        let mock_writer = Builder::new().write(&expected_1).write(&expected_2).build();
        let (tx, rx) = unbounded_channel();
        tx.send(input_1.to_vec()).unwrap();
        tx.send(input_2.to_vec()).unwrap();
        drop(tx);

        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));
        let socket_read = SocketWrite::new(mock_writer, rx, on_end);

        block_on(socket_read.run());

        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_write_too_big() {
        let input: Vec<u8> = (0..(MAX_MESSAGE_SIZE + 1))
            .into_iter()
            .map(|x| u8::try_from(x % 256).unwrap())
            .collect();

        let mock_writer = Builder::new().build(); // Expect nothing
        let (tx, rx) = unbounded_channel();
        tx.send(input).unwrap();
        drop(tx);

        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));
        let socket_read = SocketWrite::new(mock_writer, rx, on_end);

        block_on(socket_read.run());

        assert!(ended.load(Relaxed));
    }
}
