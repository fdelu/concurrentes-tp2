use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use super::{OnRead, SocketError, PACKET_SEP};

pub struct ReaderLoop<T: AsyncReadExt + Unpin, P: DeserializeOwned> {
    reader: BufReader<T>,
    on_read: OnRead<P>,
}

impl<T: AsyncReadExt + Unpin, P: DeserializeOwned> ReaderLoop<T, P> {
    pub fn new(reader: T, on_read: OnRead<P>) -> Self {
        Self {
            reader: BufReader::new(reader),
            on_read,
        }
    }

    // Returns message size or 0 if EOF reached
    async fn process_message(&mut self, buffer: &mut Vec<u8>) -> Result<usize, SocketError> {
        buffer.clear();
        let size = self.reader.read_until(PACKET_SEP, buffer).await?;

        if size > 0 {
            (self.on_read)(serde_json::from_slice(buffer)?);
        }

        Ok(size)
    }

    pub async fn run(mut self) -> Result<(), SocketError> {
        let mut buffer = vec![];
        loop {
            match self.process_message(&mut buffer).await {
                Ok(0) => return Ok(()),
                Err(e) => {
                    eprintln!("Error in ReaderLoop: {}", e);
                    return Err(e);
                }
                _ => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use tokio_test::{block_on, io::Builder};

    type DataReadFromInput<P> = Arc<Mutex<Vec<P>>>;
    type ReaderResult = Result<(), SocketError>;

    fn setup<P: DeserializeOwned + Send + 'static>(
        input: Vec<Vec<u8>>,
    ) -> (DataReadFromInput<P>, ReaderResult) {
        let mut builder = Builder::new();
        for i in input {
            builder.read(&i);
        }
        let mock_reader = builder.build();

        let read = Arc::new(Mutex::new(vec![]));
        let read_c = read.clone();
        let on_read = Box::new(move |data: P| read_c.lock().unwrap().push(data));

        let socket_read = ReaderLoop::new(mock_reader, on_read);
        (read, block_on(socket_read.run()))
    }

    #[test]
    fn test_read_basic() {
        let mut input = vec![1, 2, 3, 4]; // Largo 4, mensaje 1 2 3 4
        input = serde_json::to_vec(&input).unwrap();
        input.push(PACKET_SEP);
        let (read, result) = setup::<Vec<u8>>(vec![input]);
        result.unwrap();

        assert_eq!(*read.lock().unwrap(), vec![vec![1, 2, 3, 4]]);
    }

    #[test]
    fn test_no_new_line() {
        let mut input = vec![1, 2, 3, 4]; // Largo 4, mensaje 1 2 3 4
        input = serde_json::to_vec(&input).unwrap();
        input.extend(serde_json::to_vec(&input).unwrap());

        let (read, result) = setup::<Vec<u8>>(vec![input]);
        result.unwrap_err();

        assert!(read.lock().unwrap().is_empty());
    }

    #[test]
    fn test_read_twice() {
        let mut input = vec![9, 3]; // Largo 2, mensaje 9 3
        input = serde_json::to_vec(&input).unwrap();
        input.push(PACKET_SEP);
        let mut input_2 = vec![1, 2]; // Largo 2, mensaje 1 2
        input_2 = serde_json::to_vec(&input_2).unwrap();
        input_2.push(PACKET_SEP);

        let (read, result) = setup::<Vec<u8>>(vec![input, input_2]);
        result.unwrap();

        assert_eq!(*read.lock().unwrap(), vec![vec![9, 3], vec![1, 2]]);
    }

    #[test]
    fn test_read_twice_mixed_reads() {
        let mut inputs = vec![];
        inputs.push(serde_json::to_vec(&vec![3]).unwrap()); // [3]
        inputs.push(vec![PACKET_SEP]);
        let mut input_3 = serde_json::to_vec(&vec![4, 5, 4]).unwrap(); // [4, 5, 4]
        input_3.push(PACKET_SEP);
        inputs.push(input_3);

        let (read, result) = setup::<Vec<u8>>(inputs);
        result.unwrap();

        assert_eq!(*read.lock().unwrap(), vec![vec![3], vec![4, 5, 4]]);
    }
}
