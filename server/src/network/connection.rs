use std::{future::Future, net::SocketAddr};

use actix::{Actor, Addr};
#[cfg(test)]
use mockall::automock;
use tokio::{
    task::{spawn, JoinHandle},
    time::Duration,
};

use super::Packet;
#[cfg(test)]
use common::socket::test_util::{MockSocket as Socket, MockStream as Stream};
use common::socket::{ReceivedPacket, SocketEnd, SocketError, SocketSend};
#[cfg(not(test))]
use common::socket::{Socket, Stream};
use common::AHandler;

pub struct Connection<A: AHandler<SocketEnd>, P: Packet> {
    socket: Addr<Socket<P, P>>,
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

    pub fn send(&self, data: P) -> impl Future<Output = Result<(), SocketError>> {
        let socket = self.socket.clone();

        async move { socket.send(SocketSend { data }).await? }
    }
}

#[cfg(test)]
pub mod test {
    use std::sync::{Mutex, MutexGuard};

    use mockall::lazy_static;

    use super::{super::Packet, MockConnection as Connection, __mock_MockConnection};
    use common::socket::SocketEnd;
    use common::AHandler;

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
