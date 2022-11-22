use std::{future::Future, net::SocketAddr};

use actix::{Actor, Addr, Recipient};
#[cfg(mocks)]
use mockall::automock;
use tokio::{
    task::{spawn, JoinHandle},
    time::Duration,
};

#[cfg(mocks)]
use common::socket::test_util::{MockSocket as Socket, MockStream as Stream};
use common::socket::{PacketRecv, PacketSend, ReceivedPacket, SocketEnd, SocketError, SocketSend};
#[cfg(not(mocks))]
use common::socket::{Socket, Stream};

/// Maneja la conexión de un socket.
pub struct Connection<S: PacketSend, R: PacketRecv> {
    // Dirección del socket
    socket: Addr<Socket<S, R>>,
    // Task del timeout
    cancel_task: Option<JoinHandle<()>>,
    // Actor que recibe un mensaje cuando se cierra el socket
    end_handler: Recipient<SocketEnd>,
    // Dirección del socket
    addr: SocketAddr,
    // Tiempo de timeout
    timeout: Option<Duration>,
}

#[cfg_attr(mocks, automock)]
impl<S: PacketSend, R: PacketRecv> Connection<S, R> {
    /// Crea una nueva conexión. Argumentos:
    /// - `stream`: [Stream] del [Socket].
    /// - `end_handler`: Actor que recibe un mensaje cuando se cierra el socket.
    /// - `addr`: Dirección a la cual conectarse.
    /// - `timeout`: Tiempo de timeout.
    pub fn new(
        end_handler: Recipient<SocketEnd>,
        received_handler: Recipient<ReceivedPacket<R>>,
        addr: SocketAddr,
        stream: Stream,
        timeout: Option<Duration>,
    ) -> Self {
        let socket = Socket::new(received_handler, end_handler.clone(), addr, stream);
        let mut this = Connection {
            socket: socket.start(),
            cancel_task: None,
            end_handler,
            addr,
            timeout,
        };
        this.restart_timeout();
        this
    }

    fn cancel_timeout(&mut self) {
        if let Some(task) = self.cancel_task.take() {
            task.abort();
        }
    }

    /// Reinicia el timer de timeout, si se configuró uno.
    pub fn restart_timeout(&mut self) {
        if let Some(timeout) = self.timeout {
            self.cancel_timeout();
            let end_handler = self.end_handler.clone();
            let addr = self.addr;
            self.cancel_task = Some(spawn(async move {
                tokio::time::sleep(timeout).await;
                end_handler.do_send(SocketEnd { addr });
            }));
        }
    }

    /// Envia un paquete por el socket.
    /// Nota: Esta función no es `async` sino que devuelve un [Future] manualmente
    /// para que no mantenga una referencia a `self` durante la espera.
    pub fn send(&self, data: S) -> impl Future<Output = Result<(), SocketError>> {
        let socket = self.socket.clone();

        async move { socket.send(SocketSend { data }).await? }
    }
}

#[cfg(mocks)]
pub mod test {
    use std::sync::{Mutex, MutexGuard};

    use common::socket::{PacketRecv, PacketSend};
    use mockall::lazy_static;

    use super::{MockConnection as Connection, __mock_MockConnection};

    // ver https://github.com/asomers/mockall/blob/master/mockall/examples/synchronization.rs
    lazy_static! {
        static ref MTX: Mutex<()> = Mutex::new(());
    }

    fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
        match m.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Guard de [connection_new_context]. Contiene el contexto del mock y el guard del mutex
    /// estático que impide que se inicialice el mock en varios tests a la vez.
    pub struct Guard<S: PacketSend, R: PacketRecv> {
        pub ctx: __mock_MockConnection::__new::Context<S, R>,
        guard: MutexGuard<'static, ()>,
    }

    /// Función de utilidad para mockear la [Connection].
    pub fn connection_new_context<S: PacketSend, R: PacketRecv>() -> Guard<S, R> {
        let m = get_lock(&MTX);

        let context = Connection::new_context();

        Guard {
            ctx: context,
            guard: m,
        }
    }
}
