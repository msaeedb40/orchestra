//! Structured logging and tracing subscriber initialization.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Logging output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-friendly colored terminal format.
    #[default]
    Pretty,
    /// Compact single-line output.
    Compact,
    /// Structured JSON format for machine ingestion and daemons.
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "compact" => Ok(Self::Compact),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "Unknown log format '{}'. Valid: pretty, compact, json",
                other
            )),
        }
    }
}

/// Tracing and logging configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Minimum log level filter (trace, debug, info, warn, error).
    pub level: String,
    /// Output formatting style.
    pub format: LogFormat,
    /// Include file and line numbers in logs.
    pub with_file: bool,
    /// Include span enter and exit events.
    pub with_spans: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            with_file: false,
            with_spans: false,
        }
    }
}

/// Initializes global tracing subscriber for the process.
/// Can be safely called once during CLI or Daemon startup.
pub fn init_tracing(config: &LogConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let span_events = if config.with_spans {
        FmtSpan::ENTER | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    let registry = tracing_subscriber::registry().with(env_filter);

    match config.format {
        LogFormat::Json => {
            let json_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_file(config.with_file)
                .with_line_number(config.with_file)
                .with_span_events(span_events);
            registry.with(json_layer).try_init()?;
        }
        LogFormat::Compact => {
            let compact_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_file(config.with_file)
                .with_line_number(config.with_file)
                .with_span_events(span_events);
            registry.with(compact_layer).try_init()?;
        }
        LogFormat::Pretty => {
            let pretty_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_file(config.with_file)
                .with_line_number(config.with_file)
                .with_span_events(span_events);
            registry.with(pretty_layer).try_init()?;
        }
    }

    Ok(())
}

/// Helper to parse a standard level string into `tracing::Level`.
pub fn parse_log_level(s: &str) -> Level {
    match s.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_format_parsing() {
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("compact".parse::<LogFormat>().unwrap(), LogFormat::Compact);
        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("debug"), Level::DEBUG);
        assert_eq!(parse_log_level("TRACE"), Level::TRACE);
        assert_eq!(parse_log_level("warning"), Level::WARN);
        assert_eq!(parse_log_level("error"), Level::ERROR);
        assert_eq!(parse_log_level("unknown"), Level::INFO);
    }
}
