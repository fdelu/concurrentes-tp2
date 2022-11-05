use crate::firewall::FIREWALL_ADDRESS;
use std::net::{SocketAddr, ToSocketAddrs};

use crate::packet::Packet;
use common::udp_trait::UdpTrait;

pub struct SocketEncoder<T: UdpTrait> {
    socket: T,
}

// ProxySocket will send bytes encoded by Packet to the firewall, and receive bytes from the firewall
// which are not encoded.
impl<T: UdpTrait> UdpTrait for SocketEncoder<T> {
    fn send_to(&self, buf: &[u8], dst: SocketAddr) -> std::io::Result<usize> {
        let src = self.socket.local_addr()?;
        let packet = Packet::new(src, dst, buf.to_vec());
        self.socket.send_to(
            &packet.to_raw(),
            FIREWALL_ADDRESS
                .parse()
                .expect("Failed to parse firewall address"),
        )
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf)
    }

    fn bind<U: ToSocketAddrs>(addr: U) -> std::io::Result<Self> {
        let socket = T::bind(addr)?;
        Ok(SocketEncoder { socket })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }
}
