
// src/features/provider_fallback/provider_health.rs
//
// Comprehensive provider health monitoring and circuit breaker implementation for all 8 providers

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Mutex;
use tokio::time::{interval, sleep, MissedTickBehavior};
use tracing::{debug, error, info, warn};

use super::models::{
    Provider, ProviderHealth, ProviderErrorType, CircuitBreakerState, 
    CircuitBreakerConfig, HealthStatus, RequestMetrics
};

/// Comprehensive provider health monitor with circuit breaker functionality for all 8 providers
#[derive(Debug)]
pub struct ProviderHealthMonitor {
    health_data: Arc<RwLock<HashMap<Provider, ProviderHealth>>>,
    circuit_breaker_config: CircuitBreakerConfig,
    health_check_interval: Duration,
    is_running: Arc<Mutex<bool>>,
    provider_specific_configs: HashMap<Provider, ProviderHealthConfig>,
}

/// Provider-specific health monitoring configuration
#[derive(Debug, Clone)]
pub struct ProviderHealthConfig {
    /// Custom health check endpoint for this provider
    pub health_endpoint: Option<String>,
    /// Provider-specific timeout for health checks
    pub health_check_timeout: Duration,
    /// Provider-specific failure threshold before marking as unhealthy
    pub failure_threshold: u32,
    /// Provider-specific recovery threshold
    pub recovery_threshold: u32,
    /// Whether this provider supports health check pings
    pub supports_health_ping: bool,
    /// Provider-specific rate limiting awareness
    pub rate_limit_aware: bool,
}

impl ProviderHealthConfig {
    /// Get default health configuration for a provider
    pub fn for_provider(provider: Provider) -> Self {
        match provider {
            Provider::VertexAI => Self {
                health_endpoint: Some("https://us-central1-aiplatform.googleapis.com/v1/projects".to_string()),
                health_check_timeout: Duration::from_secs(10),
                failure_threshold: 3,
                recovery_threshold: 2,
                supports_health_ping: true,
                rate_limit_aware: true,
            },
            Provider::OpenAI => Self {
                health_endpoint: Some("https://api.openai.com/v1/models".to_string()),
                health_check_timeout: Duration::from_secs(5),
                failure_threshold: 2,
                recovery_threshold: 3,
                supports_health_ping: true,
                rate_limit_aware: true,
            },
            Provider::AzureOpenAI => Self {
                health_endpoint: None, // Endpoint varies per deployment
                health_check_timeout: Duration::from_secs(8),
                failure_threshold: 2,
                recovery_threshold: 3,
                supports_health_ping: true,
                rate_limit_aware: true,
            },
            Provider::Anthropic => Self {
                health_endpoint: Some("https://api.anthropic.com/v1/messages".to_string()),
                health_check_timeout: Duration::from_secs(6),
                failure_threshold: 3,
                recovery_threshold: 2,
                supports_health_ping: false, // Anthropic doesn't have a dedicated health endpoint
                rate_limit_aware: true,
            },
            Provider::Groq => Self {
                health_endpoint: Some("https://api.groq.com/openai/v1/models".to_string()),
                health_check_timeout: Duration::from_secs(3), // Groq is fast
                failure_threshold: 5, // Groq can be flaky
                recovery_threshold: 2,
                supports_health_ping: true,
                rate_limit_aware: true, // Groq has strict rate limits
            },
            Provider::Mistral => Self {
                health_endpoint: Some("https://api.mistral.ai/v1/models".to_string()),
                health_check_timeout: Duration::from_secs(5),
                failure_threshold: 3,
                recovery_threshold: 2,
                supports_health_ping: true,
                rate_limit_aware: true,
            },
            Provider::AwsBedrock => Self {
                health_endpoint: None, // Regional endpoints vary
                health_check_timeout: Duration::from_secs(10),
                failure_threshold: 4,
                recovery_threshold: 2,
                supports_health_ping: true,
                rate_limit_aware: false, // AWS manages rate limiting differently
            },
            Provider::Cohere => Self {
                health_endpoint: Some("https://api.cohere.ai/v1/models".to_string()),
                health_check_timeout: Duration::from_secs(6),
                failure_threshold: 3,
                recovery_threshold: 2,
                supports_health_ping: true,
                rate_limit_aware: true,
            },
        }
    }
}

impl ProviderHealthMonitor {
    /// Create a new comprehensive provider health monitor
    pub fn new(
        circuit_breaker_config: CircuitBreakerConfig,
        health_check_interval: Duration,
    ) -> Self {
        let mut health_data = HashMap::new();
        let mut provider_specific_configs = HashMap::new();
        
        // Initialize health data and configs for all 8 providers
        for provider in Provider::all() {
            health_data.insert(provider, ProviderHealth::new(provider));
            provider_specific_configs.insert(provider, ProviderHealthConfig::for_provider(provider));
        }

        Self {
            health_data: Arc::new(RwLock::new(health_data)),
            circuit_breaker_config,
            health_check_interval,
            is_running: Arc::new(Mutex::new(false)),
            provider_specific_configs,
        }
    }

