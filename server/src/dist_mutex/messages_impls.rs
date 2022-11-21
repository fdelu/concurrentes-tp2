use crate::dist_mutex::messages::{
    AckMessage, AcquireMessage, DoWithLock, OkMessage, ReleaseMessage, RequestMessage,
};
use crate::dist_mutex::packets::{AckPacket, MutexPacket, OkPacket, RequestPacket};
use crate::dist_mutex::MutexError::Mailbox;
use crate::dist_mutex::{
    do_send, DistMutex, MutexError, MutexResult, TIME_UNTIL_DISCONNECT_POLITIC, TIME_UNTIL_ERROR,
};
use crate::packet_dispatcher::messages::BroadcastMessage;
use crate::packet_dispatcher::messages::DieMessage;
use crate::packet_dispatcher::messages::PruneMessage;
use crate::packet_dispatcher::messages::SendMessage;
use crate::packet_dispatcher::packet::Packet;
use crate::ServerId;
use actix::prelude::*;
use common::AHandler;
use std::future::Future;
use tracing::{debug, info};

impl<P: AHandler<SendMessage>> Handler<ReleaseMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, _: ReleaseMessage, _: &mut Self::Context) -> Self::Result {
        let id = self.id;
        let ok = OkPacket { id };

        for (_, server) in &self.queue {
            if *server != self.server_id {
                do_send(&self.dispatcher, *server, MutexPacket::Ok(ok));
            }
        }
        self.clean_state();
    }
}

impl<P: Actor> Handler<AckMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: AckMessage, _ctx: &mut Self::Context) {
        self.ack_received.insert(msg.from);
    }
}

impl<P: Actor> Handler<OkMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: OkMessage, _ctx: &mut Self::Context) {
        debug!("{} Received ok from {}", self, msg.from);
        info!("Connected servers: {:?}", msg.connected_servers);

        self.ok_received.insert(msg.from);

        if self.ok_received.is_superset(&msg.connected_servers) {
            debug!("{} All ok received", self);
            if let Some(ch) = self.all_oks_received_channel.take() {
                ch.send(()).unwrap();
            }
        }
    }
}

impl<P: AHandler<SendMessage>> Handler<RequestMessage> for DistMutex<P> {
    type Result = ();

    fn handle(&mut self, msg: RequestMessage, _ctx: &mut Self::Context) {
        do_send(
            &self.dispatcher,
            msg.from,
            MutexPacket::Ack(AckPacket { id: self.id }),
        );

        if let Some(my_timestamp) = &self.lock_timestamp {
            if my_timestamp > &msg.timestamp {
                debug!("{} {:?} has priority over me, sending ok", self, msg.from);
                do_send(
                    &self.dispatcher,
                    msg.from,
                    MutexPacket::Ok(OkPacket { id: self.id }),
                );
            } else {
                debug!(
                    "{} I have priority over {} (my timestamp {} - other timestamp: {})",
                    self, msg.from, my_timestamp, msg.timestamp
                );
                self.queue.push((msg.timestamp, msg.from));
            }
        } else {
            debug!(
                "{} I am not waiting for lock, sending ok to {}",
                self, msg.from
            );
            do_send(
                &self.dispatcher,
                msg.from,
                MutexPacket::Ok(OkPacket { id: self.id }),
            );
        }
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

impl<F, P, R, Fut> Handler<DoWithLock<F, R, Fut>> for DistMutex<P>
where
    F: FnOnce() -> Fut + Send + 'static,
    P: AHandler<BroadcastMessage>
        + AHandler<SendMessage>
        + AHandler<PruneMessage>
        + AHandler<DieMessage>,
    Fut: Future<Output = R>,
    R: Send + 'static,
{
    type Result = ResponseActFuture<Self, MutexResult<R>>;

    fn handle(&mut self, msg: DoWithLock<F, R, Fut>, ctx: &mut Self::Context) -> Self::Result {
        let addr = ctx.address();

        async move {
            if addr.send(AcquireMessage {}).await.is_err() {
                return Err(MutexError::Timeout);
            };
            let r = (msg.action)().await;
            if addr.send(ReleaseMessage {}).await.is_err() {
                return Err(MutexError::Timeout);
            };
            Ok(r)
        }
        .into_actor(self)
        .boxed_local()
    }
}

impl RequestMessage {
    pub fn new(from: ServerId, packet: RequestPacket) -> Self {
        Self {
            from,
            timestamp: packet.timestamp,
        }
    }
}

#[cfg(test)]
#[allow(unused_must_use, clippy::type_complexity)]
mod tests {
    use std::collections::HashSet;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::{Arc, Mutex};

    use actix::prelude::*;

    use crate::dist_mutex::messages::AckMessage;
    use crate::dist_mutex::messages::AcquireMessage;
    use crate::dist_mutex::messages::OkMessage;
    use crate::server_id::ServerId;
    use crate::dist_mutex::{DistMutex, MutexCreationTrait, MutexError};
    use crate::packet_dispatcher::messages::BroadcastMessage;
    use crate::packet_dispatcher::messages::DieMessage;
    use crate::packet_dispatcher::messages::PruneMessage;
    use crate::packet_dispatcher::packet::Packet;
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
        mutex.send(AcquireMessage {}).await;

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
        let result = mutex.send(AcquireMessage {}).await.unwrap();

        assert_eq!(result.unwrap_err(), MutexError::Disconnected);
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_with_all_acks_received_returns_ok() {
        let (mutex, _, _) = create_mutex();
        let another_server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)));

        let ack = AckMessage {
            from: another_server_id,
        };
        let connected_servers = HashSet::from([another_server_id]);
        let ok = OkMessage {
            from: another_server_id,
            connected_servers,
        };

        let result = mutex.send(AcquireMessage {});
        mutex.do_send(ack);
        mutex.do_send(ok);

        assert_eq!(result.await.unwrap(), Ok(()));
        System::current().stop();
    }

    #[actix_rt::test]
    async fn test_acquire_with_ack_but_no_ok_returns_timeout() {
        let (mutex, _, _) = create_mutex();
        let another_server_id = ServerId::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        let ack = AckMessage {
            from: another_server_id,
        };

        let result = mutex.send(AcquireMessage {});
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

        let ack = AckMessage { from: server_id_1 };
        let ok = OkMessage {
            from: server_id_1,
            connected_servers,
        };

        let result = mutex.send(AcquireMessage {});
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

        let ack = AckMessage { from: server_id_1 };
        let ok = OkMessage {
            from: server_id_1,
            connected_servers,
        };

        let result = mutex.send(AcquireMessage {});
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

        let ack = AckMessage {
            from: another_server_id,
        };
        let connected_servers = HashSet::from([another_server_id]);
        let ok = OkMessage {
            from: another_server_id,
            connected_servers,
        };

        let result = mutex.send(AcquireMessage {});
        mutex.do_send(ack);
        mutex.do_send(ok);

        assert_eq!(result.await.unwrap(), Ok(()));
        System::current().stop();
    }
}
