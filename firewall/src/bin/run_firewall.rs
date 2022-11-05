use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::FromStr;

use common::udp_trait::{UDPFactory, UdpTrait};
use firewall::BlockTwoHosts;
use firewall::Firewall;

const SRC_ADDR: &str = "127.0.0.1:1234";
const DST_ADDR: &str = "127.0.0.2:5555";

struct MyUDPFactory;

impl UDPFactory for MyUDPFactory {
    fn create_udp_socket<T: ToSocketAddrs>(&self, addr: T) -> Box<dyn UdpTrait> {
        Box::new(UdpSocket::bind(addr).unwrap())
    }
}

/// Run a firewall that blocks all traffic between the hosts created with udp_src.rs and udp_dst.rs.
/// This is a simple example of how to create a firewall and add rules to it.
fn main() {
    let mut firewall = Firewall::new(MyUDPFactory);
    let src_addr = SocketAddr::from_str(SRC_ADDR).unwrap();
    let dst_addr = SocketAddr::from_str(DST_ADDR).unwrap();
    let block_rule = BlockTwoHosts::new(src_addr, dst_addr);
    firewall.add_rule(block_rule);

    firewall.run();
}
