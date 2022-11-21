use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use tracing::info;

use crate::packet_dispatcher::messages::{BlockPointsMessage, DiscountMessage, QueuePointsMessage};
use crate::packet_dispatcher::TransactionId;
use crate::{
    config::Config,
    network::{ConnectionHandler, Listen, SendPacket},
    packet_dispatcher::PacketDispatcher,
    ServerId,
};
use common::{
    error::{CoffeeError, FlattenResult},
    packet::{Amount, ClientPacket, ServerPacket, TxId, UserId},
    socket::{ReceivedPacket, SocketError},
};

/// Maneja las conexiones con clientes.
pub struct ClientConnections {
    // Dirección del ConnectionHandler
    socket: Addr<ConnectionHandler<ServerPacket, ClientPacket>>,
    // Dirección del PacketDispatcher
    dispatcher_addr: Addr<PacketDispatcher>,
    // Transacciones recibidas
    prep_transactions: HashMap<SocketAddr, HashMap<TxId, UserId>>,
    // Id del servidor asociado a esta instancia
    server_id: ServerId,
}

impl Actor for ClientConnections {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("ClientConnections stopped");
    }
}

impl ClientConnections {
    /// Crea un nuevo [ClientConnections]. Argumentos:
    /// - `cfg`: Configuración del servidor.
    /// - `dispatcher_addr`: Dirección del [PacketDispatcher].
    pub fn new(cfg: &Config, dispatcher_addr: Addr<PacketDispatcher>) -> Addr<Self> {
        let my_addr = SocketAddr::new(cfg.server_ip, cfg.client_port);
        Self::create(|ctx| {
            let socket =
                ConnectionHandler::new(ctx.address().recipient(), my_addr, false, None).start();
            ClientConnections {
                socket,
                dispatcher_addr,
                prep_transactions: HashMap::new(),
                server_id: ServerId::new(cfg.server_ip),
            }
        })
    }

    fn send_ready(&mut self, tx_id: TxId, addr: SocketAddr, user_id: UserId) {
        info!("Sending READY for user {} (tx {})", user_id, tx_id);
        let trans_to_user = self
            .prep_transactions
            .entry(addr)
            .or_insert_with(HashMap::new);
        trans_to_user.insert(tx_id, user_id);
        self.socket.do_send(SendPacket {
            to: addr,
            data: ServerPacket::Ready(tx_id),
        });
    }

    fn send_error(&mut self, tx_id: TxId, addr: SocketAddr, err: CoffeeError) {
        info!("Sending ERROR for tx {}", tx_id);
        self.socket.do_send(SendPacket {
            to: addr,
            data: ServerPacket::ServerErrror(tx_id, err),
        });
    }

    /// Maneja los mensajes de tipo PrepareOrder que se reciben a traves del socket.
    /// Envia la orden al [PacketDispatcher] con el mensaje [BlockPointsMessage] y
    /// si sale bien envia un mensaje de tipo Ready a la cafetera, sino le envia
    /// un error.
    fn prepare_order(
        &mut self,
        user_id: UserId,
        amount: Amount,
        tx_id: TxId,
        addr: SocketAddr,
    ) -> ResponseActFuture<Self, ()> {
        let dispatcher_addr = self.dispatcher_addr.clone();
        let server_id = self.server_id;
        let future = async move {
            dispatcher_addr
                .send(BlockPointsMessage {
                    transaction_id: TransactionId::Discount(server_id, addr, tx_id),
                    user_id,
                    amount,
                })
                .await
        }
        .into_actor(self)
        .boxed_local();
        future
            .then(move |message_res, this, _| {
                match message_res.flatten() {
                    Ok(_) => this.send_ready(tx_id, addr, user_id),
                    Err(err) => this.send_error(tx_id, addr, err),
                };
                async {}.into_actor(this).boxed_local()
            })
            .boxed_local()
    }

    /// Maneja los mensajes de tipo CommitOrder que se reciben a traves del socket.
    /// Primero corrobora que la transaccion a commitear se haya preparado y de ser
    /// asi envia un [DiscountMessage] al [PacketDispatcher].
    fn commit_order(&mut self, tx_id: TxId, addr: SocketAddr) -> ResponseActFuture<Self, ()> {
        let dispatcher_addr = self.dispatcher_addr.clone();
        let user_id: UserId = match self
            .prep_transactions
            .get(&addr)
            .and_then(move |hash| hash.get(&tx_id))
        {
            Some(user_id) => *user_id,
            None => {
                self.send_error(
                    tx_id,
                    addr,
                    CoffeeError::new("no prepare for this transaction"),
                );
                return async {}.into_actor(self).boxed_local();
            }
        };
        dispatcher_addr.do_send(DiscountMessage {
            transaction_id: TransactionId::Discount(self.server_id, addr, tx_id),
            user_id,
        });
        async {}.into_actor(self).boxed_local()
    }

    /// Maneja los mensajes de tipo CommitOrder que se reciben a traves del socket.
    /// Envia un mensaje tipo [QueuePointsMessage] al [PacketDispatcher].
    fn add_points(&mut self, user_id: UserId, amount: Amount) -> ResponseActFuture<Self, ()> {
        self.dispatcher_addr.do_send(QueuePointsMessage {
            id: user_id,
            amount,
        });
        async {}.into_actor(self).boxed_local()
    }
}

impl Handler<ReceivedPacket<ClientPacket>> for ClientConnections {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los paquetes que envia el cliente a traves del [ConnectionHandler].
    fn handle(
        &mut self,
        msg: ReceivedPacket<ClientPacket>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg.data {
            ClientPacket::PrepareOrder(user_id, cost, tx_id) => {
                self.prepare_order(user_id, cost, tx_id, msg.addr)
            }
            ClientPacket::CommitOrder(tx_id) => self.commit_order(tx_id, msg.addr),
            ClientPacket::AddPoints(amount, user_id, _) => {
                self.add_points(user_id, amount) //TODO: check where this data is crossed
            }
        }
    }
}

impl Handler<Listen> for ClientConnections {
    type Result = ResponseActFuture<Self, Result<(), SocketError>>;

    fn handle(&mut self, msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        let socket_actor_addr = self.socket.clone();
        let dispatcher_addr = self.dispatcher_addr.clone();
        async move {
            match socket_actor_addr.send(msg).await.flatten() {
                Ok(_) => (),
                Err(e) => return Err(e),
            };

            match dispatcher_addr.send(Listen {}).await.flatten() {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}
