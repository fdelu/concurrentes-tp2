use std::net::SocketAddr;
use crate::packet::Packet;

pub use block_two_hosts::BlockTwoHosts;
mod block_two_hosts;

pub trait Rule {
    /// Returns true if the packet should be dropped
    /// Returning false does not guarantee that the packet will be sent,
    /// because other rules may block it
    /// Only one rule that returns true is needed to drop de packet.
    fn is_matching(&self, src: SocketAddr, dst: SocketAddr, data: &[u8]) -> bool;

    /// Notifies the rule that a packet was sent
    /// It will be triggered each time a packet is sent from the firewall
    /// to some destination.
    fn notify_send(&mut self, packet: &Packet);
}
