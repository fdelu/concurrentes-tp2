use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, writer::MakeWriterExt},
    prelude::__tracing_subscriber_SubscriberExt,
    Registry,
};

/// Configuración de los logs.
/// [Level] puede ser "trace", "debug", "info", "warn" o "error".
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct LogConfig {
    /// Directorio donde se guardan los archivos de log
    pub files_directory: String,
    /// Nivel de log para archivos
    #[serde_as(as = "DisplayFromStr")]
    pub file_log_level: Level,
    /// Nivel de log para stdout
    #[serde_as(as = "DisplayFromStr")]
    pub stdout_log_level: Level,
}

pub struct LogGuard(WorkerGuard, WorkerGuard);

/// Inicializa el logger con la configuración dada.
pub fn init_logger(cfg: &LogConfig) -> LogGuard {
    let file_appender = tracing_appender::rolling::hourly(&cfg.files_directory, "logs.log");
    let (file, file_guard) = tracing_appender::non_blocking(file_appender);
    let (stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .json()
                .with_thread_names(true)
                .with_writer(file.with_max_level(cfg.file_log_level)),
        )
        .with(
            fmt::Layer::default()
                .with_thread_names(true)
                .compact()
                .with_writer(stdout.with_max_level(cfg.stdout_log_level)),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    LogGuard(file_guard, stdout_guard)
}
