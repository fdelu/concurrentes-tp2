use actix::fut::LocalBoxActorFuture;
use actix::prelude::*;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time;

use common::AHandler;
use tracing::debug;

use crate::dist_mutex::packets::{MutexPacket, RequestPacket};
use crate::dist_mutex::MutexError::Mailbox;
use crate::dist_mutex::{
    DistMutex, MutexError, MutexResult, TIME_UNTIL_DISCONNECT_POLITIC, TIME_UNTIL_ERROR,
};
use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
use crate::packet_dispatcher::messages::prune::PruneMessage;
use crate::packet_dispatcher::messages::public::die::DieMessage;
use crate::packet_dispatcher::packet::Packet;

#[derive(Message)]
#[rtype(result = "MutexResult<()>")]
pub struct AcquireMessage;

impl AcquireMessage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AcquireMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: AHandler<BroadcastMessage> + AHandler<PruneMessage> + AHandler<DieMessage>>
    Handler<AcquireMessage> for DistMutex<P>
{
    type Result = ResponseActFuture<Self, MutexResult<()>>;

    fn handle(&mut self, _: AcquireMessage, _: &mut Self::Context) -> Self::Result {
        self.clean_state();
        let packet = RequestPacket::new(self.id);
        let timestamp = packet.timestamp;

        self.dispatcher.do_send(BroadcastMessage {
            packet: Packet::Mutex(MutexPacket::Request(packet)),
        });

        self.lock_timestamp = Some(timestamp);
        self.queue.push((timestamp, self.server_id));

        self.wait_for_oks(TIME_UNTIL_DISCONNECT_POLITIC)
            .then(|r, me, _| {
                match r {
                    Ok(()) => {
                        // Lock acquired
                        debug!("[Mutex {}] I have the lock", me.id);
                        me.box_fut(Ok(()))
                    }
                    Err(MutexError::Timeout | MutexError::Disconnected) => {
                        if me.ack_received.is_empty() {
                            // We are disconnected from the network
                            debug!("[Mutex {}] We are disconnected", me.id);
                            me.dispatcher.do_send(DieMessage);
                            me.box_fut(Err(MutexError::Disconnected))
                        } else if me.ok_received == me.ack_received {
                            // There are servers that are disconnected
                            // but we have the lock
                            debug!("[Mutex {}] I have the lock", me.id);
                            me.send_prune();
                            me.box_fut(Ok(()))
                        } else {
                            // There is a server that has the lock
                            debug!("[Mutex {}] There is a server that has the lock", me.id);
                            me.wait_for_oks(TIME_UNTIL_ERROR)
                        }
                    }
                    Err(Mailbox(_)) => {
                        panic!("Mailbox error");
                    }
                }
            })
            .boxed_local()
    }
}

impl<P: Actor> DistMutex<P> {
    fn box_fut<T>(&mut self, inner: T) -> LocalBoxActorFuture<Self, T>
    where
        T: 'static,
    {
        async move { inner }.into_actor(self).boxed_local()
    }
}

impl<P: AHandler<BroadcastMessage>> DistMutex<P> {
    fn wait_for_oks(
        &mut self,
        dur: Duration,
    ) -> LocalBoxActorFuture<DistMutex<P>, Result<(), MutexError>> {
        debug!("{} Waiting {} ms for lock", self, dur.as_millis());
        let (tx, rx) = oneshot::channel();
        self.all_oks_received_channel = Some(tx);
        async move {
            if time::timeout(dur, rx).await.is_err() {
                debug!("Timeout while waiting for oks");
                Err(MutexError::Timeout)
            } else {
                debug!("All oks received");
                Ok(())
            }
        }
        .into_actor(self)
        .boxed_local()
    }
}

#[cfg(test)]
#[allow(unused_must_use, clippy::type_complexity)]
mod tests {
    use std::collections::HashSet;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::{Arc, Mutex};

    use actix::prelude::*;

    use crate::dist_mutex::messages::ack::AckMessage;
    use crate::dist_mutex::messages::ok::OkMessage;
    use crate::dist_mutex::server_id::ServerId;
    use crate::dist_mutex::{DistMutex, MutexCreationTrait, MutexError};
    use crate::packet_dispatcher::messages::broadcast::BroadcastMessage;
    use crate::packet_dispatcher::messages::prune::PruneMessage;
    use crate::packet_dispatcher::messages::public::die::DieMessage;
    use crate::packet_dispatcher::packet::Packet;
    use crate::AcquireMessage;
    use common::socket::SocketError;

    struct TestDispatcher {
        pub broadcasts: Arc<Mutex<Vec<BroadcastMessage>>>,
        pub prunes: Arc<Mutex<Vec<PruneMessage>>>,
    }

    impl Actor for TestDispatcher {
        type Context = Context<Self>;
    }

