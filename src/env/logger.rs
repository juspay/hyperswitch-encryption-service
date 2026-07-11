use std::collections::{HashMap, HashSet};

use log_utils::{
    AdditionalFieldsPlacement, ConsoleLogFormat, ConsoleLoggingConfig, DirectivePrintTarget,
    LoggerConfig, LoggerError,
};
use serde::Deserialize;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize, Debug, Clone)]
pub struct LogConfig {
    pub enabled: bool,
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
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Console,
    Json,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
        }
    }
}

fn get_envfilter_directive(
    default_log_level: tracing::Level,
    filter_log_level: tracing::Level,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> String {
    let mut workspace_members = build_info::cargo_workspace_members!();
    workspace_members.extend(build_info::framework_libs_workspace_members());
    workspace_members.extend(crates_to_filter.as_ref());

    workspace_members
        .into_iter()
        .zip(std::iter::repeat(filter_log_level))
        .fold(
            vec![default_log_level.to_string()],
            |mut directives, (target, level)| {
                directives.push(format!("{target}={level}"));
                directives
            },
        )
        .join(",")
}

fn get_logger_config(
    config: &LogConfig,
    service_name: &'static str,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> LoggerConfig {
    let console_config = if config.enabled {
        let console_filtering_directive = config.filtering_directive.clone().unwrap_or_else(|| {
            get_envfilter_directive(
                tracing::Level::WARN,
                tracing::Level::from(config.log_level),
                crates_to_filter,
            )
        });

        let log_format = match config.log_format {
            LogFormat::Console => ConsoleLogFormat::HumanReadable,
            LogFormat::Json => {
                error_stack::Report::set_color_mode(error_stack::fmt::ColorMode::None);
                ConsoleLogFormat::CompactJson
            }
        };

        Some(ConsoleLoggingConfig {
            level: tracing::Level::from(config.log_level),
            log_format,
            filtering_directive: Some(console_filtering_directive),
            print_filtering_directive: DirectivePrintTarget::Stdout,
        })
    } else {
        None
    };

    LoggerConfig {
        static_top_level_fields: HashMap::from([(
            "service".to_string(),
            serde_json::json!(service_name),
        )]),
        top_level_keys: HashSet::new(),
        persistent_keys: HashSet::new(),
        log_span_lifecycles: false,
        additional_fields_placement: AdditionalFieldsPlacement::TopLevel,
        file_config: None,
        console_config,
        global_filtering_directive: None,
    }
}

pub struct LogGuard {
    _log_guards: Vec<WorkerGuard>,
}

pub fn setup(
    config: &LogConfig,
    service_name: &'static str,
    crates_to_filter: impl AsRef<[&'static str]>,
) -> Result<LogGuard, LoggerError> {
    let logger_config = get_logger_config(config, service_name, crates_to_filter);

    let components = log_utils::build_logging_components(logger_config)?;

    let mut layers: Vec<Box<dyn Layer<_> + Send + Sync>> = Vec::new();
    layers.push(components.storage_layer.boxed());

    if let Some(console_layer) = components.console_log_layer {
        layers.push(console_layer);
    }

    tracing_subscriber::registry().with(layers).init();

    Ok(LogGuard {
        _log_guards: components.guards,
    })
}
