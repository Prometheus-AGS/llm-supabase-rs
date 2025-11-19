// src/features/provider_fallback/mod.rs
//
// Provider fallback and error handling system

pub mod models;
pub mod provider_health;
pub mod retry_strategy;
pub mod fallback_manager;

pub use fallback_manager::FallbackManager;
pub use models::{
    Provider, ProviderCapability, ProviderHealth, FallbackConfig, 
    ProviderChoice, FallbackAction, RetryConfig, CircuitBreakerConfig
};
pub use provider_health::ProviderHealthMonitor;
pub use retry_strategy::RetryStrategy;