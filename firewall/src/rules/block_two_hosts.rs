use std::net::SocketAddr;
use crate::packet::Packet;
use crate::rules::Rule;

// Simple example of a rule that blocks communication between
// two hosts
#[derive(Debug)]
pub struct BlockTwoHosts {
    pub src_to_block: SocketAddr,
    pub dst_to_block: SocketAddr,
}

impl Rule for BlockTwoHosts {
    fn is_matching(&self, src: SocketAddr, dst: SocketAddr, _: &[u8]) -> bool {
        src == self.src_to_block && dst == self.dst_to_block
    }

    fn notify_send(&mut self, _packet: &Packet) {}
}

impl BlockTwoHosts {
    pub fn new(src_to_block: SocketAddr, dst_to_block: SocketAddr) -> Self {
        BlockTwoHosts {
            src_to_block,
            dst_to_block,
        }
    }
}