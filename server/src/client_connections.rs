use std::{net::SocketAddr, collections::{HashMap, HashSet}};
use actix::{Addr, Actor, Context, AsyncContext, Handler, ResponseActFuture, WrapFuture, ActorFutureExt};
use common::{packet::{ClientPacket, ServerPacket, TxId, UserId, Amount}, socket::{ReceivedPacket, SocketError, Socket}};
use crate::{network::{ConnectionHandler, Listen, Packet, SendPacket}, config::Config, packet_dispatcher::{PacketDispatcher, messages::{dispatch, public::{block_points::BlockPointsMessage, discount::DiscountMessage}}}, two_phase_commit::TransactionId};

pub struct ClientConnections {
    socket: Addr<ConnectionHandler<ServerPacket, ClientPacket>>,
    dispatcher_addr: Addr<PacketDispatcher>,
    prep_transactions: HashMap<SocketAddr, HashMap<TxId, UserId>>,
}

impl Actor for ClientConnections {
    type Context = Context<Self>;
}

impl ClientConnections {
    pub fn new(cfg: &Config, dispatcher_addr: Addr<PacketDispatcher>) -> Addr<Self> {
        let my_addr = SocketAddr::new(cfg.server_ip, cfg.client_port);
        Self::create(|ctx| {
            let socket = ConnectionHandler::new(
                ctx.address().recipient(),
                my_addr,
                false,
                None,
            )
            .start();
            ClientConnections { socket, dispatcher_addr, prep_transactions: HashMap::new()}
        })
    }

    fn send_ready(&mut self, tx_id: TxId, addr: SocketAddr, user_id: UserId) {
        let mut trans_to_user = self.prep_transactions.entry(addr).or_insert_with(HashMap::new);
        trans_to_user.insert(tx_id, user_id);
        self.socket.do_send(SendPacket {
            to: addr,
            data: ServerPacket::Ready(tx_id),
        });
    }

    fn send_error(&mut self, tx_id: TxId, addr: SocketAddr, err: SocketError) {
        self.socket.do_send(SendPacket {
            to: addr,
            data: ServerPacket::ServerErrror(tx_id, err),
        });
    }

    fn prepare_order(&mut self, user_id: UserId, amount: Amount, tx_id: TxId, addr: SocketAddr) -> ResponseActFuture<Self, ()> {
        let dispatcher_addr = self.dispatcher_addr.clone();
        let future = async move {
            let transaction_id: u64 = ((user_id as u64) << 32) + tx_id as u64;
            dispatcher_addr
            .send(BlockPointsMessage {
                transaction_id,
                client_id: user_id,
                amount,
            })
            .await
        }
        .into_actor(self)
        .boxed_local();
        future.then(move |message_res, this, _| {
            match message_res {
                Ok(Ok(_)) => this.send_ready(tx_id, addr, user_id),
                Ok(Err(_)) => this.send_error(tx_id, addr, SocketError::new("packet dispatcher error")),
                Err(err) => this.send_error(tx_id, addr, err.into()),
            };
            async { () }.into_actor(this).boxed_local()
        }).boxed_local()
    }

    fn commit_order(&mut self, tx_id: TxId, addr: SocketAddr) -> ResponseActFuture<Self, ()> {
        let dispatcher_addr = self.dispatcher_addr.clone();
        let user_id: UserId =
        match self.prep_transactions.get(&addr).and_then(move |hash| hash.get(&tx_id)) {
            Some(user_id) => {
               *user_id
            },
            None => {self.send_error(tx_id, addr, SocketError::new("no prepare for this transaction"));
                return async { () }.into_actor(self).boxed_local()
            },
        };
        dispatcher_addr
            .do_send(DiscountMessage {
                transaction_id: ((user_id as u64) << 32) + tx_id as u64,
                client_id: user_id,
            });
        async { () }.into_actor(self).boxed_local()
    }
}

impl Handler<ReceivedPacket<ClientPacket>> for ClientConnections {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ReceivedPacket<ClientPacket>, ctx: &mut Self::Context) -> Self::Result {
        match msg.data {
            ClientPacket::PrepareOrder(user_id, cost, tx_id) => self.prepare_order(user_id, cost, tx_id, msg.addr),
            ClientPacket::CommitOrder(tx_id) => self.commit_order(tx_id, msg.addr),
            ClientPacket::AddPoints(user_id, amount, tx_id) => todo!(),
        }
    }
}

impl Handler<Listen> for ClientConnections {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        let socket_actor_addr = self.socket.clone();
        let dispatcher_addr = self.dispatcher_addr.clone();
        async move {
            match socket_actor_addr.send(msg).await {
                Ok(_) => (),
                Err(e) => return Err(SocketError::from(e)),
            };

            match dispatcher_addr.send(Listen {}).await {
                Ok(_) => Ok(()),
                Err(e) => Err(SocketError::from(e)),
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

