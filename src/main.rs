use llm_supabase_rs::app::App;
use llm_supabase_rs::config::AppConfig;
use llm_supabase_rs::infrastructure::{LoggingConfig, LogFormat};
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = AppConfig::from_env()?;

    // Initialize logging with enhanced configuration
    let log_format = if cfg!(debug_assertions) {
        LogFormat::Pretty  // Colored, pretty logs for development
    } else {
        LogFormat::Json    // Structured JSON logs for production
    };

    LoggingConfig {
        level: config.log_level.clone(),
        format: log_format,
        include_location: cfg!(debug_assertions),
        trace_spans: cfg!(debug_assertions),
        ..Default::default()
    }
    .init()?;

    tracing::info!("Starting LLM Supabase Service");
    tracing::info!("Configuration loaded: host={}:{}", config.host, config.port);

    // Create the application
    let app = App::new(config).await?;

    // Set up graceful shutdown signal handling
    let shutdown_signal = shutdown_signal();

    // Run the application with graceful shutdown
    tokio::select! {
        result = app.run() => {
            if let Err(e) = result {
                tracing::error!("Application error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown_signal => {
            tracing::info!("Shutdown signal received, starting graceful shutdown...");
        }
    }

    tracing::info!("Application shutdown complete");
    Ok(())
}

/// Set up graceful shutdown signal handling
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM signal");
        },
    }
}
