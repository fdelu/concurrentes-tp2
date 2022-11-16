use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use actix::prelude::*;

use common::AHandler;

use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::packets::{get_timestamp, MutexPacket, ResourceId, Timestamp};
use crate::dist_mutex::server_id::ServerId;
use crate::dist_mutex::{DistMutex, MutexCreationTrait};
use crate::network::messages::ReceivedPacket;
use crate::network::{ConnectionHandler, SendPacket};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::{Packet, SyncRequestPacket, SyncResponsePacket};

pub mod messages;
pub mod packet;

pub trait TCPActorTrait: AHandler<SendPacket> {}

impl<A: AHandler<ReceivedPacket>> TCPActorTrait for ConnectionHandler<A> {}

pub trait PacketDispatcherTrait:
    AHandler<ReceivedPacket>
    + AHandler<BroadcastMessage>
    + AHandler<PruneMessage>
    + AHandler<SendMessage>
{
}

impl PacketDispatcherTrait for PacketDispatcher {}

pub(crate) const SERVERS: [ServerId; 3] =
    [ServerId { id: 0 }, ServerId { id: 1 }, ServerId { id: 2 }];

pub trait TCPActorCreationTrait<P: PacketDispatcherTrait> {
    fn new(receiver_handler: Addr<P>) -> Self
    where
        Self: TCPActorTrait;
}

pub struct PacketDispatcher {
    server_id: ServerId,
    mutexes: HashMap<ResourceId, Addr<DistMutex<Self>>>,
    socket: Addr<ConnectionHandler<Self>>,
    servers_last_seen: HashMap<ServerId, Option<Timestamp>>,
    // TODO: Replace with a proper data structure containing
    // TODO: every user and their amount of points
    data: Vec<u32>,
}

impl Actor for PacketDispatcher {
    type Context = Context<Self>;
}

impl PacketDispatcher {
    pub fn new(my_id: ServerId) -> Addr<Self> {
        let servers_last_seen = SERVERS
            .iter()
            .filter(|&&server_id| server_id != my_id)
            .map(|&server_id| (server_id, None))
            .collect();

        Self::create(|ctx| {
            let socket = ConnectionHandler::new(ctx.address(), SocketAddr::from(my_id)).start();
            let mut ret = Self {
                server_id: my_id,
                mutexes: HashMap::new(),
                socket,
                servers_last_seen,
                data: vec![1, 2, 3, 4, 5, 6],
            };
            ret.send_sync_request(ctx);
            ret
        })
    }

    fn send_sync_request(&mut self, ctx: &mut Context<Self>) {
        let packet = SyncRequestPacket {
            timestamp: get_timestamp(),
        };
        let my_addr = ctx.address();
        self.servers_last_seen.iter().for_each(|(&server_id, _)| {
            my_addr.do_send(SendMessage {
                to: server_id,
                packet: Packet::SyncRequest(packet),
            });
        });
    }

    fn handle_mutex(&mut self, from: ServerId, packet: MutexPacket, ctx: &mut Context<Self>) {
        match packet {
            MutexPacket::Request(request) => {
                println!("Received request from {}", from);
                let mutex = self.get_or_create_mutex(ctx, request.id);
                let message = RequestMessage::new(from, request);

                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ack(ack) => {
                let mutex = self.get_or_create_mutex(ctx, ack.id);
                let message = AckMessage::new(from, ack);
                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ok(ok) => {
                let connected_servers = self.get_connected_servers();
                let mutex = self.get_or_create_mutex(ctx, ok.id);

                let message = OkMessage::new(from, connected_servers, ok);
                mutex.try_send(message).unwrap();
            }
        }
    }

    fn handle_sync_request(
        &mut self,
        from: ServerId,
        _packet: SyncRequestPacket,
        ctx: &mut Context<Self>,
    ) {
        let packet = SyncResponsePacket {
            data: self.data.clone(),
        };
        ctx.address()
            .try_send(SendMessage {
                to: from,
                packet: Packet::SyncResponse(packet),
            })
            .unwrap();
    }

    fn handle_sync_response(
        &mut self,
        _from: ServerId,
        packet: SyncResponsePacket,
        _ctx: &mut Context<Self>,
    ) {
        // TODO: wait for all servers to respond and then update data based on the majority
        self.data = packet.data;
    }

    fn get_or_create_mutex(
        &mut self,
        ctx: &mut Context<PacketDispatcher>,
        id: ResourceId,
    ) -> &mut Addr<DistMutex<PacketDispatcher>> {
        let mutex = self.mutexes.entry(id).or_insert_with(|| {
            println!("Creating mutex for {}", id);
            DistMutex::new(self.server_id, id, ctx.address()).start()
        });
        mutex
    }

    fn get_connected_servers(&self) -> HashSet<ServerId> {
        self.servers_last_seen
            .iter()
            .filter(|(_, last_seen)| last_seen.is_some())
            .map(|(server_id, _)| *server_id)
            .collect()
    }

    fn send_data(
        &mut self,
        to: ServerId,
        data: Vec<u8>,
    ) -> Request<ConnectionHandler<PacketDispatcher>, SendPacket> {
        self.socket.send(SendPacket {
            to: to.into(),
            data,
        })
    }
}
