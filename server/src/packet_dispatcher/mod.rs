use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use actix::prelude::*;
use async_trait;
use tokio::task;

use common::AHandler;

use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::packets::{MutexPacket, RequestPacket};
use crate::dist_mutex::{DistMutex, DistMutexTrait, MutexCreationTrait, ResourceId, ServerId};
use crate::network::error::SocketError;
use crate::network::{ConnectionHandler, ReceivedPacket, SendPacket};
use crate::packet_dispatcher::messages::send_from_mutex::SendFromMutexMessage;

pub mod messages;
pub mod packet;

pub trait TCPActorTrait: AHandler<SendPacket> {}

impl<A: AHandler<ReceivedPacket>> TCPActorTrait for ConnectionHandler<A> {}

pub trait PacketDispatcherTrait: AHandler<ReceivedPacket> + AHandler<SendFromMutexMessage> {}

impl PacketDispatcherTrait for PacketDispatcher {}

pub trait TCPActorCreationTrait<P: PacketDispatcherTrait> {
    fn new(receiver_handler: Addr<P>) -> Self
    where
        Self: TCPActorTrait;
}

pub struct PacketDispatcher {
    mutexes: HashMap<ResourceId, Addr<DistMutex<Self>>>,
    socket: Addr<ConnectionHandler<Self>>,
    connected_servers: HashSet<ServerId>,
}

impl Actor for PacketDispatcher {
    type Context = Context<Self>;
}

impl PacketDispatcher {
    pub fn new(servers: HashSet<ServerId>) -> Addr<Self> {
        Self::create(|ctx| {
            let socket = ConnectionHandler::new(ctx.address()).start();
            Self {
                mutexes: HashMap::new(),
                socket,
                connected_servers: servers,
            }
        })
    }

    fn handle_mutex(&mut self, from: ServerId, packet: MutexPacket, ctx: &mut Context<Self>) {
        match packet {
            MutexPacket::Request(request) => {
                println!("Received request from {:?}", from);
                println!("{:?}", request);
                let mutex = self.get_or_create_mutex(ctx, request.id());
                let message = RequestMessage::new(from, request);

                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ack(ack) => {
                let mutex = self.get_or_create_mutex(ctx, ack.id());
                let message = AckMessage::new(from, ack);
                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ok(ok) => {
                let mutex = self.get_or_create_mutex(ctx, ok.id());
                let message = OkMessage::new(from, ok);
                mutex.try_send(message).unwrap();
            }
            MutexPacket::Release(release) => {
                let mutex = self.get_or_create_mutex(ctx, release.id());
                let message = ReleaseMessage::new(release);
                mutex.try_send(message).unwrap();
            }
        }
    }

    fn get_or_create_mutex(
        &mut self,
        ctx: &mut Context<PacketDispatcher>,
        id: ResourceId,
    ) -> &mut Addr<DistMutex<PacketDispatcher>> {
        let mutex = self.mutexes.entry(id).or_insert_with(|| {
            println!("Creating mutex for {}", id);
            DistMutex::new(id, ctx.address()).start()
        });
        mutex
    }
}
