mod order_processor;
use order_processor::processor::OrderProcessor;
use std::io::prelude::*;
use std::net::TcpListener;
use std::{thread, time};
extern crate actix;
use actix::Actor;
use order_processor::processor_messages::PrepareOrder;

#[actix_rt::main]
async fn main() {
    println!("Hello, world!");

    let join_handle = thread::spawn(|| {
        let listener = TcpListener::bind("127.0.0.1:34254").unwrap();
        println!("listener ready");
        let (mut stream, _) = listener.accept().unwrap();

        let mut buf = [0_u8; 3];
        println!("reading");
        let _ = stream.read(&mut buf).unwrap();
        println!("Hello, world!{}, {}, {}", buf[0] as char, buf[1], buf[2]);

        let response = [b'r'];
        println!("reading");
        let _ = stream.write(&response).unwrap();
    });

    println!("waiting main");
    thread::sleep(time::Duration::from_millis(1000));
    println!("finished waiting main");
    let a = OrderProcessor::new("127.0.0.1:34254", 1000).unwrap();
    println!("created order processor");
    let a_addr = a.start();
    println!("started actor order processor");

    let order = PrepareOrder {
        user_id: 3,
        cost: 5,
    };

    thread::sleep(time::Duration::from_millis(1000));
    println!("sending");
    let res = a_addr.send(order).await;
    let resres = res.unwrap();
    println!("finished sending, {}", resres.unwrap());
    join_handle.join().unwrap();
}
