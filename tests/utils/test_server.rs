//! Test server utilities for integration testing
//! 
//! This module provides utilities for starting and managing test server instances
//! with different configurations for comprehensive integration testing.

use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

use llm_supabase_rs::{
    app::create_app,
    config::app::AppConfig,
};

/// Test server instance
pub struct TestServer {
    pub addr: SocketAddr,
    pub base_url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

/// Configuration for test server
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    pub port: Option<u16>,
    pub enable_auth: bool,
    pub enable_conversations: bool,
    pub enable_provider_fallback: bool,
    pub primary_provider: String,
    pub fallback_providers: Vec<String>,
    pub database_url: Option<String>,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            port: None, // Will use available port
            enable_auth: false, // Disable for testing
            enable_conversations: true,
            enable_provider_fallback: true,
            primary_provider: "vertex".to_string(),
            fallback_providers: vec!["groq".to_string()],
            database_url: None, // Will use in-memory for tests
        }
    }
}

impl TestServer {
    /// Start a new test server with default configuration
    pub async fn start() -> Result<Self> {
        Self::start_with_config(TestServerConfig::default()).await
    }

    /// Start a test server with custom configuration
    pub async fn start_with_config(config: TestServerConfig) -> Result<Self> {
        // Create test app configuration
        let app_config = create_test_app_config(&config)?;
        
        // Create the application router
        let app = create_test_app(app_config).await?;

        // Bind to available port
        let addr = if let Some(port) = config.port {
            format!("127.0.0.1:{}", port)
        } else {
            "127.0.0.1:0".to_string()
        };

        let listener = TcpListener::bind(&addr).await?;
        let bound_addr = listener.local_addr()?;
        let base_url = format!("http://{}", bound_addr);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // Start the server
        let handle = tokio::spawn(async move {
            let server = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                });

            if let Err(e) = server.await {
                eprintln!("Test server error: {}", e);
            }
        });

        // Wait a moment for server to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(TestServer {
            addr: bound_addr,
            base_url,
            shutdown_tx: Some(shutdown_tx),
            handle: Some(handle),
        })
    }

    /// Get the server base URL
    pub fn url(&self) -> &str {
        &self.base_url
    }

    /// Get the server address
    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Check if server is healthy
    pub async fn is_healthy(&self) -> bool {
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", self.base_url);
        
        match timeout(Duration::from_secs(5), client.get(&health_url).send()).await {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false,
        }
    }

    /// Wait for server to be ready
    pub async fn wait_for_ready(&self, max_wait_seconds: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let max_duration = Duration::from_secs(max_wait_seconds);

        while start.elapsed() < max_duration {
            if self.is_healthy().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(anyhow::anyhow!("Server did not become ready within {} seconds", max_wait_seconds))
    }

    /// Shutdown the test server
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.handle.take() {
            match timeout(Duration::from_secs(10), handle).await {
                Ok(result) => {
                    if let Err(e) = result {
                        eprintln!("Test server shutdown error: {}", e);
                    }
                }
                Err(_) => {
                    eprintln!("Test server shutdown timeout");
                }
            }
        }

        Ok(())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Create test app configuration
fn create_test_app_config(config: &TestServerConfig) -> Result<AppConfig> {
    let mut app_config = AppConfig::default();

    // Set test-specific configurations
    app_config.host = "127.0.0.1".to_string();
    app_config.port = config.port.unwrap_or(0);
    
    // Disable authentication for testing unless specifically enabled
    if !config.enable_auth {
        app_config.jwt_secret = Some("test-secret-key".to_string());
        app_config.supabase_url = None;
        app_config.supabase_anon_key = None;
    }

    // Configure database for testing
    if let Some(ref db_url) = config.database_url {
        app_config.database_url = Some(db_url.clone());
    } else {
        // Use in-memory database for tests
        app_config.database_url = Some("sqlite::memory:".to_string());
    }

    // Configure providers for testing
    app_config.vertex_project_id = Some("test-project".to_string());
    app_config.vertex_location = Some("us-central1".to_string());
    
    Ok(app_config)
}

/// Create test application with mocked dependencies
async fn create_test_app(config: AppConfig) -> Result<Router> {
    // This would normally call the main app creation function
    // but with test-specific overrides for providers, database, etc.
    
    // For now, create a basic router that matches the main app structure
    let app = create_app(config).await?;
    
    Ok(app)
}

/// Test server manager for managing multiple server instances
pub struct TestServerManager {
    servers: Vec<TestServer>,
}

impl TestServerManager {
    /// Create a new server manager
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    /// Start a server with given configuration
    pub async fn start_server(&mut self, config: TestServerConfig) -> Result<&TestServer> {
        let server = TestServer::start_with_config(config).await?;
        self.servers.push(server);
        Ok(self.servers.last().unwrap())
    }

    /// Start multiple servers for load balancing tests
    pub async fn start_multiple_servers(&mut self, count: usize) -> Result<Vec<&TestServer>> {
        let mut servers = Vec::new();
        
        for i in 0..count {
            let config = TestServerConfig {
                port: Some(3000 + i as u16),
                ..Default::default()
            };
            
            let server = self.start_server(config).await?;
            servers.push(server);
        }
        
        Ok(servers)
    }

    /// Get all server URLs
    pub fn get_server_urls(&self) -> Vec<String> {
        self.servers.iter().map(|s| s.base_url.clone()).collect()
    }

    /// Shutdown all servers
    pub async fn shutdown_all(self) -> Result<()> {
        for server in self.servers {
            server.shutdown().await?;
        }
        Ok(())
    }
}

/// Utility functions for common test scenarios
pub struct TestScenarios;

impl TestScenarios {
    /// Create a server configured for Codex CLI testing
    pub async fn codex_cli_server() -> Result<TestServer> {
        let config = TestServerConfig {
            enable_conversations: true,
            enable_provider_fallback: false, // Single provider for predictable testing
            primary_provider: "vertex".to_string(),
            ..Default::default()
        };
        
        TestServer::start_with_config(config).await
    }

    /// Create a server configured for multi-turn conversation testing
    pub async fn conversation_server() -> Result<TestServer> {
        let config = TestServerConfig {
            enable_conversations: true,
            enable_provider_fallback: false,
            database_url: Some("sqlite::memory:".to_string()), // In-memory for isolation
            ..Default::default()
        };
        
        TestServer::start_with_config(config).await
    }

    /// Create a server configured for provider fallback testing
    pub async fn fallback_server() -> Result<TestServer> {
        let config = TestServerConfig {
            enable_provider_fallback: true,
            primary_provider: "vertex".to_string(),
            fallback_providers: vec!["groq".to_string(), "openai".to_string()],
            ..Default::default()
        };
        
        TestServer::start_with_config(config).await
    }

    /// Create a server configured for streaming testing
    pub async fn streaming_server() -> Result<TestServer> {
        let config = TestServerConfig {
            enable_conversations: false, // Focus on streaming
            enable_provider_fallback: false,
            ..Default::default()
        };
        
        TestServer::start_with_config(config).await
    }

    /// Create a server configured for performance testing
    pub async fn performance_server() -> Result<TestServer> {
        let config = TestServerConfig {
            enable_conversations: true,
            enable_provider_fallback: true,
            database_url: Some("sqlite::memory:".to_string()),
            ..Default::default()
        };
        
        TestServer::start_with_config(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_startup() {
        let server = TestServer::start().await.unwrap();
        assert!(server.is_healthy().await);
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_server_manager() {
        let mut manager = TestServerManager::new();
        let _server = manager.start_server(TestServerConfig::default()).await.unwrap();
        
        assert_eq!(manager.get_server_urls().len(), 1);
        
        manager.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_scenario_servers() {
        // Test that each scenario server can start successfully
        let codex_server = TestScenarios::codex_cli_server().await.unwrap();
        assert!(codex_server.is_healthy().await);
        codex_server.shutdown().await.unwrap();

        let conv_server = TestScenarios::conversation_server().await.unwrap();
        assert!(conv_server.is_healthy().await);
        conv_server.shutdown().await.unwrap();
    }
}