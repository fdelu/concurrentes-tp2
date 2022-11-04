use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;
use crate::firewall::ADDR_LENGTH;

#[derive(Debug)]
pub struct Packet{
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub data: Vec<u8>
}

impl Packet {
    pub fn new(src: SocketAddr, dst: SocketAddr, data: Vec<u8>) -> Self {
        Packet {
            src,
            dst,
            data
        }
    }

    pub fn from_raw(src: SocketAddr, buf: &[u8]) -> Self {
        let mut addr = [0; ADDR_LENGTH];
        addr.copy_from_slice(&buf[..ADDR_LENGTH]);
        let dst = SocketAddr::from_str(&String::from_utf8(addr.to_vec()).unwrap()).unwrap();
        let data = buf[ADDR_LENGTH..].to_vec();
        Packet::new(src, dst, data)
    }

    pub fn to_raw(&self) -> Vec<u8> {
        let mut bytes = vec![0; self.data.len() + ADDR_LENGTH];
        bytes[..ADDR_LENGTH].copy_from_slice(self.dst.to_string().as_bytes());
        bytes[ADDR_LENGTH..].copy_from_slice(&self.data);
        bytes
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Packet {{ src: {}, dst: {}, data: {:?} }}", self.src, self.dst, self.data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn build_socket_addr(port: u16) -> SocketAddr {
        SocketAddr::from_str("127.0.0.1").expect("Failed to parse socket address")
    }

    #[test]
    fn test_packet_is_correctly_created() {
        let src = build_socket_addr(1234);
        let dst = build_socket_addr(5678);
        let data = vec![1, 2, 3, 4];

        let packet = Packet::new(src, dst, data.clone());

        assert_eq!(packet.src, src);
        assert_eq!(packet.dst, dst);
        assert_eq!(packet.data, data);
    }

    #[test]
    fn test_packet_is_correctly_converted_to_raw() {
        let src = build_socket_addr(1234);
        let dst = build_socket_addr(5678);
        let data = vec![1, 2, 3, 4];
        let packet = Packet::new(src, dst, data.clone());

        let raw = packet.to_raw();

        assert_eq!(raw.len(), ADDR_LENGTH + data.len());
        assert_eq!(&raw[..ADDR_LENGTH], dst.to_string().as_bytes());
        assert_eq!(raw[ADDR_LENGTH..], data);
    }

    #[test]
    fn test_packet_is_correctly_created_from_raw() {
        let src = build_socket_addr(1234);
        let dst = build_socket_addr(5678);
        let data = vec![1, 2, 3, 4];
        let raw = Packet::new(src, dst, data.clone()).to_raw();

        let packet = Packet::from_raw(src, &raw);

        assert_eq!(packet.src, src);
        assert_eq!(packet.dst, dst);
        assert_eq!(packet.data, data);
    }
}