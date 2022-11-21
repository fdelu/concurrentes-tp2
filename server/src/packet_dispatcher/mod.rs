use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::net::SocketAddr;
use std::time::Duration;

use common::packet::{CoffeeMakerId, TxId};
use tracing::{debug, info, trace};

use crate::config::Config;
use crate::dist_mutex::messages::AckMessage;
use crate::dist_mutex::messages::OkMessage;
use crate::dist_mutex::messages::RequestMessage;
use crate::dist_mutex::packets::{get_timestamp, MutexPacket, ResourceId, Timestamp};
use crate::dist_mutex::server_id::ServerId;
use crate::dist_mutex::{DistMutex, MutexCreationTrait};
use crate::network::{ConnectionHandler, SendPacket};
use crate::packet_dispatcher::packet::{Packet, SyncRequestPacket};
use crate::two_phase_commit::packets::TPCommitPacket;
use crate::two_phase_commit::{make_initial_database, TwoPhaseCommit};
use messages::DieMessage;
use messages::QueuePointsMessage;
use messages::SendMessage;
use messages::TryAddPointsMessage;

pub mod messages;
pub mod messages_impls;
pub mod packet;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(120);
const ADD_POINTS_ATTEMPT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TransactionId {
    Discount(ServerId, CoffeeMakerId, TxId),
    Add(ServerId, u32),
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionId::Discount(server_id, client_id, tx_id) => {
                write!(f, "Discount({}, {}, {})", server_id, client_id, tx_id)
            }
            TransactionId::Add(server_id, id) => {
                write!(f, "Add({}, {})", server_id, id)
            }
        }
    }
}

pub struct PacketDispatcher {
    server_id: ServerId,
    mutexes: HashMap<ResourceId, Addr<DistMutex<Self>>>,
    socket: Addr<ConnectionHandler<Packet, Packet>>,
    servers_last_seen: HashMap<ServerId, Option<Timestamp>>,
    two_phase_commit: Addr<TwoPhaseCommit<Self>>,
    points_queue: Vec<QueuePointsMessage>,
    points_ids_counter: u32,
    config: Config,
}

impl Actor for PacketDispatcher {
    type Context = Context<Self>;
}

impl Supervised for PacketDispatcher {
    fn restarting(&mut self, ctx: &mut Self::Context) {
        info!("Restarting PacketDispatcher");
        self.mutexes.values().for_each(|mutex| {
            mutex.do_send(DieMessage);
        });
        self.initialize_add_points_loop(ctx);
    }
}

impl PacketDispatcher {
    pub fn new(cfg: &Config) -> Addr<Self> {
        Self::create(|ctx| Self::new_with_context(cfg, ctx))
    }

    pub fn new_with_context(cfg: &Config, ctx: &mut Context<Self>) -> Self {
        let my_addr = SocketAddr::new(cfg.server_ip, cfg.server_port);
        let my_id = ServerId::new(cfg.server_ip);
        let servers_last_seen = cfg
            .servers
            .iter()
            .filter(|&&server_id| server_id != my_id)
            .map(|&server_id| (server_id, None))
            .collect();
        trace!("Initial servers: {:?}", servers_last_seen);

        let socket = ConnectionHandler::new(
            ctx.address().recipient(),
            my_addr,
            true,
            Some(CONNECTION_TIMEOUT),
        )
        .start();
        let two_phase_commit = TwoPhaseCommit::new(ctx.address());
        let mutexes = make_initial_database()
            .iter()
            .map(|(&client_id, _)| {
                let my_id_c = my_id;
                let client_id_c = client_id;
                let dispatcher_addr = ctx.address();
                let mutex = Supervisor::start(move |_| {
                    DistMutex::new(my_id_c, client_id_c, dispatcher_addr)
                });
                (client_id, mutex)
            })
            .collect();

        let mut ret = Self {
            server_id: my_id,
            mutexes,
            socket,
            servers_last_seen,
            two_phase_commit,
            points_queue: Vec::new(),
            points_ids_counter: 0,
            config: cfg.clone(),
        };
        ret.initialize_add_points_loop(ctx);
        ret.send_sync_request(ctx);
        ret
    }

    fn initialize_add_points_loop(&mut self, ctx: &mut Context<Self>) {
        ctx.notify_later(TryAddPointsMessage, ADD_POINTS_ATTEMPT_INTERVAL);
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
                debug!("Received request from {}", from);
                let mutex = self.get_or_create_mutex(ctx, request.id);
                let message = RequestMessage::new(from, request);
                mutex.do_send(message);
            }
            MutexPacket::Ack(ack) => {
                debug!("Received ack from {}", from);
                let mutex = self.get_or_create_mutex(ctx, ack.id);
                mutex.do_send(AckMessage { from });
            }
            MutexPacket::Ok(ok) => {
                debug!("Received ok from {}", from);
                let connected_servers = self.get_connected_servers();
                let mutex = self.get_or_create_mutex(ctx, ok.id);
                mutex.do_send(OkMessage {
                    from,
                    connected_servers,
                });
            }
        };
    }

    fn handle_commit(&mut self, from: ServerId, packet: TPCommitPacket, _ctx: &mut Context<Self>) {
        match packet {
            TPCommitPacket::Prepare(packet) => {
                self.two_phase_commit.do_send(packet.to_message(from));
            }
            TPCommitPacket::Commit(packet) => {
                self.two_phase_commit.do_send(packet.to_message(from));
            }
            TPCommitPacket::Rollback(packet) => {
                self.two_phase_commit.do_send(packet.to_message());
            }
            TPCommitPacket::VoteYes(packet) => {
                self.two_phase_commit
                    .do_send(packet.to_message(from, self.get_connected_servers()));
            }
            TPCommitPacket::VoteNo(packet) => {
                self.two_phase_commit.do_send(packet.to_message(from));
            }
        }
    }

    fn get_or_create_mutex(
        &mut self,
        ctx: &mut Context<PacketDispatcher>,
        id: ResourceId,
    ) -> &mut Addr<DistMutex<PacketDispatcher>> {
        let mutex = self.mutexes.entry(id).or_insert_with(|| {
            info!("Creating mutex for Resource {}", id);
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
        data: Packet,
    ) -> Request<ConnectionHandler<Packet, Packet>, SendPacket<Packet>> {
        self.socket.send(SendPacket {
            to: to.get_socket_addr(self.config.server_port),
            data,
        })
    }
}
