mod order_processor;
use order_processor::processor::OrderProcessor;
use std::{thread, time};
extern crate actix;
use actix::{Actor, Addr};
use order_processor::processor_messages::{PrepareOrder, AbortOrder, CommitOrder, AddMoney};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use rand::Rng;
use std::str::FromStr;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn prepare_coffee() -> bool {
    let mut rng = rand::thread_rng();
    thread::sleep(time::Duration::from_millis(rng.gen_range(0..100)));
    if rng.gen_range(0..100) < 15 {
        //TODO: log failed coffee
        return false
    }
    thread::sleep(time::Duration::from_millis(rng.gen_range(0..100)));
    //TODO: log successfull coffee
    true
}

async fn abort(order_actor: &Addr<OrderProcessor>) {
    let abort = AbortOrder {};
    let response = order_actor.send(abort).await;
    let _ = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };
}

async fn commit(order_actor: &Addr<OrderProcessor>) {
    let abort = CommitOrder {};
    let response = order_actor.send(abort).await;
    let _ = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };
}


async fn sale_order(order_data: &[&str], order_actor: &Addr<OrderProcessor>) {
    let order = PrepareOrder {
        user_id: FromStr::from_str(order_data[0]).expect("field was not u8"),
        cost: FromStr::from_str(order_data[1]).expect("field was not u8"),
    };

    println!("order: {}, {}", order.user_id, order.cost);//debug only

    let response = order_actor.send(order).await;
    let message = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };

    let result = match message {
        Ok(result) => result,
        Err(error) => panic!("message problem {:?}", error), //TODO log message error and return
    };

    if result != String::from("ready") {
        //TODO log error message with the result string
        return
    }

    if prepare_coffee() {
        commit(order_actor).await;
    } else {
        abort(order_actor).await;
    }
}

async fn recharge_order(order_data: &[&str], order_actor: &Addr<OrderProcessor>) {
    let order = AddMoney {
        user_id: FromStr::from_str(order_data[0]).expect("field was not u8"),
        amount: FromStr::from_str(order_data[1]).expect("field was not u8"),
    };

    println!("order: {}, {}", order.user_id, order.amount);//debug only
    let response = order_actor.send(order).await;
    let message = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };

    let result = match message {
        Ok(result) => result,
        Err(error) => panic!("message problem {:?}", error), //TODO log message error and return
    };

    if result != String::from("Ok") {
        //TODO log error message with the result string
        return
    }
    //TODO log okey
}


async fn process_order(order: String, order_actor: &Addr<OrderProcessor>) {
    let order_data: Vec<&str> = order.split(',').collect();

    if order_data[0] == "sale" {
        sale_order(&order_data[1..], &order_actor).await;
    } else if order_data[0] == "recharge" {
        recharge_order(&order_data[1..], &order_actor).await;
    } else {
        //TODO log error in order
    }
}

#[actix_rt::main]
async fn start_coffee_maker(path: &str, addr: &str) {
    let order_actor = OrderProcessor::new(addr, 1000).expect("couldnt initialize actor");
    let order_actor_addr = order_actor.start();
    if let Ok(lines) = read_lines(path) {
        for order in lines.flatten() {
            process_order(order, &order_actor_addr).await;
        }
    } else {
        panic!("file not found");
    }
    //TODO log finished orders
}

fn main() {
    start_coffee_maker("./orders/one_order.csv", "127.0.0.1:34255");
}


#[cfg(test)]
mod test {
    use std::net::TcpListener;
    use std::{thread, time};
    use std::io::prelude::*;
    use crate::start_coffee_maker;

    #[test]
    fn test_one_order() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34235").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();
    
            let mut buf = [0_u8; 3];
            println!("reading");
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);
    
            let response = [b'r'];
            let _ = stream.write(&response).unwrap();

            let mut buf2 = [0_u8; 1];
            println!("reading");
            let _ = stream.read(&mut buf2).unwrap();
            assert!(buf2 == [b'c'] || buf2 == [b'a']);
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34235");

        join_handle.join().unwrap();
    }

    #[test]
    fn test_repeated_order() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34234").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();
            for _ in 0..10 {
                let mut buf = [0_u8; 3];
                println!("reading");
                let _ = stream.read(&mut buf).unwrap();
                assert_eq!(buf, ['p' as u8, 3 as u8, 5 as u8]);
        
                let response = [b'r'];
                let _ = stream.write(&response).unwrap();

                let mut buf2 = [0_u8; 1];
                println!("reading");
                let _ = stream.read(&mut buf2).unwrap();
                assert!(buf2 == [b'c'] || buf2 == [b'a']);
            }
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/repeated_order.csv", "127.0.0.1:34234");

        join_handle.join().unwrap();
    }

    #[test]
    fn test_one_recharge() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34233").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();

            let mut buf = [0_u8; 3];
            println!("reading");
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 7 as u8, 100 as u8]);
    
            let response = [b'o'];
            let _ = stream.write(&response).unwrap();
        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/one_recharge.csv", "127.0.0.1:34233");

        join_handle.join().unwrap();
    }
}
