use super::processor_messages::{AbortOrder, AddMoney, CommitOrder, PrepareOrder};
use actix::{Actor, Context, Handler};
use std::io;
use std::io::prelude::*;
use std::net::TcpStream;
use std::time;

pub(crate) struct OrderProcessor {
    server_socket: TcpStream,
}

impl Actor for OrderProcessor {
    type Context = Context<Self>;
}

impl OrderProcessor {
    pub(crate) fn new(server_addr: &str, read_timeout: u64) -> Result<Self, std::io::Error> {
        let socket = TcpStream::connect(server_addr)?;
        socket.set_read_timeout(Some(time::Duration::from_millis(read_timeout)))?;
        Ok(Self {
            server_socket: socket,
        })
    }
}

impl Handler<PrepareOrder> for OrderProcessor {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, msg: PrepareOrder, _ctx: &mut Self::Context) -> Result<String, io::Error> {
        let _ = self.server_socket.write(&[b'p', msg.user_id, msg.cost])?;
        let mut buf = [0_u8];
        let read_res = self.server_socket.read(&mut buf);
        let bytes_read = match read_res {
            Ok(v) => v,
            Err(_) => return Ok(String::from("timeout")),
        };
        if bytes_read < 1 {
            return Ok(String::from("None"));
        } else if buf[0] == b'i' {
            return Ok(String::from("insufficient"));
        } else if buf[0] == b'a' {
            return Ok(String::from("abort"));
        } else if buf[0] == b'r' {
            return Ok(String::from("ready"));
        }
        Ok(String::from("unknown"))
    }
}

impl Handler<CommitOrder> for OrderProcessor {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, _msg: CommitOrder, _ctx: &mut Self::Context) -> Result<String, io::Error> {
        let _ = self.server_socket.write(&[b'c'])?;
        Ok(String::from("sent"))
    }
}

impl Handler<AbortOrder> for OrderProcessor {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, _msg: AbortOrder, _ctx: &mut Self::Context) -> Result<String, io::Error> {
        let _ = self.server_socket.write(&[b'a'])?;
        Ok(String::from("sent"))
    }
}

impl Handler<AddMoney> for OrderProcessor {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, msg: AddMoney, _ctx: &mut Self::Context) -> Result<String, io::Error> {
        let _ = self.server_socket.write(&[b'm', msg.user_id, msg.amount])?;
        let mut buf = [0_u8];
        let read_res = self.server_socket.read(&mut buf);
        let bytes_read = match read_res {
            Ok(v) => v,
            Err(_) => return Ok(String::from("timeout")),
        };
        if bytes_read < 1 {
            return Ok(String::from("None"));
        } else if buf[0] == b'o' {
            return Ok(String::from("Ok"));
        }
        Ok(String::from("unknown"))
    }
}

#[cfg(test)]
mod test {

    use crate::order_processor::processor::OrderProcessor;
    use std::io::prelude::*;
    use std::net::TcpListener;
    use std::{thread, time};
    extern crate actix;
    use crate::order_processor::processor_messages::{
        AbortOrder, AddMoney, CommitOrder, PrepareOrder,
    };
    use actix::Actor;

    #[actix_rt::test]
    async fn test_prepare_ready() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34244").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['r' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34244", 10000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("ready"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_timeout() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34254").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(1500));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['r' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(1000));

        let a = OrderProcessor::new("127.0.0.1:34254", 100).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("timeout"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_insufficient() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34243").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['i' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34243", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("insufficient"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_abort() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34242").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['a' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34242", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("abort"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_prepare_unknown() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34241").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);

            let mut response = ['c' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34241", 1000).unwrap();
        let a_addr = a.start();

        let order = PrepareOrder {
            user_id: 3,
            cost: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("unknown"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_commit_sent() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34240").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['c' as u8]);
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34240", 1000).unwrap();
        let a_addr = a.start();

        let order = CommitOrder {};

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("sent"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_abort_sent() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34239").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['a' as u8]);
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34239", 1000).unwrap();
        let a_addr = a.start();

        let order = AbortOrder {};

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("sent"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_ok() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34238").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);

            let mut response = ['o' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34238", 10000).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("Ok"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_unknown() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34237").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(150));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);

            let mut response = ['t' as u8];
            stream.write(&mut response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));

        let a = OrderProcessor::new("127.0.0.1:34237", 10000).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("unknown"));
        join_handle.join().unwrap();
    }

    #[actix_rt::test]
    async fn test_add_money_timeout() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34236").unwrap();

            let (mut stream, _) = listener.accept().unwrap();

            thread::sleep(time::Duration::from_millis(1500));
            let mut buf = [0 as u8; 3];
            stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 3 as u8, 5 as u8]);
        });

        thread::sleep(time::Duration::from_millis(1000));

        let a = OrderProcessor::new("127.0.0.1:34236", 100).unwrap();
        let a_addr = a.start();

        let order = AddMoney {
            user_id: 3,
            amount: 5,
        };

        let res = a_addr.send(order).await.unwrap().unwrap();
        assert_eq!(res, String::from("timeout"));
        join_handle.join().unwrap();
    }
}
