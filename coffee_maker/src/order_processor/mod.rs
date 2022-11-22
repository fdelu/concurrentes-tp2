use std::{collections::HashMap, future::Future, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Recipient, ResponseActFuture,
    System, WrapFuture,
};
use rand::Rng;
use tokio::sync::oneshot;
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

#[derive(Debug)]
/// Enum that holds the state of a transaction
enum TransactionInfo {
    Preparing(TransactionData),
    MakingCoffee(TransactionData),
    Finished(TransactionResult),
}

#[derive(Debug)]
/// Holds the data of a coffee in preparation
struct TransactionData {
    coffee: Coffee,
    coffee_maker: Recipient<MakeCoffee>,
    result_tx: oneshot::Sender<TransactionResult>,
}

#[derive(Debug, Clone)]
/// Result of a transaction
pub enum TransactionResult {
    Completed,
    Failed(CoffeeError),
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

    /// Para cuando se recibio un mensaje ready del servidor.
    /// Le avisa a la cafetera que puede comenzar la preparacion del cafe.
    fn handle_ready(&mut self, tx_id: TxId) {
        let status = self.transactions.remove(&tx_id);
        if let Some(TransactionInfo::Preparing(data)) = status {
            data.coffee_maker.do_send(MakeCoffee {
                coffee: data.coffee.clone(),
                tx_id,
            });
            self.transactions
                .insert(tx_id, TransactionInfo::MakingCoffee(data));
            return;
        } else if let Some(s) = status {
            warn!(
                "Got Ready for transaction {} that is not being redeemed. Status: {:?}",
                tx_id, s
            );
            self.transactions.insert(tx_id, s); // Re-insert the removed status
            return;
        }
        warn!("Got Ready for unknown transaction {}", tx_id);
    }

    /// Para cuando se recibio un mensaje insufficient del servidor.
    /// Imprime el error y descuntinua el identificador de transaccion.
    fn handle_insufficient(&mut self, tx_id: TxId) {
        let status = self.transactions.remove(&tx_id);
        if let Some(TransactionInfo::Preparing(data)) = status {
            info!(
                "Couldn't make coffee {}: Insufficient funds\nTransaction ID: {}",
                data.coffee.name, tx_id
            );
            let result = TransactionResult::Failed(CoffeeError::new("Insufficient points"));
            self.finish(tx_id, result, data.result_tx);
            return;
        } else if let Some(s) = status {
            warn!(
                "Got InsufficientPoints for transaction {} that is not being redeemed. Status: {:?}",
                tx_id, s
            );
            self.transactions.insert(tx_id, s); // Re-insert the removed status
            return;
        }
        warn!("Got InsufficientPoints for unknown transaction {}", tx_id);
    }

    /// Para cuando se recibio un mensaje error del servidor.
    /// Imprime el error y descuntinua el identificador de transaccion.
    fn handle_server_error(&mut self, tx_id: TxId, error: CoffeeError) {
        let status = self.transactions.remove(&tx_id);
        if let Some(TransactionInfo::Preparing(data)) = status {
            error!(
                "Couldn't make coffee {}: Server error ({})\nTransaction ID: {}",
                data.coffee.name, error, tx_id
            );
            let result = TransactionResult::Failed(CoffeeError::from(error));
            self.finish(tx_id, result, data.result_tx);
            return;
        } else if let Some(s) = status {
            warn!(
                "Got Server Error for transaction {} that is not being redeemed. Status: {:?}",
                tx_id, s
            );
            self.transactions.insert(tx_id, s); // Re-insert the removed status
            return;
        }
        warn!("Got Server Error for unknown transaction {}", tx_id);
    }

