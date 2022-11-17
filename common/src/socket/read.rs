use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use super::{OnEnd, OnRead, SocketError, PACKET_SEP};

pub struct ReaderLoop<T: AsyncReadExt + Unpin, P: DeserializeOwned> {
    reader: BufReader<T>,
    on_read: OnRead<P>,
    on_end: OnEnd,
}

impl<T: AsyncReadExt + Unpin, P: DeserializeOwned> ReaderLoop<T, P> {
    pub fn new(reader: T, on_read: OnRead<P>, on_end: OnEnd) -> Self {
        Self {
            reader: BufReader::new(reader),
            on_read,
            on_end,
        }
    }

    // Returns message size or 0 if EOF reached
    async fn process_message(&mut self, buffer: &mut Vec<u8>) -> Result<usize, SocketError> {
        buffer.clear();
        let size = self.reader.read_until(PACKET_SEP, buffer).await?;
        (self.on_read)(serde_json::from_slice(buffer)?);

        Ok(size)
    }

    pub async fn run(mut self) {
        let mut buffer = vec![];
        loop {
            match self.process_message(&mut buffer).await {
                Ok(0) => break,
                Err(e) => {
                    eprintln!("Error in ReaderLoop: {}", e);
                    break;
                }
                _ => (),
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

    fn setup<P: DeserializeOwned + Send + 'static>(
        input: Vec<Vec<u8>>,
    ) -> (Arc<Mutex<Vec<P>>>, Arc<AtomicBool>) {
        let mut builder = Builder::new();
        for i in input {
            builder.read(&i);
        }
        let mock_reader = builder.build();

        let read = Arc::new(Mutex::new(vec![]));
        let ended = Arc::new(AtomicBool::new(false));
        let read_c = read.clone();
        let ended_c = ended.clone();
        let on_read = Box::new(move |data: P| read_c.lock().unwrap().push(data));
        let on_end = Box::new(move || ended_c.store(true, Relaxed));

        let socket_read = ReaderLoop::new(mock_reader, on_read, on_end);
        block_on(socket_read.run());
        (read, ended)
    }

    #[test]
    fn test_read_basic() {
        let mut input = vec![1, 2, 3, 4]; // Largo 4, mensaje 1 2 3 4
        input = serde_json::to_vec(&input).unwrap();
        input.push(PACKET_SEP);
        let (read, ended) = setup::<Vec<u8>>(vec![input]);

        assert_eq!(*read.lock().unwrap(), vec![vec![1, 2, 3, 4]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_no_new_line() {
        let mut input = vec![1, 2, 3, 4]; // Largo 4, mensaje 1 2 3 4
        input = serde_json::to_vec(&input).unwrap();
        input.extend(serde_json::to_vec(&input).unwrap());

        let (read, ended) = setup::<Vec<u8>>(vec![input]);

        assert!(read.lock().unwrap().is_empty());
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_read_twice() {
        let mut input = vec![9, 3]; // Largo 2, mensaje 9 3
        input = serde_json::to_vec(&input).unwrap();
        input.push(PACKET_SEP);
        let mut input_2 = vec![1, 2]; // Largo 2, mensaje 1 2
        input_2 = serde_json::to_vec(&input_2).unwrap();
        input_2.push(PACKET_SEP);

        let (read, ended) = setup::<Vec<u8>>(vec![input, input_2]);

        assert_eq!(*read.lock().unwrap(), vec![vec![9, 3], vec![1, 2]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_read_twice_mixed_reads() {
        let mut inputs = vec![];
        inputs.push(serde_json::to_vec(&vec![3]).unwrap()); // [3]
        inputs.push(vec![PACKET_SEP]);
        let mut input_3 = serde_json::to_vec(&vec![4, 5, 4]).unwrap(); // [4, 5, 4]
        input_3.push(PACKET_SEP);
        inputs.push(input_3);

        let (read, ended) = setup::<Vec<u8>>(inputs);

        assert_eq!(*read.lock().unwrap(), vec![vec![3], vec![4, 5, 4]]);
        assert!(ended.load(Relaxed));
    }

    #[test]
    fn test_not_ended() {
        let mut input = serde_json::to_vec(&[9, 3]).unwrap(); // Largo 2, mensaje 9 3
        input.push(PACKET_SEP);
        let mut input_2 = serde_json::to_vec(&[1, 2]).unwrap(); // Largo 2, mensaje 1 2
        input_2.push(PACKET_SEP);

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
            assert!(!ended_c2.load(Relaxed)); // todavia no deberia figurar como ended
            before_tx.write_all(&input_2).await.unwrap();
            drop(before_tx);
            future.await;
        });

        let result = rx.iter().collect::<Vec<Vec<u8>>>();
        assert_eq!(result, vec![vec![9, 3], vec![1, 2]]);
        assert!(ended.load(Relaxed));
    }
}