    impl Handler<BroadcastMessage> for TestDispatcher {
        type Result = ResponseActFuture<Self, Result<(), SocketError>>;

        fn handle(&mut self, msg: BroadcastMessage, _: &mut Self::Context) -> Self::Result {
            self.broadcasts.lock().unwrap().push(msg);
            async { Ok(()) }.into_actor(self).boxed_local()
        }
    }

    impl Handler<PruneMessage> for TestDispatcher {
        type Result = ();

        fn handle(&mut self, msg: PruneMessage, _: &mut Self::Context) -> Self::Result {
            self.prunes.lock().unwrap().push(msg);
        }
    }

    impl Handler<DieMessage> for TestDispatcher {
        type Result = ();

        fn handle(&mut self, _: DieMessage, ctx: &mut Self::Context) -> Self::Result {
            ctx.stop();
        }
    }

    fn create_mutex() -> (
        Addr<DistMutex<TestDispatcher>>,
        Arc<Mutex<Vec<BroadcastMessage>>>,
        Arc<Mutex<Vec<PruneMessage>>>,
    ) {
        let broadcasts = Arc::new(Mutex::new(Vec::new()));
        let prunes = Arc::new(Mutex::new(Vec::new()));
        let dispatcher = TestDispatcher {
            broadcasts: broadcasts.clone(),
            prunes: prunes.clone(),
        };
        let dispatcher_addr = dispatcher.start();
        let resource_id = 1;
        let server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let mutex = DistMutex::new(server_id, resource_id, dispatcher_addr);
        let mutex_addr = mutex.start();
        (mutex_addr, broadcasts, prunes)
    }

    #[actix_rt::test]
    async fn test_acquire_sends_broadcast_to_dispatcher() {
        let (mutex, dispatcher, _) = create_mutex();
        mutex.send(AcquireMessage::new()).await;

        let broadcasts = dispatcher.lock().unwrap();
        assert_eq!(broadcasts.len(), 1);
        if let Packet::Mutex(_) = broadcasts[0].packet {
        } else {
            panic!("Wrong packet type");
        }

        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_without_any_ack_received_returns_disconnected() {
        let (mutex, _, _) = create_mutex();
        let result = mutex.send(AcquireMessage::new()).await.unwrap();

        assert_eq!(result.unwrap_err(), MutexError::Disconnected);
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_with_all_acks_received_returns_ok() {
        let (mutex, _, _) = create_mutex();
        let another_server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)));

        let ack = AckMessage{from: another_server_id};
        let connected_servers = HashSet::from([another_server_id]);
        let ok = OkMessage{from: another_server_id, connected_servers};

        let result = mutex.send(AcquireMessage::new());
        mutex.do_send(ack);
        mutex.do_send(ok);

        assert_eq!(result.await.unwrap(), Ok(()));
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_with_ack_but_no_ok_returns_timeout() {
        let (mutex, _, _) = create_mutex();
        let another_server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        let ack = AckMessage{from: another_server_id};

        let result = mutex.send(AcquireMessage::new());
        mutex.do_send(ack);

        assert_eq!(result.await.unwrap().unwrap_err(), MutexError::Timeout);
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_with_oks_received_from_all_servers_that_sent_ack_means_i_have_the_lock() {
        let (mutex, _, _) = create_mutex();
        let server_id_1 = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let server_id_2 = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        let connected_servers = HashSet::from([server_id_1, server_id_2]);

        let ack = AckMessage{from: server_id_1};
        let ok = OkMessage{from: server_id_1, connected_servers};

        let result = mutex.send(AcquireMessage::new());
        mutex.do_send(ack);
        mutex.do_send(ok);

        assert_eq!(result.await.unwrap(), Ok(()));
    }

    #[actix_rt::test]
    async fn test_acquire_with_timeout_but_lock_acquired_sends_prune_to_dispatcher() {
        let (mutex, _, prunes) = create_mutex();
        let server_id_1 = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let server_id_2 = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        let connected_servers = HashSet::from([server_id_1, server_id_2]);

        let ack = AckMessage{from: server_id_1};
        let ok = OkMessage{from: server_id_1, connected_servers};

        let result = mutex.send(AcquireMessage::new());
        mutex.do_send(ack);
        mutex.do_send(ok);

        result.await.unwrap();

        let prunes = prunes.lock().unwrap();
        assert_eq!(prunes.len(), 1);
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_without_timeout_does_not_send_prune_to_dispatcher() {
        let (mutex, _, _) = create_mutex();
        let another_server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)));

        let ack = AckMessage{from: another_server_id};
        let connected_servers = HashSet::from([another_server_id]);
        let ok = OkMessage{from: another_server_id, connected_servers};

        let result = mutex.send(AcquireMessage::new());
        mutex.do_send(ack);
        mutex.do_send(ok);

        assert_eq!(result.await.unwrap(), Ok(()));
        System::current().stop();
    }
}
