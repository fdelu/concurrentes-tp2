use std::{collections::HashMap, net::SocketAddr};

use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Recipient, ResponseActFuture,
    System, WrapFuture,
};
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
    active_coffees: HashMap<TxId, (Coffee, Recipient<MakeCoffee>)>,
    next_tx_id: TxId,
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
            active_coffees: HashMap::new(),
            next_tx_id: 0,
        })
    }

    /// Crea un identificador de transaccion unico dentro de esta cafetera.
    /// Tambien guarda ese identificador entre los identificadores activos.
    fn add_tx_id(&mut self, coffee: Option<(Coffee, Recipient<MakeCoffee>)>) -> TxId {
        let tx_id = self.next_tx_id;
        self.next_tx_id = self.next_tx_id.wrapping_add(1);

        if let Some(tuple) = coffee {
            self.active_coffees.insert(tx_id, tuple);
        }
        tx_id
    }

    /// Saca un identificador de transaccion de los identificadores activos.
    fn remove_tx_id(&mut self, tx_id: TxId) -> Option<(Coffee, Recipient<MakeCoffee>)> {
        let tuple = self.active_coffees.remove(&tx_id);
        if tuple.is_none() {
            warn!(
                "Got message from the server for inactive transaction: {}",
                tx_id
            );
        }
        tuple
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
        }
    }
}

impl Handler<PrepareOrder> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo PrepareOrder.
    /// Le envia al servidor la orden.
    fn handle(&mut self, msg: PrepareOrder, _ctx: &mut Self::Context) -> Self::Result {
        let coffee = msg.coffee.clone();
        let transaction_id = self.add_tx_id(Some((msg.coffee, msg.maker)));
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
        warn!("Aborting order with transaction id {}", msg.transaction_id)
    }
}

impl Handler<AddMoney> for OrderProcessor {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja los mensajes de tipo AddMoney.
    /// Envia al servidor para agregar dinero a la cuenta de un usuario.
    fn handle(&mut self, msg: AddMoney, _ctx: &mut Self::Context) -> Self::Result {
        let transaction_id = self.add_tx_id(None);
        let server_socket = self.server_socket.clone();

        async move {
            let res = server_socket
                .send(SocketSend {
                    data: ClientPacket::AddPoints(msg.user_id, msg.amount, transaction_id),
                })
                .await;

            if let Err(e) = res.flatten() as Result<(), CoffeeError> {
                error!("Error sending AddMoney: {}", e);
            } else {
                info!("Sent AddMoney with transaction id {}", transaction_id)
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
