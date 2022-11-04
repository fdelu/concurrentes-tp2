use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;
use firewall::{SocketEncoder, UdpTrait};

const DST_ADDR: &str = "127.0.0.2:5555";

fn main() {
    let addrs = [
        SocketAddr::from_str(DST_ADDR).unwrap()
    ];

    let socket: SocketEncoder<UdpSocket> = SocketEncoder::bind(&addrs[..]).unwrap();

    let mut buf = [0; 1024];
    let (size, src) = socket.recv_from(&mut buf).unwrap();
    println!("Received {} bytes from {}", size, src);
    println!("Received: {}", String::from_utf8_lossy(&buf[..size]));
}