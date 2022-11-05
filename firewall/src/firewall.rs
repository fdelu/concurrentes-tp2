use std::net::SocketAddr;
use std::str::FromStr;

use crate::packet::Packet;
use crate::rules::Rule;
use common::udp_trait::{UDPFactory, UdpTrait};

pub const FIREWALL_ADDRESS: &str = "127.0.0.1:1883";
pub const ADDR_LENGTH: usize = 14;

pub struct Firewall {
    rules: Vec<Box<dyn Rule>>,
    socket: Box<dyn UdpTrait>,
}

// Firewall will be waiting for packets and will check if they are allowed by the rules
// If they are allowed, it will send them to the corresponding address
// If they are not allowed, it will drop them
// Rules can be added dynamically, and their state may change when packets are sent
// There is only one firewall running in the entire system, in a separate process
// The servers will send packets to the firewall, then the firewall will forward them to the
// address specified in the metadata of the packet
impl Firewall {
    pub fn new<T>(socket_factory: T) -> Self
    where
        T: UDPFactory,
    {
        let addrs =
            [SocketAddr::from_str(FIREWALL_ADDRESS).expect("Failed to parse firewall address")];

        let socket = socket_factory.create_udp_socket(&addrs[..]);

        Firewall {
            rules: Vec::new(),
            socket,
        }
    }

    pub fn add_rule<R: Rule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    pub fn run(&mut self) {
        loop {
            let packet = self.read_packet();
            println!("Received packet: {}", packet);
            if self.is_allowed(&packet) {
                println!("Packet is allowed");
                self.send_packet(&packet);
            } else {
                println!("Packet is not allowed");
            }
        }
    }

    fn is_allowed(&self, packet: &Packet) -> bool {
        for rule in &self.rules {
            if rule.is_matching(packet.src, packet.dst, &packet.data) {
                return false;
            }
        }
        true
    }

    fn send_packet(&mut self, packet: &Packet) {
        self.socket
            .send_to(
                &packet.data,
                SocketAddr::from_str(&packet.dst.to_string()).unwrap(),
            )
            .unwrap();
        for rule in &mut self.rules {
            rule.notify_send(packet);
        }
    }

    fn read_packet(&mut self) -> Packet {
        let mut buf = [0; 1024];
        let (size, src) = self.socket.recv_from(&mut buf).unwrap();
        Packet::from_raw(src, &buf[..size])
    }
}
