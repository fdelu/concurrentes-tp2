use std::io;

use actix::Addr;
#[cfg(not(test))]
use actix_rt::net::TcpListener;
#[cfg(test)]
use common::socket::test_util::MockTcpListener as TcpListener;
use common::AHandler;
use tokio::{
    net::ToSocketAddrs,
    task::{spawn, JoinHandle},
};

use super::AddStream;
use common::socket::SocketError;
#[cfg(test)]
use mockall::automock;

pub struct Listener {
    listener: TcpListener,
}

#[cfg_attr(test, automock)]
impl Listener {
    pub async fn bind<A: ToSocketAddrs + 'static>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    async fn add_connection<A: AHandler<AddStream>>(
        listener: &mut TcpListener,
        handler: &Addr<A>,
    ) -> Result<(), SocketError> {
        let (stream, addr) = listener.accept().await?;
        handler.send(AddStream { stream, addr }).await?;
        Ok(())
    }

    pub fn run<A: AHandler<AddStream>>(mut self, add_handler: Addr<A>) -> JoinHandle<()> {
        spawn(async move {
            loop {
                if let Err(e) = Self::add_connection(&mut self.listener, &add_handler).await {
                    eprintln!("Error accepting connection: {e}");
                }
            }
        })
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
    pub struct Guard {
        pub ctx: __mock_MockListener::__bind::Context,
        guard: MutexGuard<'static, ()>,
    }

    /// Función de utilidad para mockear el [Listener].
    pub fn listener_new_context() -> Guard {
        let m = get_lock(&MTX);

        let context = Listener::bind_context();

        Guard {
            ctx: context,
            guard: m,
        }
    }
}
