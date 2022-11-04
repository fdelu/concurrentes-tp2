use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

pub trait UDPFactory {
    fn create_udp_socket<T: ToSocketAddrs>(&self, addr: T) -> Box<dyn UdpTrait>;
}

pub trait UdpTrait {
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize>;

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)>;

    fn bind<U: ToSocketAddrs>(addr: U) -> std::io::Result<Self> where Self: Sized;

    fn local_addr(&self) -> std::io::Result<SocketAddr>;
}

impl UdpTrait for UdpSocket {
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.send_to(buf, addr)
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.recv_from(buf)
    }

    fn bind<U: ToSocketAddrs>(addr: U) -> std::io::Result<Self> where Self: Sized {
        UdpSocket::bind(addr)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.local_addr()
    }
}