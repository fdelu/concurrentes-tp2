use crate::order_processor::processor::OrderProcessor;
use std::{thread, time};
extern crate actix;
use actix::{Actor, Addr};
use crate::order_processor::processor_messages::{PrepareOrder, AbortOrder, CommitOrder, AddMoney};
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

///prepares coffee, the time it takes is random and theres a chance to fail.
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

///Sends the server an abort message.
async fn abort(order_actor: &Addr<OrderProcessor>) {
    let abort = AbortOrder {};
    let response = order_actor.send(abort).await;
    let _ = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };
}

///Sends the server a commit message.
async fn commit(order_actor: &Addr<OrderProcessor>) {
    let abort = CommitOrder {};
    let response = order_actor.send(abort).await;
    let _ = match response {
        Ok(message) => message,
        Err(error) => panic!("actor problem {:?}", error), //TODO log actor error and return
    };
}

///Sends the order to buy coffee to the server.
/// If its oked starts preparing the coffee.
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

///Sends the server a message to add money in an users account.
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

///checks the type of order
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

///loops through the order file and prepares the orders
#[actix_rt::main]
pub async fn start_coffee_maker(path: &str, addr: &str) {
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



#[cfg(test)]
mod test {
    use std::net::TcpListener;
    use std::{thread, time};
    use std::io::prelude::*;
    use std::net::TcpStream;
    use crate::start_coffee_maker;
    // use futures::join;
    
    fn assert_order_ready(stream: &mut TcpStream, id: u8, cost: u8) {
        let mut buf = [0_u8; 3];
        println!("reading");
        let _ = stream.read(&mut buf).unwrap();
        assert_eq!(buf, ['p' as u8, id, cost]);

        let response = [b'r'];
        let _ = stream.write(&response).unwrap();

        let mut buf2 = [0_u8; 1];
        println!("reading");
        let _ = stream.read(&mut buf2).unwrap();
        assert!(buf2 == [b'c'] || buf2 == [b'a']);
    }

    #[test]
    fn test_one_order() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34235").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();
    
            assert_order_ready(&mut stream, 3 as u8, 5 as u8);
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
                assert_order_ready(&mut stream, 3 as u8, 5 as u8);
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

    #[test]
    fn test_diff_orders() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34232").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();

            assert_order_ready(&mut stream, 3 as u8, 5 as u8);


            let mut buf = [0_u8; 3];
            println!("reading");
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 7 as u8, 100 as u8]);
    
            let response = [b'o'];
            let _ = stream.write(&response).unwrap();

            assert_order_ready(&mut stream, 100 as u8, 3 as u8);

            assert_order_ready(&mut stream, 5 as u8, 12 as u8);

            let mut buf = [0_u8; 3];
            println!("reading");
            let _ = stream.read(&mut buf).unwrap();
            assert_eq!(buf, ['m' as u8, 12 as u8, 20 as u8]);
    
            let response = [b'o'];
            let _ = stream.write(&response).unwrap();

        });

        thread::sleep(time::Duration::from_millis(100));
        start_coffee_maker("./src/orders/diff_orders.csv", "127.0.0.1:34232");

        join_handle.join().unwrap();
    }

    async fn assert_order_ready_async(stream: &mut TcpStream, id: u8, cost: u8, debug: i32) {
        if debug == 1 {
            tokio::time::sleep(time::Duration::from_millis(100)).await;
        }
        let mut buf = [0_u8; 3];
        println!("reading in {}", debug);
        let _ = stream.read(&mut buf).unwrap();
        assert_eq!(buf, ['p' as u8, id, cost]);

        let response = [b'r'];
        let _ = stream.write(&response).unwrap();

        let mut buf2 = [0_u8; 1];
        println!("reading second in {}", debug);
        let _ = stream.read(&mut buf2).unwrap();
        assert!(buf2 == [b'c'] || buf2 == [b'a']);
    }

    #[tokio::main]
    async fn split_processing(mut stream: &mut TcpStream,mut stream2: &mut TcpStream) {
        let fut1 = assert_order_ready_async(&mut stream, 3 as u8, 5 as u8, 1);
        let fut2 = assert_order_ready_async(&mut stream2, 3 as u8, 5 as u8, 2);
        tokio::join!(fut1, fut2);
    }

    #[test]
    fn test_two_coffee_makers() {
        let join_handle = thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34231").unwrap();
            println!("listener ready");
            let (mut stream, _) = listener.accept().unwrap();
            let (mut stream2, _) = listener.accept().unwrap();
            split_processing(&mut stream, &mut stream2);
        });

        thread::sleep(time::Duration::from_millis(100));
        let join_handle2 = thread::spawn(|| {
            start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34231");
        });
        start_coffee_maker("./src/orders/one_order.csv", "127.0.0.1:34231");

        join_handle.join().unwrap();
        join_handle2.join().unwrap();
    }
}