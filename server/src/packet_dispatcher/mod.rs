use std::collections::HashMap;

use actix::prelude::*;

use common::AHandler;

use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::public::release::ReleaseMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::packets::MutexPacket;
use crate::dist_mutex::{DistMutexTrait, MutexCreationTrait, ResourceId, ServerId};
use crate::network::{ConnectionHandler, ReceivedPacket, SendPacket};
use crate::packet_dispatcher::messages::send_from_mutex::SendFromMutexMessage;

pub mod messages;
pub mod packet;


pub trait TCPActorTrait: AHandler<SendPacket> {}

impl<A: AHandler<ReceivedPacket>> TCPActorTrait for ConnectionHandler<A> {}

pub trait PacketDispatcherTrait:
    AHandler<ReceivedPacket> + AHandler<SendFromMutexMessage>
{
}

impl<D: DistMutexTrait + MutexCreationTrait<Self>, T: TCPActorTrait> PacketDispatcherTrait for PacketDispatcher<D, T> {}

pub(crate) struct PacketDispatcher<D: DistMutexTrait, T: TCPActorTrait> {
    mutexes: HashMap<ResourceId, Addr<D>>,
    socket: Addr<T>,
}

impl<D: DistMutexTrait, T: TCPActorTrait> Actor for PacketDispatcher<D, T> {
    type Context = Context<Self>;
}

impl<D: DistMutexTrait + MutexCreationTrait<Self>, T: TCPActorTrait> PacketDispatcher<D, T> {
    pub fn new(socket: Addr<T>) -> Self {
        Self {
            mutexes: HashMap::new(),
            socket,
        }
    }

    fn handle_mutex(&mut self, from: ServerId, packet: MutexPacket, ctx: &mut Context<Self>) {
        match packet {
            MutexPacket::Request(request) => {
                let mutex = self.mutexes.entry(request.id()).or_insert_with(|| {
                    D::new(from, request.id(), ctx.address()).start()
                });
                let message = RequestMessage::new(from, request);

                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ack(ack) => {
                let mutex = self.mutexes.get_mut(&ack.id()).unwrap();
                let message = AckMessage::new(from, ack);
                mutex.try_send(message).unwrap();
            }
            MutexPacket::Ok(ok) => {
                let mutex = self.mutexes.get_mut(&ok.id()).unwrap();
                let message = OkMessage::new(from, ok);
                mutex.try_send(message).unwrap();
            }
            MutexPacket::Release(release) => {
                let mutex = self.mutexes.get_mut(&release.id()).unwrap();
                let message = ReleaseMessage::new(release);
                mutex.try_send(message).unwrap();
            }
        }
    }
}
