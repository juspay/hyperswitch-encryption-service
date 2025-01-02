use serde::Deserialize;

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

#[derive(Deserialize, Debug, Clone)]
pub struct LogConfig {
    pub log_level: LogLevel,
    pub log_format: LogFormat,
    pub filtering_directive: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Console,
    Json,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
            LogLevel::Off => Self::OFF,
        }
    }
}

pub(super) struct LogGuard {
    _log_guard: WorkerGuard,
}

#[derive(Clone)]
pub struct OnRequest {
    level: LogLevel,
}

impl OnRequest {
    pub fn with_level(level: LogLevel) -> Self {
        Self { level }
    }
}

impl<B> tower_http::trace::OnRequest<B> for OnRequest {
    fn on_request(&mut self, _: &hyper::Request<B>, _: &tracing::Span) {
        match self.level {
            LogLevel::Debug => {
                tracing::event!(tracing::Level::DEBUG, "Started processing request");
            }
            LogLevel::Warn => {
                tracing::event!(tracing::Level::WARN, "Started processing request");
            }
            LogLevel::Error => {
                tracing::event!(tracing::Level::ERROR, "Started processing request");
            }
            LogLevel::Info => {
                tracing::event!(tracing::Level::INFO, "Started processing request");
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct OnResponse {
    level: LogLevel,
}

impl OnResponse {
    pub fn with_level(level: LogLevel) -> Self {
        Self { level }
    }
}

impl<B> tower_http::trace::OnResponse<B> for OnResponse {
    fn on_response(
        self,
        response: &hyper::Response<B>,
        latency: std::time::Duration,
        _: &tracing::Span,
    ) {
        let status = response.status().as_u16();
        let latency = latency.as_micros();

        match self.level {
            LogLevel::Debug => {
                tracing::event!(
                    tracing::Level::DEBUG,
                    status,
                    latency,
                    "Finished processing request"
                );
            }
            LogLevel::Warn => {
                tracing::event!(
                    tracing::Level::WARN,
                    status,
                    latency,
                    "Finished processing request"
                );
            }
            LogLevel::Error => {
                tracing::event!(
                    tracing::Level::ERROR,
                    status,
                    latency,
                    "Finished processing request"
                );
            }
            LogLevel::Info => {
                tracing::event!(
                    tracing::Level::INFO,
                    status,
                    latency,
                    "Finished processing request"
                );
            }
            _ => {}
        }
    }
}

pub(super) fn setup_logging_pipeline(
    log_config: &LogConfig,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> LogGuard {
    let subscriber = tracing_subscriber::registry();

    let console_filter = get_envfilter(
        log_config.filtering_directive.as_ref(),
        LogLevel::Warn,
        log_config.log_level,
        &crates_to_filter,
    );

    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

    match log_config.log_format {
        LogFormat::Console => {
            let logging_layer = fmt::layer()
                .with_timer(fmt::time::time())
                .pretty()
                .with_writer(non_blocking)
                .with_filter(console_filter);

            subscriber.with(logging_layer).init();
        }
        LogFormat::Json => {
            let logging_layer = fmt::layer()
                .json()
                .with_timer(fmt::time::time())
                .with_writer(non_blocking)
                .with_filter(console_filter);

            subscriber.with(logging_layer).init();
        }
    }

    LogGuard { _log_guard: guard }
}

#[macro_export]
macro_rules! workspace_members {
    () => {
        std::env!("CARGO_WORKSPACE_MEMBERS")
            .split(",")
            .collect::<std::collections::HashSet<&'static str>>()
    };
}

fn get_envfilter<'a>(
    filtering_directive: Option<&String>,
    default_log_level: impl Into<LevelFilter> + Copy,
    filter_log_level: impl Into<LevelFilter> + Copy,
    crates_to_filter: impl AsRef<[&'a str]>,
) -> EnvFilter {
    filtering_directive
        .map(|filter| {
            // Try to create target filter from specified filtering directive, if set

            // Safety: If user is overriding the default filtering directive, then we need to panic
            // for invalid directives.
            #[allow(clippy::expect_used)]
            EnvFilter::builder()
                .with_default_directive(default_log_level.into().into())
                .parse(filter)
                .expect("Invalid EnvFilter filtering directive")
        })
        .unwrap_or_else(|| {
            // Construct a default target filter otherwise
            let mut workspace_members = workspace_members!();
            workspace_members.extend(crates_to_filter.as_ref());

            workspace_members
                .drain()
                .zip(std::iter::repeat(filter_log_level.into()))
                .fold(
                    EnvFilter::default().add_directive(default_log_level.into().into()),
                    |env_filter, (target, level)| {
                        // Safety: This is a hardcoded basic filtering directive. If even the basic
                        // filter is wrong, it's better to panic.
                        #[allow(clippy::expect_used)]
                        env_filter.add_directive(
                            format!("{target}={level}")
                                .parse()
                                .expect("Invalid EnvFilter directive format"),
                        )
                    },
                )
        })
}
