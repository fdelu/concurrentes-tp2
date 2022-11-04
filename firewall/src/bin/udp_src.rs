use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;

use firewall::{SocketEncoder, UdpTrait};

const SRC_ADDR: &str = "127.0.0.1:1234";
const DST_ADDR: &str = "127.0.0.2:5555";

fn main() {
    let addrs = [
        SocketAddr::from_str(SRC_ADDR).unwrap()
    ];

    let socket: SocketEncoder<UdpSocket> = SocketEncoder::bind(&addrs[..]).unwrap();

    let buf = b"Hello World";
    let dst = SocketAddr::from_str(DST_ADDR).unwrap();
    socket.send_to(buf, dst).unwrap();
}