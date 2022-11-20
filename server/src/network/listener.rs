use std::io;

use actix::Recipient;
#[cfg(test)]
use common::socket::test_util::MockTcpListener as TcpListener;
#[cfg(not(test))]
use tokio::net::TcpListener;
use tokio::{
    net::ToSocketAddrs,
    task::{spawn, JoinHandle},
};
use tracing::{debug, error, trace};

use super::AddStream;
use common::socket::SocketError;
#[cfg(test)]
use mockall::automock;

/// Loop que acepta conexiones de un [TcpListener] y
/// las envía como mensaje a un Actor dado.
pub struct Listener {
    listener: TcpListener,
}

#[cfg_attr(test, automock)]
impl Listener {
    /// Crea un nuevo Listener. Argumentos:
    /// - `addr`: Dirección en la cual escuchar.
    pub(crate) async fn bind<A: ToSocketAddrs + 'static>(addr: A) -> io::Result<Self> {
        trace!("Binding listener...");
        let listener = TcpListener::bind(addr).await?;
        trace!("Bound to {:?}", listener.local_addr());
        Ok(Self { listener })
    }

    async fn add_connection(
        listener: &mut TcpListener,
        handler: &Recipient<AddStream>,
    ) -> Result<(), SocketError> {
        let (stream, addr) = listener.accept().await?;
        trace!("Accepted connection from {}", addr);
        handler.send(AddStream { stream, addr }).await?;
        Ok(())
    }

    /// Inicia el loop de aceptar conexiones.
    /// Argumentos:
    /// - `handler`: Actor al cual enviar los [TcpStream](tokio::net::TcpStream).
    pub(crate) fn run(mut self, add_handler: Recipient<AddStream>) -> JoinHandle<()> {
        spawn(async move {
            debug!(
                "Listening for connections ({:?})",
                self.listener.local_addr()
            );
            loop {
                if let Err(e) = Self::add_connection(&mut self.listener, &add_handler).await {
                    error!("Error accepting connection: {e}");
                }
            }
        })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        debug!("Dropping listener");
    }
}

#[cfg(test)]
pub mod test {
    use super::{MockListener as Listener, __mock_MockListener};
    use mockall::lazy_static;
    use std::sync::{Mutex, MutexGuard};

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

    /// Guard de [listener_new_context]. Contiene el contexto del mock y el guard del mutex
    /// estático que impide que se inicialice el mock en varios tests a la vez.
    pub(crate) struct Guard {
        pub ctx: __mock_MockListener::__bind::Context,
        guard: MutexGuard<'static, ()>,
    }

    /// Función de utilidad para mockear el [Listener].
    pub(crate) fn listener_new_context() -> Guard {
        let m = get_lock(&MTX);

        let context = Listener::bind_context();

        Guard {
            ctx: context,
            guard: m,
        }
    }
}
