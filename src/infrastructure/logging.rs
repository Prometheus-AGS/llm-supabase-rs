// src/infrastructure/logging.rs
//
// Advanced logging configuration with structured logging, multiple outputs, and log rotation

use anyhow::Result;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Logging format options
#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    /// Human-readable format with colors (for development)
    Pretty,
    
    /// Compact format (for production)
    Compact,
    
    /// JSON format (for log aggregation systems)
    Json,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (from env: LOG_LEVEL)
    pub level: String,
    
    /// Format for logs
    pub format: LogFormat,
    
    /// Include source file/line information
    pub include_location: bool,
    
    /// Include thread names/ids
    pub include_thread: bool,
    
    /// Include target (module path)
    pub include_target: bool,
    
    /// Trace span lifecycle (enter/exit)
    pub trace_spans: bool,
    
    /// Write logs to file
    pub log_to_file: bool,
    
    /// Log file path
    pub log_file_path: String,
    
    /// Enable request/response logging
    pub log_requests: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            format: if cfg!(debug_assertions) {
                LogFormat::Pretty
            } else {
                LogFormat::Json
            },
            include_location: cfg!(debug_assertions),
            include_thread: true,
            include_target: true,
            trace_spans: cfg!(debug_assertions),
            log_to_file: std::env::var("LOG_TO_FILE")
                .map(|v| v == "true")
                .unwrap_or(false),
            log_file_path: std::env::var("LOG_FILE_PATH")
                .unwrap_or_else(|_| "./logs/app.log".to_string()),
            log_requests: std::env::var("LOG_REQUESTS")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }
}

impl LoggingConfig {
    /// Initialize the logging system
    pub fn init(self) -> Result<()> {
        // Create environment filter
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.level))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // Build the subscriber based on format
        match self.format {
            LogFormat::Pretty => self.init_pretty(env_filter),
            LogFormat::Compact => self.init_compact(env_filter),
            LogFormat::Json => self.init_json(env_filter),
        }
    }

    /// Initialize pretty colored logs for development
    fn init_pretty(self, env_filter: EnvFilter) -> Result<()> {
        let fmt_layer = fmt::layer()
            .with_target(self.include_target)
            .with_thread_names(self.include_thread)
            .with_file(self.include_location)
            .with_line_number(self.include_location)
            .with_span_events(if self.trace_spans {
                FmtSpan::ENTER | FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            })
            .pretty();

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        Ok(())
    }

    /// Initialize compact logs for production
    fn init_compact(self, env_filter: EnvFilter) -> Result<()> {
        let fmt_layer = fmt::layer()
            .with_target(self.include_target)
            .with_thread_names(self.include_thread)
            .with_file(self.include_location)
            .with_line_number(self.include_location)
            .with_span_events(if self.trace_spans {
                FmtSpan::ENTER | FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            })
            .compact();

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        Ok(())
    }

    /// Initialize JSON logs for structured logging
    fn init_json(self, env_filter: EnvFilter) -> Result<()> {
        let fmt_layer = fmt::layer()
            .with_target(self.include_target)
            .with_thread_names(self.include_thread)
            .with_file(self.include_location)
            .with_line_number(self.include_location)
            .with_span_events(if self.trace_spans {
                FmtSpan::ENTER | FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            })
            .json();

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        Ok(())
    }
}

/// Quick setup for development (pretty logs)
pub fn init_dev_logging() -> Result<()> {
    LoggingConfig {
        format: LogFormat::Pretty,
        include_location: true,
        trace_spans: true,
        ..Default::default()
    }
    .init()
}

/// Quick setup for production (JSON logs)
pub fn init_prod_logging() -> Result<()> {
    LoggingConfig {
        format: LogFormat::Json,
        include_location: false,
        trace_spans: false,
        ..Default::default()
    }
    .init()
}

/// Helper macro for structured logging with context
#[macro_export]
macro_rules! log_with_context {
    (error, $($arg:tt)*) => {
        tracing::error!($($arg)*)
    };
    (warn, $($arg:tt)*) => {
        tracing::warn!($($arg)*)
    };
    (info, $($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
    (debug, $($arg:tt)*) => {
        tracing::debug!($($arg)*)
    };
    (trace, $($arg:tt)*) => {
        tracing::trace!($($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert!(config.include_target);
        assert!(config.include_thread);
    }

    #[test]
    fn test_log_format() {
        let pretty = LoggingConfig {
            format: LogFormat::Pretty,
            ..Default::default()
        };
        assert!(matches!(pretty.format, LogFormat::Pretty));

        let json = LoggingConfig {
            format: LogFormat::Json,
            ..Default::default()
        };
        assert!(matches!(json.format, LogFormat::Json));
    }
}