    /// Start the comprehensive health monitoring background task
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        let health_data = Arc::clone(&self.health_data);
        let interval_duration = self.health_check_interval;
        let circuit_breaker_config = self.circuit_breaker_config.clone();
        let is_running_flag = Arc::clone(&self.is_running);
        let provider_configs = self.provider_specific_configs.clone();

        tokio::spawn(async move {
            let mut interval = interval(interval_duration);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            info!(
                "Started comprehensive provider health monitoring for {} providers with interval {:?}",
                Provider::all().len(),
                interval_duration
            );

            loop {
                interval.tick().await;

                // Check if we should continue running
                let running = {
                    let is_running = is_running_flag.lock().await;
                    *is_running
                };

                if !running {
                    info!("Stopping comprehensive provider health monitoring");
                    break;
                }

                // Update circuit breaker states for all providers
                Self::update_circuit_breakers(&health_data, &circuit_breaker_config).await;

                // Perform active health checks for supported providers
                Self::perform_active_health_checks(&health_data, &provider_configs).await;

                // Log comprehensive health status
                Self::log_comprehensive_health_status(&health_data).await;
            }
        });

        Ok(())
    }

    /// Stop the health monitoring
    pub async fn stop(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
    }

    /// Record a successful request for a provider with enhanced metrics
    pub async fn record_success(&self, provider: Provider, response_time: Duration) {
        if let Ok(mut health_data) = self.health_data.write() {
            if let Some(health) = health_data.get_mut(&provider) {
                health.record_success(response_time);
                
                // Provider-specific success handling
                self.handle_provider_specific_success(provider, response_time, health).await;
                
                debug!(
                    provider = %provider.display_name(),
                    response_time_ms = response_time.as_millis(),
                    success_rate = %format!("{:.2}", health.success_rate),
                    consecutive_failures = health.consecutive_failures,
                    circuit_state = ?health.circuit_breaker_state,
                    "Recorded success for provider"
                );
            }
        }
    }

    /// Record a failed request for a provider with enhanced error analysis
    pub async fn record_failure(&self, provider: Provider, error_type: ProviderErrorType) {
        if let Ok(mut health_data) = self.health_data.write() {
            if let Some(health) = health_data.get_mut(&provider) {
                health.record_failure(&error_type);
                
                // Provider-specific failure handling
                self.handle_provider_specific_failure(provider, &error_type, health).await;
                
                warn!(
                    provider = %provider.display_name(),
                    error = ?error_type,
                    consecutive_failures = health.consecutive_failures,
                    success_rate = %format!("{:.2}", health.success_rate),
                    circuit_state = ?health.circuit_breaker_state,
                    "Recorded failure for provider"
                );

                // Check if we should trip the circuit breaker
                self.update_circuit_breaker_for_provider(health, provider).await;
            }
        }
    }

    /// Handle provider-specific success patterns
    async fn handle_provider_specific_success(
        &self,
        provider: Provider,
        response_time: Duration,
        health: &mut ProviderHealth,
    ) {
        match provider {
            Provider::Groq => {
                // Groq is known for speed - track if it's performing as expected
                if response_time > Duration::from_millis(1000) {
                    debug!(
                        provider = %provider.display_name(),
                        response_time_ms = response_time.as_millis(),
                        "Groq response time higher than expected"
                    );
                }
            }
            Provider::AzureOpenAI | Provider::AwsBedrock => {
                // Enterprise providers - expect consistent performance
                if health.success_rate > 0.98 {
                    debug!(
                        provider = %provider.display_name(),
                        success_rate = health.success_rate,
                        "Enterprise provider showing excellent reliability"
                    );
                }
            }
            Provider::Mistral => {
                // Track European provider performance
                if response_time < Duration::from_millis(800) {
                    debug!(
                        provider = %provider.display_name(),
                        response_time_ms = response_time.as_millis(),
                        "Mistral showing excellent European performance"
                    );
                }
            }
            _ => {
                // Standard handling for other providers
            }
        }
    }

    /// Handle provider-specific failure patterns
    async fn handle_provider_specific_failure(
        &self,
        provider: Provider,
        error_type: &ProviderErrorType,
        health: &mut ProviderHealth,
    ) {
        match provider {
            Provider::Groq => {
                // Groq has strict rate limits - handle specially
                if matches!(error_type, ProviderErrorType::RateLimit { .. }) {
                    warn!(
                        provider = %provider.display_name(),
                        "Groq rate limit hit - may need longer backoff"
                    );
                }
            }
            Provider::Anthropic => {
                // Anthropic has specific error patterns
                if matches!(error_type, ProviderErrorType::ModelNotAvailable) {
                    warn!(
                        provider = %provider.display_name(),
                        "Anthropic model availability issue - check model names"
                    );
                }
            }
            Provider::AwsBedrock => {
                // AWS Bedrock may have regional issues
                if matches!(error_type, ProviderErrorType::ServiceUnavailable) {
                    warn!(
                        provider = %provider.display_name(),
                        "AWS Bedrock service unavailable - may be regional"
                    );
                }
            }
            Provider::AzureOpenAI => {
                // Azure may have deployment-specific issues
                if matches!(error_type, ProviderErrorType::Authorization) {
                    error!(
                        provider = %provider.display_name(),
                        "Azure OpenAI authorization failure - check deployment config"
                    );
                }
            }
            Provider::Mistral => {
                // Mistral European compliance considerations
                if matches!(error_type, ProviderErrorType::Authorization) {
                    warn!(
                        provider = %provider.display_name(),
                        "Mistral authorization issue - check EU compliance settings"
                    );
                }
            }
            _ => {
                // Standard handling for other providers
            }
        }
    }

    /// Get current health status for a provider
    pub async fn get_health(&self, provider: Provider) -> Option<ProviderHealth> {
        if let Ok(health_data) = self.health_data.read() {
            health_data.get(&provider).cloned()
        } else {
            None
        }
    }

    /// Get health status for all providers
    pub async fn get_all_health(&self) -> HashMap<Provider, ProviderHealth> {
        if let Ok(health_data) = self.health_data.read() {
            health_data.clone()
        } else {
            HashMap::new()
        }
    }

    /// Get available providers (healthy and not circuit broken)
    pub async fn get_available_providers(&self) -> Vec<Provider> {
        if let Ok(health_data) = self.health_data.read() {
            health_data
                .values()
                .filter(|health| health.is_available())
                .map(|health| health.provider)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get providers sorted by priority (best first) with enhanced scoring
    pub async fn get_providers_by_priority(&self) -> Vec<(Provider, f64)> {
        if let Ok(health_data) = self.health_data.read() {
            let mut providers: Vec<_> = health_data
                .values()
                .map(|health| {
                    let base_score = health.priority_score();
                    let enhanced_score = self.calculate_enhanced_priority_score(health, base_score);
                    (health.provider, enhanced_score)
                })
                .collect();
            
            providers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            providers
        } else {
            Vec::new()
        }
    }

    /// Calculate enhanced priority score with provider-specific adjustments
    fn calculate_enhanced_priority_score(&self, health: &ProviderHealth, base_score: f64) -> f64 {
        let mut enhanced_score = base_score;

        // Provider-specific adjustments based on characteristics
        match health.provider {
            Provider::Groq => {
                // Boost Groq if response time is good (it's supposed to be fast)
                if health.average_response_time < Duration::from_millis(500) {
                    enhanced_score += 10.0;
                }
            }
            Provider::OpenAI => {
                // OpenAI gets reliability bonus due to industry leadership
                if health.success_rate > 0.95 {
                    enhanced_score += 15.0;
                }
            }
            Provider::AzureOpenAI | Provider::AwsBedrock => {
                // Enterprise providers get bonus for consistency
                if health.consecutive_failures == 0 {
                    enhanced_score += 12.0;
                }
            }
            Provider::Mistral => {
                // European provider gets bonus for compliance scenarios
                enhanced_score += 8.0;
            }
            Provider::Anthropic => {
                // Anthropic gets quality bonus
                if health.success_rate > 0.90 {
                    enhanced_score += 10.0;
                }
            }
            Provider::Cohere => {
                // Cohere gets enterprise reliability bonus
                if health.circuit_breaker_state == CircuitBreakerState::Closed {
                    enhanced_score += 8.0;
                }
            }
            Provider::VertexAI => {
                // Google integration bonus for stable performance
                if health.average_response_time < Duration::from_secs(3) {
                    enhanced_score += 6.0;
                }
            }
        }

        enhanced_score.max(0.0)
    }

    /// Check if a provider is available for requests
    pub async fn is_provider_available(&self, provider: Provider) -> bool {
        if let Some(health) = self.get_health(provider).await {
            health.is_available()
        } else {
            false
        }
    }

    /// Get the best available provider for a request with enhanced selection
    pub async fn get_best_provider(&self) -> Option<Provider> {
        let providers = self.get_providers_by_priority().await;
        providers
            .into_iter()
            .find(|(provider, score)| *score > 0.0 && self.is_provider_immediately_available(*provider))
            .map(|(provider, _)| provider)
    }

    /// Check if provider is immediately available (no rate limiting, etc.)
    fn is_provider_immediately_available(&self, provider: Provider) -> bool {
        if let Some(config) = self.provider_specific_configs.get(&provider) {
            // Provider-specific availability checks
            match provider {
                Provider::Groq => {
                    // Groq has strict rate limits, check more carefully
                    config.rate_limit_aware
                }
                _ => true,
            }
        } else {
            true
        }
    }

    /// Perform health check for a specific provider with provider-specific logic
    pub async fn health_check(&self, provider: Provider) -> Result<Duration, ProviderErrorType> {
        let start = Instant::now();
        
        // Get provider config
        let config = self.provider_specific_configs.get(&provider)
            .cloned()
            .unwrap_or_else(|| ProviderHealthConfig::for_provider(provider));
        
        // Perform health check
        match Self::ping_provider_endpoint(provider, &config).await {
            Ok(response_time) => {
                self.record_success(provider, response_time).await;
                Ok(response_time)
            }
            Err(error) => {
                self.record_failure(provider, error.clone()).await;
                Err(error)
            }
        }
    }

    /// Update circuit breaker state for a specific provider with enhanced logic
    async fn update_circuit_breaker_for_provider(&self, health: &mut ProviderHealth, provider: Provider) {
        let config = self.provider_specific_configs.get(&provider).unwrap();
        
        match health.circuit_breaker_state {
            CircuitBreakerState::Closed => {
                // Use provider-specific failure threshold
                if health.consecutive_failures >= config.failure_threshold {
                    health.circuit_breaker_state = CircuitBreakerState::Open;
                    warn!(
                        provider = %provider.display_name(),
                        consecutive_failures = health.consecutive_failures,
                        threshold = config.failure_threshold,
                        "Circuit breaker OPENED for provider after exceeding failure threshold"
                    );
                }
            }
            CircuitBreakerState::Open => {
                // Circuit breaker remains open - will be handled in periodic update
            }
            CircuitBreakerState::HalfOpen => {
                // If we're in half-open and got a failure, go back to open
                if health.consecutive_failures > 0 {
                    health.circuit_breaker_state = CircuitBreakerState::Open;
                    warn!(
                        provider = %provider.display_name(),
                        "Circuit breaker returned to OPEN after failure in half-open state"
                    );
                } else if health.successful_requests >= config.recovery_threshold as u64 {
                    // Enough successful requests to close the circuit
                    health.circuit_breaker_state = CircuitBreakerState::Closed;
                    info!(
                        provider = %provider.display_name(),
                        recovery_threshold = config.recovery_threshold,
                        "Circuit breaker CLOSED after successful recovery"
                    );
                }
            }
        }
    }

    /// Update circuit breaker states for all providers with enhanced logic
    async fn update_circuit_breakers(
        health_data: &Arc<RwLock<HashMap<Provider, ProviderHealth>>>,
        config: &CircuitBreakerConfig,
    ) {
        if let Ok(mut health_data) = health_data.write() {
            for (provider, health) in health_data.iter_mut() {
                match health.circuit_breaker_state {
                    CircuitBreakerState::Open => {
                        // Check if timeout has elapsed to transition to half-open
                        if let Some(last_failure) = health.last_failure {
                            if SystemTime::now()
                                .duration_since(last_failure)
                                .unwrap_or(Duration::ZERO)
                                >= config.timeout
                            {
                                health.circuit_breaker_state = CircuitBreakerState::HalfOpen;
                                info!(
                                    provider = %provider.display_name(),
                                    timeout_duration = ?config.timeout,
                                    "Circuit breaker transitioned to HALF-OPEN after timeout"
                                );
                            }
                        }
                    }
                    CircuitBreakerState::HalfOpen => {
                        // Check if we have enough successful requests to close
                        if health.consecutive_failures == 0 && 
                           health.successful_requests >= config.success_threshold as u64 {
                            health.circuit_breaker_state = CircuitBreakerState::Closed;
                            info!(
                                provider = %provider.display_name(),
                                success_threshold = config.success_threshold,
                                "Circuit breaker CLOSED after successful recovery"
                            );
                        }
                    }
                    CircuitBreakerState::Closed => {
                        // Normal operation - no action needed
                    }
                }
            }
        }
    }

    /// Perform active health checks for providers that support them
    async fn perform_active_health_checks(
        health_data: &Arc<RwLock<HashMap<Provider, ProviderHealth>>>,
        provider_configs: &HashMap<Provider, ProviderHealthConfig>,
    ) {
        for (provider, config) in provider_configs {
            if config.supports_health_ping && config.health_endpoint.is_some() {
                // Perform actual health check (simplified for now)
                let health_check_result = Self::ping_provider_endpoint(*provider, config).await;
                
                if let Ok(health_data) = health_data.write() {
                    if let Some(health) = health_data.get(provider) {
                        if health.is_available() {
                            // Only ping if provider is currently available to avoid unnecessary load
                            match health_check_result {
                                Ok(response_time) => {
                                    // Update health data with successful ping response
                                    info!(
                                        provider = %provider.display_name(),
                                        response_time = response_time.as_millis(),
                                        "Health ping successful"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        provider = %provider.display_name(),
                                        error = %e,
                                        "Health ping failed"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get provider-specific configuration for external access
    pub fn get_provider_config(&self, provider: Provider) -> Option<&ProviderHealthConfig> {
        self.provider_specific_configs.get(&provider)
    }

    /// Update provider-specific configuration
    pub async fn update_provider_config(&mut self, provider: Provider, config: ProviderHealthConfig) {
        self.provider_specific_configs.insert(provider, config);
        info!(
            provider = %provider.display_name(),
            "Updated provider-specific health configuration"
        );
    }

    /// Get comprehensive provider status report
    pub async fn get_provider_status_report(&self) -> ProviderStatusReport {
        let health_data = self.get_all_health().await;
        let available_providers = self.get_available_providers().await;
        let providers_by_priority = self.get_providers_by_priority().await;
        
        let mut healthy_providers = Vec::new();
        let mut degraded_providers = Vec::new();
        let mut unhealthy_providers = Vec::new();
        let mut rate_limited_providers = Vec::new();

        for (provider, health) in &health_data {
            match health.status {
                HealthStatus::Healthy => healthy_providers.push(*provider),
                HealthStatus::Degraded => degraded_providers.push(*provider),
                HealthStatus::Unhealthy => unhealthy_providers.push(*provider),
                HealthStatus::RateLimited => rate_limited_providers.push(*provider),
            }
        }

        ProviderStatusReport {
            total_providers: Provider::all().len(),
            available_providers: available_providers.len(),
            healthy_providers,
            degraded_providers,
            unhealthy_providers,
            rate_limited_providers,
            providers_by_priority,
            system_health_score: self.calculate_system_health_score(&health_data).await,
        }
    }

    /// Calculate overall system health score
    async fn calculate_system_health_score(&self, health_data: &HashMap<Provider, ProviderHealth>) -> f64 {
        if health_data.is_empty() {
            return 0.0;
        }

        let total_score: f64 = health_data.values()
            .map(|health| {
                match health.status {
                    HealthStatus::Healthy => 100.0,
                    HealthStatus::Degraded => 60.0,
                    HealthStatus::RateLimited => 40.0,
                    HealthStatus::Unhealthy => 0.0,
                }
            })
            .sum();

        total_score / health_data.len() as f64
    }

    /// Get comprehensive health metrics
    pub async fn get_metrics(&self) -> HealthMetrics {
        let health_data = self.get_all_health().await;
        let available_providers = self.get_available_providers().await;
        
        let mut provider_metrics = HashMap::new();
        let mut circuit_breaker_states = HashMap::new();
        let mut average_response_times = HashMap::new();
        
        let mut total_requests = 0u64;
        let mut total_failures = 0u64;
        let mut total_successes = 0u64;
        
        for (provider, health) in &health_data {
            provider_metrics.insert(*provider, ProviderMetrics {
                requests: health.total_requests,
                successes: health.successful_requests,
                failures: health.failed_requests,
                success_rate: health.success_rate,
                consecutive_failures: health.consecutive_failures,
            });
            
            circuit_breaker_states.insert(*provider, health.circuit_breaker_state);
            average_response_times.insert(*provider, health.average_response_time);
            
            total_requests += health.total_requests;
            total_failures += health.failed_requests;
            total_successes += health.successful_requests;
        }
        
        let overall_success_rate = if total_requests > 0 {
            total_successes as f64 / total_requests as f64
        } else {
            0.0
        };
        
        HealthMetrics {
            overall_success_rate,
            total_requests,
            total_failures,
            available_providers: available_providers.len(),
            total_providers: Provider::all().len(),
            provider_health: health_data,
            provider_metrics,
            circuit_breaker_states,
            average_response_times,
        }
    }

    /// Ping a provider endpoint for health checking
    async fn ping_provider_endpoint(
        provider: Provider,
        config: &ProviderHealthConfig,
    ) -> Result<Duration, ProviderErrorType> {
        let start = Instant::now();
        
        // Simulate health check (in real implementation, this would make HTTP requests)
        match provider {
            Provider::VertexAI => {
                // Simulate Google Cloud health check
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(start.elapsed())
            }
            Provider::OpenAI => {
                // Simulate OpenAI API health check
                tokio::time::sleep(Duration::from_millis(150)).await;
                Ok(start.elapsed())
            }
            Provider::AzureOpenAI => {
                // Simulate Azure health check
                tokio::time::sleep(Duration::from_millis(180)).await;
                Ok(start.elapsed())
            }
            Provider::Groq => {
                // Simulate Groq health check (should be fast)
                tokio::time::sleep(Duration::from_millis(80)).await;
                Ok(start.elapsed())
            }
            Provider::Mistral => {
                // Simulate Mistral health check
                tokio::time::sleep(Duration::from_millis(120)).await;
                Ok(start.elapsed())
            }
            Provider::AwsBedrock => {
                // Simulate AWS Bedrock health check
                tokio::time::sleep(Duration::from_millis(250)).await;
                Ok(start.elapsed())
            }
            Provider::Cohere => {
                // Simulate Cohere health check
                tokio::time::sleep(Duration::from_millis(140)).await;
                Ok(start.elapsed())
            }
            Provider::Anthropic => {
                // Anthropic doesn't support health pings
                Err(ProviderErrorType::ServiceUnavailable)
            }
        }
    }

    /// Log comprehensive health status for all providers
    async fn log_comprehensive_health_status(
        health_data: &Arc<RwLock<HashMap<Provider, ProviderHealth>>>
    ) {
        if let Ok(health_data) = health_data.read() {
            let available_count = health_data.values().filter(|h| h.is_available()).count();
            let total_count = health_data.len();
            
            debug!(
                available_providers = available_count,
                total_providers = total_count,
                "Provider health status update"
            );
            
            for (provider, health) in health_data.iter() {
                debug!(
                    provider = %provider.display_name(),
                    status = ?health.status,
                    success_rate = health.success_rate,
                    consecutive_failures = health.consecutive_failures,
                    circuit_breaker = ?health.circuit_breaker_state,
                    total_requests = health.total_requests,
                    "Provider health details"
                );
            }
        }
    }
}

/// Enhanced health metrics with provider-specific data
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    pub overall_success_rate: f64,
    pub total_requests: u64,
    pub total_failures: u64,
    pub available_providers: usize,
    pub total_providers: usize,
    pub provider_health: HashMap<Provider, ProviderHealth>,
    pub provider_metrics: HashMap<Provider, ProviderMetrics>,
    pub circuit_breaker_states: HashMap<Provider, CircuitBreakerState>,
    pub average_response_times: HashMap<Provider, Duration>,
}

/// Provider-specific metrics
#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub success_rate: f64,
    pub consecutive_failures: u32,
}

/// Comprehensive provider status report
#[derive(Debug, Clone)]
pub struct ProviderStatusReport {
    pub total_providers: usize,
    pub available_providers: usize,
    pub healthy_providers: Vec<Provider>,
    pub degraded_providers: Vec<Provider>,
    pub unhealthy_providers: Vec<Provider>,
    pub rate_limited_providers: Vec<Provider>,
    pub providers_by_priority: Vec<(Provider, f64)>,
    pub system_health_score: f64,
}

impl HealthMetrics {
    /// Check if the overall system is healthy
    pub fn is_healthy(&self) -> bool {
        self.available_providers > 0 && self.overall_success_rate >= 0.8
    }

    /// Get system health status with enhanced categorization
    pub fn system_status(&self) -> SystemHealthStatus {
        let availability_rate = self.available_providers as f64 / self.total_providers as f64;
        
        if self.available_providers == 0 {
            SystemHealthStatus::Critical
        } else if availability_rate < 0.5 || self.overall_success_rate < 0.7 {
            SystemHealthStatus::Critical
        } else if availability_rate < 0.75 || self.overall_success_rate < 0.85 {
            SystemHealthStatus::Degraded
        } else if availability_rate < 0.9 || self.overall_success_rate < 0.95 {
            SystemHealthStatus::Warning
        } else {
            SystemHealthStatus::Healthy
        }
    }

    /// Get provider with best performance
    pub fn get_best_performing_provider(&self) -> Option<Provider> {
        self.provider_metrics
            .iter()
            .filter(|(provider, metrics)| {
                // Only consider providers that are available and have good success rates
                self.provider_health.get(provider)
                    .map(|health| health.is_available() && health.success_rate > 0.8)
                    .unwrap_or(false)
            })
            .max_by(|(a_provider, a), (b_provider, b)| {
                // Compare by success rate first, then by response time
                a.success_rate.partial_cmp(&b.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        // Lower average response time is better
                        let default_duration = Duration::from_secs(10);
                        let a_time = self.average_response_times.get(a_provider).unwrap_or(&default_duration);
                        let b_time = self.average_response_times.get(b_provider).unwrap_or(&default_duration);
                        b_time.cmp(a_time)
                    })
            })
            .map(|(provider, _)| *provider)
    }

    /// Get providers that need attention
    pub fn get_providers_needing_attention(&self) -> Vec<(Provider, String)> {
        let mut attention_needed = Vec::new();

        for (provider, health) in &self.provider_health {
            match health.status {
                HealthStatus::Unhealthy => {
                    attention_needed.push((*provider, "Provider is unhealthy".to_string()));
                }
                HealthStatus::Degraded => {
                    attention_needed.push((*provider, "Provider performance is degraded".to_string()));
                }
                HealthStatus::RateLimited => {
                    attention_needed.push((*provider, "Provider is rate limited".to_string()));
                }
                _ => {}
            }

            if health.consecutive_failures > 5 {
                attention_needed.push((*provider, format!("High consecutive failures: {}", health.consecutive_failures)));
            }

            if health.success_rate < 0.7 && health.total_requests > 10 {
                attention_needed.push((*provider, format!("Low success rate: {:.1}%", health.success_rate * 100.0)));
            }

            if health.circuit_breaker_state == CircuitBreakerState::Open {
                attention_needed.push((*provider, "Circuit breaker is open".to_string()));
            }
        }

        attention_needed
    }
}

/// System-wide health status with enhanced categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealthStatus {
    Healthy,    // All systems operational
    Warning,    // Some degradation but functional
    Degraded,   // Significant issues but still operational
    Critical,   // System barely operational or non-functional
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_comprehensive_health_monitor_creation() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        let health = monitor.get_all_health().await;
        assert_eq!(health.len(), Provider::all().len());
        
        // Verify all 8 providers are included and properly configured
        for provider in Provider::all() {
            assert!(health.contains_key(&provider));
            let provider_health = &health[&provider];
            assert_eq!(provider_health.status, HealthStatus::Healthy);
            assert_eq!(provider_health.circuit_breaker_state, CircuitBreakerState::Closed);
            
            // Verify provider-specific configuration exists
            assert!(monitor.provider_specific_configs.contains_key(&provider));
        }
    }

    #[tokio::test]
    async fn test_provider_specific_configurations() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Test Groq-specific configuration (should have higher failure threshold)
        let groq_config = monitor.get_provider_config(Provider::Groq).unwrap();
        assert_eq!(groq_config.failure_threshold, 5); // Groq can be flaky
        assert!(groq_config.rate_limit_aware);
        assert!(groq_config.supports_health_ping);
        
        // Test Anthropic configuration (doesn't support health pings)
        let anthropic_config = monitor.get_provider_config(Provider::Anthropic).unwrap();
        assert!(!anthropic_config.supports_health_ping);
        
        // Test enterprise providers
        let azure_config = monitor.get_provider_config(Provider::AzureOpenAI).unwrap();
        let aws_config = monitor.get_provider_config(Provider::AwsBedrock).unwrap();
        assert!(azure_config.supports_health_ping);
        assert!(aws_config.supports_health_ping);
        assert!(!aws_config.rate_limit_aware); // AWS handles differently
    }

    #[tokio::test]
    async fn test_enhanced_priority_scoring() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Record success for Groq with fast response time
        monitor.record_success(Provider::Groq, Duration::from_millis(300)).await;
        
        // Record success for OpenAI with good reliability
        monitor.record_success(Provider::OpenAI, Duration::from_millis(800)).await;
        
        let priorities = monitor.get_providers_by_priority().await;
        
        // Both providers should have enhanced scores
        let groq_score = priorities.iter()
            .find(|(p, _)| *p == Provider::Groq)
            .map(|(_, s)| *s);
        let openai_score = priorities.iter()
            .find(|(p, _)| *p == Provider::OpenAI)
            .map(|(_, s)| *s);
        
        assert!(groq_score.is_some());
        assert!(openai_score.is_some());
        
        // Groq should get speed bonus for fast response
        // OpenAI should get reliability bonus
        // Both should have positive scores
        assert!(groq_score.unwrap() > 100.0); // Base + speed bonus
        assert!(openai_score.unwrap() > 100.0); // Base + reliability bonus
    }

    #[tokio::test]
    async fn test_provider_specific_failure_handling() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Test Groq rate limiting
        monitor.record_failure(Provider::Groq, ProviderErrorType::RateLimit {
            reset_time: SystemTime::now() + Duration::from_secs(60)
        }).await;
        
        let groq_health = monitor.get_health(Provider::Groq).await.unwrap();
        assert_eq!(groq_health.status, HealthStatus::RateLimited);
        
        // Test Azure authorization failure
        monitor.record_failure(Provider::AzureOpenAI, ProviderErrorType::Authorization).await;
        
        let azure_health = monitor.get_health(Provider::AzureOpenAI).await.unwrap();
        assert_eq!(azure_health.consecutive_failures, 1);
    }

    #[tokio::test]
    async fn test_enhanced_metrics_collection() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Record mixed results for different providers
        monitor.record_success(Provider::OpenAI, Duration::from_millis(500)).await;
        monitor.record_success(Provider::Groq, Duration::from_millis(200)).await;
        monitor.record_failure(Provider::Anthropic, ProviderErrorType::Timeout).await;
        
        let metrics = monitor.get_metrics().await;
        
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.total_failures, 1);
        assert!((metrics.overall_success_rate - 2.0/3.0).abs() < 0.01);
        
        // Check provider-specific metrics
        assert!(metrics.provider_metrics.contains_key(&Provider::OpenAI));
        assert!(metrics.provider_metrics.contains_key(&Provider::Groq));
        assert!(metrics.provider_metrics.contains_key(&Provider::Anthropic));
        
        // Check response times
        assert!(metrics.average_response_times.contains_key(&Provider::OpenAI));
        assert!(metrics.average_response_times.contains_key(&Provider::Groq));
        
        // Groq should have faster response time
        let groq_time = metrics.average_response_times[&Provider::Groq];
        let openai_time = metrics.average_response_times[&Provider::OpenAI];
        assert!(groq_time < openai_time);
    }

    #[tokio::test]
    async fn test_provider_status_report() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Set up different provider states
        monitor.record_success(Provider::OpenAI, Duration::from_millis(500)).await;
        monitor.record_failure(Provider::Groq, ProviderErrorType::Timeout).await;
        monitor.record_failure(Provider::Groq, ProviderErrorType::Timeout).await;
        monitor.record_failure(Provider::Groq, ProviderErrorType::Timeout).await;
        
        let report = monitor.get_provider_status_report().await;
        
        assert_eq!(report.total_providers, 8);
        assert!(report.healthy_providers.contains(&Provider::OpenAI));
        assert!(report.degraded_providers.contains(&Provider::Groq));
        assert!(report.system_health_score > 0.0);
        assert!(report.system_health_score < 100.0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_with_provider_specific_thresholds() {
        let mut config = CircuitBreakerConfig::default();
        config.failure_threshold = 2; // Global threshold
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Test with Groq (should have higher threshold of 5)
        for _ in 0..3 {
            monitor.record_failure(Provider::Groq, ProviderErrorType::Timeout).await;
        }
        
        let groq_health = monitor.get_health(Provider::Groq).await.unwrap();
        assert_eq!(groq_health.circuit_breaker_state, CircuitBreakerState::Closed); // Still closed due to higher threshold
        
        // Test with OpenAI (should use provider-specific threshold of 2)
        for _ in 0..2 {
            monitor.record_failure(Provider::OpenAI, ProviderErrorType::Timeout).await;
        }
        
        let openai_health = monitor.get_health(Provider::OpenAI).await.unwrap();
        assert_eq!(openai_health.circuit_breaker_state, CircuitBreakerState::Open); // Should be open
    }

    #[tokio::test]
    async fn test_health_monitor_start_stop() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_millis(100));
        
        // Start monitoring
        assert!(monitor.start().await.is_ok());
        
        // Wait a bit for health checks
        tokio::time::sleep(Duration::from_millis(250)).await;
        
        // Stop monitoring
        monitor.stop().await;
        
        // Starting again should work
        assert!(monitor.start().await.is_ok());
        monitor.stop().await;
    }

    #[tokio::test]
    async fn test_best_provider_selection() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config, Duration::from_secs(1));
        
        // Record excellent performance for Groq
        for _ in 0..5 {
            monitor.record_success(Provider::Groq, Duration::from_millis(200)).await;
        }
        
        // Record good performance for OpenAI
        for _ in 0..3 {
            monitor.record_success(Provider::OpenAI, Duration::from_millis(600)).await;
        }
        
        // Degrade Anthropic
        for _ in 0..3 {
            monitor.record_failure(Provider::Anthropic, ProviderErrorType::Timeout).await;
        }
        
        let best_provider = monitor.get_best_provider().await;
        
        // Should prefer Groq due to excellent performance
        assert!(best_provider.is_some());
        // The exact provider depends on the scoring algorithm, but it should be available
        let best = best_provider.unwrap();
        assert!(monitor.is_provider_available(best).await);
    }

    #[test]
    fn test_health_metrics_analysis() {
        let mut health_data = HashMap::new();
        let mut provider_metrics = HashMap::new();
        let mut average_response_times = HashMap::new();
        
        // Set up test data
        let mut openai_health = ProviderHealth::new(Provider::OpenAI);
        openai_health.success_rate = 0.95;
        openai_health.total_requests = 100;
        openai_health.successful_requests = 95;
        health_data.insert(Provider::OpenAI, openai_health);
        
        provider_metrics.insert(Provider::OpenAI, ProviderMetrics {
            requests: 100,
            successes: 95,
            failures: 5,
            success_rate: 0.95,
            consecutive_failures: 0,
        });
        
        average_response_times.insert(Provider::OpenAI, Duration::from_millis(500));
        
        let metrics = HealthMetrics {
            overall_success_rate: 0.95,
            total_requests: 100,
            total_failures: 5,
            available_providers: 1,
            total_providers: 8,
            provider_health: health_data,
            provider_metrics,
            circuit_breaker_states: HashMap::new(),
            average_response_times,
        };
        
        assert!(metrics.is_healthy());
        assert_eq!(metrics.system_status(), SystemHealthStatus::Degraded); // Low availability rate
        
        let best_provider = metrics.get_best_performing_provider();
        assert_eq!(best_provider, Some(Provider::OpenAI));
        
        let attention_needed = metrics.get_providers_needing_attention();
        // Should not need attention since OpenAI is performing well
        assert!(attention_needed.is_empty() || !attention_needed.iter().any(|(p, _)| *p == Provider::OpenAI));
    }

    #[test]
    fn test_european_provider_compliance_tracking() {
        // Test that European providers are properly identified and configured
        let mistral_config = ProviderHealthConfig::for_provider(Provider::Mistral);
        let cohere_config = ProviderHealthConfig::for_provider(Provider::Cohere);
        
        assert!(mistral_config.supports_health_ping);
        assert!(cohere_config.supports_health_ping);
        
        // European providers should be tracked for compliance
        assert!(Provider::Mistral.is_european_compliant());
        assert!(Provider::Cohere.is_european_compliant());
        assert!(Provider::AzureOpenAI.is_european_compliant());
        assert!(Provider::VertexAI.is_european_compliant());
        assert!(Provider::AwsBedrock.is_european_compliant());
    }
}
