use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Duration;

use actix::prelude::*;

use common::AHandler;
use tracing::{debug, info, trace};

use crate::config::Config;
use crate::dist_mutex::messages::ack::AckMessage;
use crate::dist_mutex::messages::ok::OkMessage;
use crate::dist_mutex::messages::request::RequestMessage;
use crate::dist_mutex::packets::{get_timestamp, MutexPacket, ResourceId, Timestamp};
use crate::dist_mutex::server_id::ServerId;
use crate::dist_mutex::{DistMutex, MutexCreationTrait};
use crate::network::{ConnectionHandler, ReceivedPacket, SendPacket};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use crate::packet_dispatcher::messages::send::SendMessage;
use crate::packet_dispatcher::packet::{Packet, SyncRequestPacket, SyncResponsePacket};
use crate::two_phase_commit::messages::commit::CommitMessage;
use crate::two_phase_commit::messages::forward_database::ForwardDatabaseMessage;
use crate::two_phase_commit::messages::prepare::PrepareMessage;
use crate::two_phase_commit::messages::rollback::RollbackMessage;
use crate::two_phase_commit::messages::update_database::UpdateDatabaseMessage;
use crate::two_phase_commit::messages::vote_no::VoteNoMessage;
use crate::two_phase_commit::messages::vote_yes::VoteYesMessage;
use crate::two_phase_commit::packets::TwoPhaseCommitPacket;
use crate::two_phase_commit::{make_initial_database, TwoPhaseCommit};

pub mod messages;
pub mod packet;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(120);
pub type ClientId = u32;

pub trait PacketDispatcherTrait:
    AHandler<ReceivedPacket<Packet>>
    + AHandler<BroadcastMessage>
    + AHandler<PruneMessage>
    + AHandler<SendMessage>
{
}

impl PacketDispatcherTrait for PacketDispatcher {}

pub struct PacketDispatcher {
    server_id: ServerId,
    mutexes: HashMap<ResourceId, Addr<DistMutex<Self>>>,
    socket: Addr<ConnectionHandler<Packet, Packet>>,
    servers_last_seen: HashMap<ServerId, Option<Timestamp>>,
    two_phase_commit: Addr<TwoPhaseCommit<Self>>,
    config: Config,
}

impl Actor for PacketDispatcher {
    type Context = Context<Self>;
}

impl PacketDispatcher {
    pub fn new(cfg: &Config) -> Addr<Self> {
        let my_addr = SocketAddr::new(cfg.server_ip, cfg.server_port);
        let my_id = ServerId::new(cfg.server_ip);
        let servers_last_seen = cfg
            .servers
            .iter()
            .filter(|&&server_id| server_id != my_id)
            .map(|&server_id| (server_id, None))
            .collect();
        trace!("Initial servers: {:?}", servers_last_seen);

        Self::create(|ctx| {
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
                    let mutex = DistMutex::new(my_id, client_id, ctx.address()).start();
                    (client_id, mutex)
                })
                .collect();

            let mut ret = Self {
                server_id: my_id,
                mutexes,
                socket,
                servers_last_seen,
                two_phase_commit,
                config: cfg.clone(),
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
                debug!("Received request from {}", from);
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

    fn handle_commit(
        &mut self,
        from: ServerId,
        packet: TwoPhaseCommitPacket,
        _ctx: &mut Context<Self>,
    ) {
        match packet {
            TwoPhaseCommitPacket::Prepare(packet) => {
                let message = PrepareMessage {
                    from,
                    id: packet.id,
                    transaction: packet.transaction,
                };
                self.two_phase_commit.try_send(message).unwrap();
            }
            TwoPhaseCommitPacket::Commit(packet) => {
                let message = CommitMessage {
                    id: packet.id,
                    from,
                };
                self.two_phase_commit.try_send(message).unwrap();
            }
            TwoPhaseCommitPacket::Rollback(packet) => {
                let message = RollbackMessage { id: packet.id };
                self.two_phase_commit.try_send(message).unwrap();
            }
            TwoPhaseCommitPacket::VoteYes(packet) => {
                let message = VoteYesMessage {
                    id: packet.id,
                    from,
                    connected_servers: self.get_connected_servers(),
                };
                self.two_phase_commit.try_send(message).unwrap();
            }
            TwoPhaseCommitPacket::VoteNo(packet) => {
                let message = VoteNoMessage {
                    id: packet.id,
                    from,
                };
                self.two_phase_commit.try_send(message).unwrap();
            }
        }
    }

    fn handle_sync_request(
        &mut self,
        from: ServerId,
        _packet: SyncRequestPacket,
        _ctx: &mut Context<Self>,
    ) {
        self.two_phase_commit
            .try_send(ForwardDatabaseMessage { to: from })
            .unwrap();
    }

    fn handle_sync_response(
        &mut self,
        _from: ServerId,
        packet: SyncResponsePacket,
        _ctx: &mut Context<Self>,
    ) {
        self.two_phase_commit
            .try_send(UpdateDatabaseMessage {
                snapshot_from: packet.snapshot_from,
                database: packet.database,
                logs: packet.logs,
            })
            .unwrap();
    }

    fn get_or_create_mutex(
        &mut self,
        ctx: &mut Context<PacketDispatcher>,
        id: ResourceId,
    ) -> &mut Addr<DistMutex<PacketDispatcher>> {
        let mutex = self.mutexes.entry(id).or_insert_with(|| {
            info!("Creating mutex for {}", id);
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
