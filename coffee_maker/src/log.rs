// use std::io::Read;

// use crate::config::FileConfig;
// pub use crate::server::{Server, ServerController};
// pub use crate::traits::Config;
// use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, Registry};

pub fn init() -> WorkerGuard {
    // let config = FileConfig::new("config.txt").expect("Error cargando la configuracion");

    let file_appender = tracing_appender::rolling::hourly("./src/logs", "logs.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .json()
                .with_thread_names(true)
                .with_writer(file_writer),
        )
        .with(
            fmt::Layer::default()
                .with_thread_names(true)
                .pretty()
                .with_writer(std::io::stdout),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    guard
}