    /// Para cuando se recibe un mensaje de Redeem de la cafetera.
    /// Envía el mensaje al servidor y devuelve el resultado final de la transacción
    fn handle_redeem(
        &self,
        tx_id: TxId,
        coffee: Coffee,
        result_rx: oneshot::Receiver<TransactionResult>,
    ) -> impl Future<Output = TransactionResult> {
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::RedeemCoffee(coffee.user_id, coffee.cost, tx_id),
                })
                .await;
            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending RedeemCoffee: {}", e);
                TransactionResult::Failed(CoffeeError::from(e))
            } else {
                info!("Sent RedeemCoffee with transaction id {}", tx_id);
                result_rx
                    .await
                    .unwrap_or_else(|e| TransactionResult::Failed(CoffeeError::from(e)))
            }
        }
    }

    /// Para cuando se recibe un mensaje de CommitRedemption de la cafetera.
    /// Envía el mensaje al servidor y envía por result_tx el resultado final,
    /// devolviéndolo
    fn handle_commit(
        &self,
        tx_id: TxId,
        result_tx: oneshot::Sender<TransactionResult>,
    ) -> ResponseActFuture<Self, ()> {
        let server_socket = self.server_socket.clone();
        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::CommitRedemption(tx_id),
                })
                .await;
            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending CommitRedemption: {}", e);
                TransactionResult::Failed(CoffeeError::from(e))
            } else {
                info!("Sent CommitRedemption with transaction id {}", tx_id);
                TransactionResult::Completed
            }
        }
        .into_actor(self)
        .then(move |r, this, _| {
            this.finish(tx_id, r, result_tx);
            async {}.into_actor(this).boxed_local()
        })
        .boxed_local()
    }

    /// Marca una transacción como terminada, enviando el resultado por el sender
    fn finish(
        &mut self,
        tx_id: TxId,
        result: TransactionResult,
        sender: oneshot::Sender<TransactionResult>,
    ) {
        let _ = sender.send(result.clone());
        self.transactions
            .insert(tx_id, TransactionInfo::Finished(result));
    }
}

impl Handler<RedeemCoffee> for OrderProcessor {
    type Result = ResponseActFuture<Self, TransactionResult>;

    /// Maneja los mensajes de tipo RedeemCoffee.
    /// Le envia al servidor la solicitud de canje de puntos por café.
    fn handle(&mut self, msg: RedeemCoffee, _ctx: &mut Self::Context) -> Self::Result {
        let coffee = msg.coffee.clone();
        let (result_tx, result_rx) = oneshot::channel();
        let transaction_id = self.add_tx_id(TransactionInfo::Preparing(TransactionData {
            coffee: msg.coffee,
            coffee_maker: msg.maker,
            result_tx,
        }));
        info!(
            "Assigned transaction id {} to coffee {}",
            transaction_id, coffee.name
        );

        self.handle_redeem(transaction_id, coffee, result_rx)
            .into_actor(self)
            .boxed_local()
    }
}

impl Handler<CommitRedemption> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo CommitRedemption.
    /// Envia al servidor para concretar el canje.
    fn handle(&mut self, msg: CommitRedemption, _ctx: &mut Self::Context) -> Self::Result {
        let status = self.transactions.remove(&msg.transaction_id);
        if let Some(TransactionInfo::MakingCoffee(data)) = status {
            return self.handle_commit(msg.transaction_id, data.result_tx);
        } else if let Some(s) = status {
            warn!(
                "Got CommitRedemption for transaction {} that is not in the making. Status: {:?}",
                msg.transaction_id, s
            );
            self.transactions.insert(msg.transaction_id, s); // Re-insert the removed status
        } else {
            warn!(
                "Got CommitRedemption for unknown transaction {}",
                msg.transaction_id
            );
        }

        async {}.into_actor(self).boxed_local()
    }
}

impl Handler<AbortRedemption> for OrderProcessor {
    type Result = ();

    /// Maneja los mensajes de tipo AbortRedemption.
    /// Envia al servidor para abortar canjeo del café.
    fn handle(&mut self, msg: AbortRedemption, _ctx: &mut Self::Context) {
        let status = self.transactions.remove(&msg.transaction_id);
        if let Some(TransactionInfo::MakingCoffee(data)) = status {
            warn!("Aborting order with transaction id {}", msg.transaction_id);
            let result =
                TransactionResult::Failed(CoffeeError::new("Order aborted: Coffee maker failed"));
            self.finish(msg.transaction_id, result, data.result_tx);
            return;
        }
        warn!(
            "Got AbortRedemption for transaction {} that is not in the making. Status: {:?}",
            msg.transaction_id, status
        );
    }
}

impl Handler<AddPoints> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo AddPoints.
    /// Envia al servidor para agregar dinero a la cuenta de un usuario.
    fn handle(&mut self, msg: AddPoints, _ctx: &mut Self::Context) -> Self::Result {
        let transaction_id =
            self.add_tx_id(TransactionInfo::Finished(TransactionResult::Completed));
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

    /// Recibe los mensajes del servidor y los procesa.
    fn handle(
        &mut self,
        msg: ReceivedPacket<ServerPacket>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg.data {
            ServerPacket::Ready(tx_id) => self.handle_ready(tx_id),
            ServerPacket::Insufficient(tx_id) => self.handle_insufficient(tx_id),
            ServerPacket::ServerError(tx_id, e) => self.handle_server_error(tx_id, e),
        }
    }
}
