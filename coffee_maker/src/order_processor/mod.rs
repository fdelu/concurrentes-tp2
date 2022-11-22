use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Recipient, ResponseActFuture,
    System, WrapFuture,
};
use rand::Rng;
use tracing::{error, info, warn};

use crate::coffee_maker::{Coffee, MakeCoffee};
use common::{
    error::{CoffeeError, FlattenResult},
    packet::{ClientPacket, ServerPacket, TxId},
    socket::{ReceivedPacket, Socket, SocketEnd, SocketSend, Stream},
};

mod messages;
pub use messages::*;

impl OrderProcessorTrait for OrderProcessor {}

///Actor para comunicacion entre la cafetera y su servidor local
pub(crate) struct OrderProcessor {
    server_socket: Addr<Socket<ClientPacket, ServerPacket>>,
    transactions: HashMap<TxId, TransactionInfo>,
}

/// Enum that holds the state of a transaction
enum TransactionInfo {
    PreparingCoffee(Coffee, Recipient<MakeCoffee>),
    Completed,
}

impl Actor for OrderProcessor {
    type Context = Context<Self>;
}

impl OrderProcessor {
    ///constructor de OrderProcessor, recibe la direccion del servidor
    pub fn new(server_addr: SocketAddr) -> Addr<Self> {
        Self::create(move |ctx| Self {
            server_socket: Socket::new(
                ctx.address().recipient(),
                ctx.address().recipient(),
                server_addr,
                Stream::New,
            )
            .start(),
            transactions: HashMap::new(),
        })
    }

    /// Crea un identificador de transaccion unico dentro de esta cafetera.
    /// Tambien guarda ese identificador entre los identificadores activos.
    fn add_tx_id(&mut self, info: TransactionInfo) -> TxId {
        let mut rng = rand::thread_rng();
        let mut tx_id = rng.gen();
        while self.transactions.contains_key(&tx_id) {
            tx_id = rng.gen();
        }

        self.transactions.insert(tx_id, info);
        tx_id
    }

    /// Saca un identificador de transacción de los identificadores activos.
    /// Si el identificador corresponde a un café en preparación, lo devuelve junto
    /// con el actor que lo prepara.
    fn remove_tx_id(&mut self, tx_id: TxId) -> Option<(Coffee, Recipient<MakeCoffee>)> {
        let trx_info = self.transactions.remove(&tx_id);
        match trx_info {
            None => warn!(
                "Got message from the server for unknown transaction: {}",
                tx_id
            ),
            Some(TransactionInfo::Completed) => warn!(
                "Got message from the server for already completed transaction: {}",
                tx_id
            ),
            Some(TransactionInfo::PreparingCoffee(c, m)) => return Some((c, m)),
        }
        None
    }

    /// Para cuando se recibio un mensaje ready del servidor.
    /// Le avisa a la cafetera que puede comenzar la preparacion del cafe.
    fn handle_ready(&mut self, tx_id: TxId) {
        if let Some((coffee, maker)) = self.remove_tx_id(tx_id) {
            maker.do_send(MakeCoffee { coffee, tx_id });
        }
    }

    /// Para cuando se recibio un mensaje insufficient del servidor.
    /// Imprime el error y descuntinua el identificador de transaccion.
    fn handle_insufficient(&mut self, tx_id: TxId) {
        if let Some((coffee, _)) = self.remove_tx_id(tx_id) {
            info!(
                "Couldn't make coffee {}: Insufficient funds\nTransaction ID: {}",
                coffee.name, tx_id
            );
            self.transactions.insert(tx_id, TransactionInfo::Completed);
        }
    }

    /// Para cuando se recibio un mensaje error del servidor.
    /// Imprime el error y descuntinua el identificador de transaccion.
    fn handle_server_error(&mut self, tx_id: TxId, error: CoffeeError) {
        if let Some((coffee, _)) = self.remove_tx_id(tx_id) {
            error!(
                "Couldn't make coffee {}: Server error ({})\nTransaction ID: {}",
                coffee.name, error, tx_id
            );
            self.transactions.insert(tx_id, TransactionInfo::Completed);
        }
    }
}

impl Handler<PrepareOrder> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo PrepareOrder.
    /// Le envia al servidor la orden.
    fn handle(&mut self, msg: PrepareOrder, _ctx: &mut Self::Context) -> Self::Result {
        let coffee = msg.coffee.clone();
        let transaction_id =
            self.add_tx_id(TransactionInfo::PreparingCoffee(msg.coffee, msg.maker));
        info!(
            "Assigned transaction id {} to coffee {}",
            transaction_id, coffee.name
        );
        let server_socket = self.server_socket.clone();
        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::PrepareOrder(coffee.user_id, coffee.cost, transaction_id),
                })
                .await;
            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending PrepareOrder: {}", e);
            } else {
                info!("Sent PrepareOrder with transaction id {}", transaction_id);
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<CommitOrder> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo CommitOrder.
    /// Envia al servidor para conretar la orden.
    fn handle(&mut self, msg: CommitOrder, _ctx: &mut Self::Context) -> Self::Result {
        let server_socket = self.server_socket.clone();
        self.transactions
            .insert(msg.transaction_id, TransactionInfo::Completed);
        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::CommitOrder(msg.transaction_id),
                })
                .await;
            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending CommitOrder: {}", e);
            } else {
                info!(
                    "Sent CommitOrder with transaction id {}",
                    msg.transaction_id
                )
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<AbortOrder> for OrderProcessor {
    type Result = ();

    /// Maneja los mensajes de tipo AbortOrder.
    /// Envia al servidor para abortar la orden.
    /// Esta funcion esta fuera de uso.
    fn handle(&mut self, msg: AbortOrder, _ctx: &mut Self::Context) {
        self.transactions
            .insert(msg.transaction_id, TransactionInfo::Completed);
        warn!("Aborting order with transaction id {}", msg.transaction_id)
    }
}

impl Handler<AddPoints> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo AddPoints.
    /// Envia al servidor para agregar dinero a la cuenta de un usuario.
    fn handle(&mut self, msg: AddPoints, _ctx: &mut Self::Context) -> Self::Result {
        let transaction_id = self.add_tx_id(TransactionInfo::Completed);
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::AddPoints(msg.user_id, msg.amount, transaction_id),
                })
                .await;

            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending AddPoints: {}", e);
            } else {
                info!("Sent AddPoints with transaction id {}", transaction_id)
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl Handler<SocketEnd> for OrderProcessor {
    type Result = ();

    /// Maneja el mensaje para finalizar la ejecucion.
    fn handle(&mut self, _msg: SocketEnd, _ctx: &mut Self::Context) -> Self::Result {
        error!("Critical error: server socket closed");
        System::current().stop_with_code(-1);
    }
}

impl Handler<ReceivedPacket<ServerPacket>> for OrderProcessor {
    type Result = ();

    /// Recive los mensajes del servidor y los procesa.
    fn handle(
        &mut self,
        msg: ReceivedPacket<ServerPacket>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg.data {
            ServerPacket::Ready(tx_id) => self.handle_ready(tx_id),
            ServerPacket::Insufficient(tx_id) => self.handle_insufficient(tx_id),
            ServerPacket::ServerErrror(tx_id, e) => self.handle_server_error(tx_id, e),
        }
    }
}
