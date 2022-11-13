use std::mem::swap;

use tokio::io::AsyncReadExt;

use crate::network::error::SocketError;

use super::{OnEnd, OnRead};

use super::write::{LEN_BYTES, MAGIC_NUMBER};

pub struct ReaderLoop<T: AsyncReadExt + Unpin> {
    reader: T,
    on_read: OnRead,
    on_end: OnEnd,
}

impl<T: AsyncReadExt + Unpin> ReaderLoop<T> {
    pub fn new(reader: T, on_read: OnRead, on_end: OnEnd) -> Self {
        Self {
            reader,
            on_read,
            on_end,
        }
    }

    fn check_message(message: &mut Vec<u8>, on_read: &mut OnRead) -> Result<(), SocketError> {
        while message.len() >= MAGIC_NUMBER.len() + LEN_BYTES {
            if !message.starts_with(&MAGIC_NUMBER) {
                // Something went wrong: Magic number does not match
                println!("{:?}", message);
                return Err(SocketError::new("Magic number does not match"));
            }

            let length = u16::from_be_bytes(
                message[MAGIC_NUMBER.len()..MAGIC_NUMBER.len() + LEN_BYTES].try_into()?,
            ) as usize;

            if message.len() < MAGIC_NUMBER.len() + LEN_BYTES + length {
                // We don't have the whole message yet
                break;
            }

            let mut splitted = message.split_off(MAGIC_NUMBER.len() + LEN_BYTES + length);
            swap(message, &mut splitted);
            on_read(splitted.split_off(MAGIC_NUMBER.len() + LEN_BYTES));
        }
        Ok(())
    }

    pub async fn run(mut self) {
        let mut message = vec![];
        let mut buf = [0; 1024];
        while let Ok(size) = self.reader.read(&mut buf).await {
            if size == 0 {
                break;
            }
            message.extend_from_slice(&buf[..size]);
            if let Err(e) = Self::check_message(&mut message, &mut self.on_read) {
                eprintln!("Error reading from stream: {}", e);
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
        mpsc::channel,
        Arc, Mutex,
    };

    use super::*;
    use tokio::{
        io::{duplex, AsyncWriteExt},
        task,
    };
    use tokio_test::{block_on, io::Builder};

    type DataVec = Vec<Vec<u8>>;

    fn setup(input: Vec<Vec<u8>>) -> (Arc<Mutex<DataVec>>, Arc<AtomicBool>) {
        let mut builder = Builder::new();
        for i in input {
            builder.read(&i);
        }
        let mock_reader = builder.build();

        let read = Arc::new(Mutex::new(vec![]));
        let ended = Arc::new(AtomicBool::new(false));
        let read_c = read.clone();
        let ended_c = ended.clone();
        let on_read = Box::new(move |data: Vec<u8>| read_c.lock().unwrap().push(data));
        let on_end = Box::new(move || ended_c.store(true, Relaxed));

        let socket_read = ReaderLoop::new(mock_reader, on_read, on_end);
        block_on(socket_read.run());
        (read, ended)
    }

    #[test]
    fn test_read_basic() {
        let mut input = MAGIC_NUMBER.to_vec();
        input.extend_from_slice(&[0, 4, 1, 2, 3, 4]); // Largo 4, mensaje 1 2 3 4
        let (read, ended) = setup(vec![input]);

        assert_eq!(read.lock().unwrap().clone(), vec![vec![1, 2, 3, 4]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_read_invalid_number() {
        let mut input = MAGIC_NUMBER.to_vec();
        input.extend_from_slice(&[0, 4, 1, 2, 3, 4]); // Largo 4, mensaje 1 2 3 4
        input[0] += 1;

        let (read, ended) = setup(vec![input]);

        assert!(read.lock().unwrap().is_empty());
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_read_twice() {
        let mut input = MAGIC_NUMBER.to_vec();
        input.extend_from_slice(&[0, 2, 9, 3]); // Largo 2, mensaje 9 3
        let mut input_2 = MAGIC_NUMBER.to_vec();
        input_2.extend_from_slice(&[0, 2, 1, 2]); // Largo 2, mensaje 1 2

        let (read, ended) = setup(vec![input, input_2]);

        assert_eq!(read.lock().unwrap().clone(), vec![vec![9, 3], vec![1, 2]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_read_twice_mixed_reads() {
        let mut inputs = vec![];
        MAGIC_NUMBER
            .iter()
            .for_each(|byte| inputs.push(vec![*byte]));
        inputs.push(vec![0, 1, 3, MAGIC_NUMBER[0]]); // Largo 1, mensaje 3, principio del siguiente
        MAGIC_NUMBER[1..]
            .iter()
            .for_each(|byte| inputs.push(vec![*byte]));
        inputs.push(vec![0, 3, 4, 5]);
        inputs.push(vec![4]); // Largo 3, mensaje 4 5 4

        let (read, ended) = setup(inputs);

        assert_eq!(read.lock().unwrap().clone(), vec![vec![3], vec![4, 5, 4]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_not_ended() {
        let mut input = MAGIC_NUMBER.to_vec();
        input.extend_from_slice(&[0, 2, 9, 3]); // Largo 2, mensaje 9 3
        let mut input_2 = MAGIC_NUMBER.to_vec();
        input_2.extend_from_slice(&[0, 2, 1, 2]); // Largo 2, mensaje 1 2

        let (tx, rx) = channel();
        let (mut before_tx, before_rx) = duplex(1024);
        block_on(before_tx.write_all(&input)).unwrap();
        let on_read = Box::new(move |x: Vec<u8>| tx.send(x).unwrap());
        let ended = Arc::new(AtomicBool::new(false));
        let ended_c = ended.clone();
        let ended_c2 = ended.clone();
        let on_end = Box::new(move || ended_c.store(true, Relaxed));

        block_on(async move {
            let socket = ReaderLoop::new(before_rx, on_read, on_end);
            let future = socket.run();
            task::yield_now().await; // more likely que se empiece a ejecutar el socket
            assert!(!ended_c2.load(Relaxed));
            before_tx.write_all(&input_2).await.unwrap();
            drop(before_tx);
            future.await;
        });

        let result = rx.iter().collect::<Vec<Vec<u8>>>();
        assert_eq!(result, vec![vec![9, 3], vec![1, 2]]);
        assert!(ended.load(Relaxed));
    }
}
