use std::net::SocketAddr;

use actix::{Actor, Addr};
use common::AHandler;
use tokio::{
    task::{spawn, JoinHandle},
    time::Duration,
};

#[cfg(test)]
use super::socket::tests::MockSocket as Socket;
#[cfg(not(test))]
use super::socket::Socket;
use super::socket::{Packet, ReceivedPacket, SocketEnd, Stream};
#[cfg(test)]
use mockall::automock;

pub struct Connection<A: AHandler<SocketEnd>, P: Packet> {
    socket: Addr<Socket<P>>,
    cancel_task: Option<JoinHandle<()>>,
    end_handler: Addr<A>,
    addr: SocketAddr,
}

const CANCEL_TIMEOUT: Duration = Duration::from_secs(120);

#[cfg_attr(test, automock)]
impl<A: AHandler<SocketEnd>, P: Packet> Connection<A, P> {
    pub fn new<B: AHandler<ReceivedPacket<P>>>(
        end_handler: Addr<A>,
        received_handler: Addr<B>,
        addr: SocketAddr,
        stream: Stream,
    ) -> Self {
        let socket = Socket::new(received_handler, end_handler.clone(), addr, stream);
        let mut this = Connection {
            socket: socket.start(),
            cancel_task: None,
            end_handler,
            addr,
        };
        this.restart_timeout();
        this
    }

    fn cancel_timeout(&mut self) {
        if let Some(task) = self.cancel_task.take() {
            task.abort();
        }
    }

    pub fn restart_timeout(&mut self) {
        self.cancel_timeout();
        let end_handler = self.end_handler.clone();
        let addr = self.addr;
        self.cancel_task = Some(spawn(async move {
            tokio::time::sleep(CANCEL_TIMEOUT).await;
            end_handler.do_send(SocketEnd { addr });
        }));
    }

    pub fn get_socket(&self) -> Addr<Socket<P>> {
        self.socket.clone()
    }
}

#[cfg(test)]
pub mod test {
    use crate::network::socket::{Packet, SocketEnd};

    use super::MockConnection as Connection;
    use common::AHandler;
    use mockall::lazy_static;
    use std::sync::{Mutex, MutexGuard};

    use super::__mock_MockConnection;

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
    pub struct Guard<A: AHandler<SocketEnd>, P: Packet> {
        pub ctx: __mock_MockConnection::__new::Context<A, P>,
        guard: MutexGuard<'static, ()>,
    }

    /// Función de utilidad para mockear la [Connection].
    pub fn connection_new_context<A: AHandler<SocketEnd> + Send, P: Packet>() -> Guard<A, P> {
        let m = get_lock(&MTX);

        let context = Connection::new_context();

        Guard {
            ctx: context,
            guard: m,
        }
    }
}
