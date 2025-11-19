
// src/features/provider_fallback/fallback_manager.rs
//
// Core fallback logic and intelligent provider routing

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, instrument};
use uuid::Uuid;

use crate::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
};

use super::models::{
    Provider, ProviderChoice, FallbackAction, ProviderErrorType, 
    ProviderRequirements, ProviderCapability, RequestMetrics, FallbackConfig,
    RoutingStrategy, ScoringWeights, ProviderPerformanceProfile, IntelligentRoutingConfig
};
use super::provider_health::ProviderHealthMonitor;
use super::retry_strategy::{RetryStrategy, RetryContext, AdaptiveRetryStrategy};

/// Core fallback manager that orchestrates provider selection and failure handling
#[derive(Debug)]
pub struct FallbackManager {
    config: FallbackConfig,
    health_monitor: Arc<ProviderHealthMonitor>,
    retry_strategy: AdaptiveRetryStrategy,
    active_requests: Arc<RwLock<HashMap<String, RequestMetrics>>>,
    provider_capabilities: HashMap<Provider, ProviderCapability>,
    provider_performance: HashMap<Provider, ProviderPerformanceProfile>,
}

impl FallbackManager {
    /// Create a new fallback manager
    pub fn new(config: FallbackConfig) -> Self {
        let health_monitor = Arc::new(ProviderHealthMonitor::new(
            config.circuit_breaker_config.clone(),
            config.health_check_interval,
        ));

        let base_retry_strategy = RetryStrategy::new(config.retry_config.clone());
        let retry_strategy = AdaptiveRetryStrategy::new(base_retry_strategy);

        let mut provider_capabilities = HashMap::new();
        let mut provider_performance = HashMap::new();
        
        for provider in Provider::all() {
            provider_capabilities.insert(provider, ProviderCapability::for_provider(provider));
            provider_performance.insert(provider, provider.performance_characteristics());
        }

        Self {
            config,
            health_monitor,
            retry_strategy,
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            provider_capabilities,
            provider_performance,
        }
    }

    /// Start the fallback manager (starts health monitoring)
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting intelligent fallback manager with strategy: {:?}", self.config.routing_strategy);
        self.health_monitor.start().await?;
        info!("Fallback manager started successfully");
        Ok(())
    }

    /// Stop the fallback manager
    pub async fn stop(&self) {
        info!("Stopping fallback manager");
        self.health_monitor.stop().await;
        info!("Fallback manager stopped");
    }

    /// Route a request to the best available provider using intelligent routing
    #[instrument(skip(self, request))]
    pub async fn route_request(
        &self,
        request: &ChatCompletionRequest,
        preferred_providers: Option<Vec<Provider>>,
        routing_strategy: Option<RoutingStrategy>,
    ) -> Result<ProviderChoice, FallbackManagerError> {
        let request_id = Uuid::new_v4().to_string();
        let strategy = routing_strategy.unwrap_or(self.config.routing_strategy);
        
        debug!(
            request_id = %request_id,
            model = %request.model,
            routing_strategy = ?strategy,
            "Routing request with intelligent provider selection"
        );

        // Determine requirements from request
        let requirements = self.extract_requirements(request, strategy);

        // Get candidate providers using intelligent routing
        let candidates = self.get_intelligent_candidates(
            preferred_providers,
            &requirements,
            strategy,
        ).await;

        if candidates.is_empty() {
            return Err(FallbackManagerError::NoAvailableProviders {
                requirements: requirements.clone(),
            });
        }

        // Select best provider using intelligent scoring
        let choice = self.select_best_provider_intelligent(
            candidates,
            request,
            &requirements,
            strategy,
        ).await?;

        debug!(
            request_id = %request_id,
            provider = %choice.provider.display_name(),
            model = %choice.model,
            reason = %choice.reason,
            confidence = choice.confidence,
            routing_strategy = ?choice.routing_strategy,
            "Selected provider using intelligent routing"
        );

        // Track active request
        let mut metrics = RequestMetrics::new(request_id.clone(), choice.provider, choice.model.clone());
        metrics.routing_strategy = strategy;
        self.active_requests.write().await.insert(request_id, metrics);

        Ok(choice)
    }

    /// Handle provider failure and determine fallback action with intelligent switching
    #[instrument(skip(self))]
    pub async fn handle_provider_failure(
        &self,
        request_id: &str,
        provider: Provider,
        error: ProviderErrorType,
        attempt: u32,
    ) -> FallbackAction {
        warn!(
            request_id = %request_id,
            provider = %provider.display_name(),
            error = ?error,
            attempt = attempt,
            "Handling provider failure with intelligent fallback"
        );

        // Record failure in health monitor
        self.health_monitor.record_failure(provider, error.clone()).await;

        // Update request metrics
        if let Some(mut metrics) = self.active_requests.write().await.get_mut(request_id) {
            metrics.retry_count = attempt;
            metrics.complete_failure(error.clone());
        }

        // Create retry context for adaptive strategy
        let retry_context = RetryContext::new(request_id.to_string(), provider, error.clone());

        // Check if we should retry with the same provider
        if self.retry_strategy.should_retry_adaptive(provider, &error, attempt, &retry_context) {
            let delay = self.retry_strategy.calculate_adaptive_delay(provider, &error, attempt);
            
            info!(
                request_id = %request_id,
                provider = %provider.display_name(),
                delay_ms = delay.as_millis(),
                attempt = attempt + 1,
                "Retrying with same provider after failure"
            );

            return FallbackAction::Retry {
                provider,
                delay,
                attempt: attempt + 1,
            };
        }

        // Try intelligent fallback provider selection
        if let Some(fallback_provider) = self.get_intelligent_fallback_provider(
            provider,
            &error,
            request_id,
        ).await {
            info!(
                request_id = %request_id,
                from_provider = %provider.display_name(),
                to_provider = %fallback_provider.display_name(),
                "Switching to intelligently selected fallback provider"
            );

            // Mark request as using fallback
            if let Some(mut metrics) = self.active_requests.write().await.get_mut(request_id) {
                metrics.fallback_used = true;
            }

            return FallbackAction::Switch {
                from: provider,
                to: fallback_provider,
                reason: format!(
                    "Provider {} failed with {:?}, switching to intelligently selected {}",
                    provider.display_name(),
                    error,
                    fallback_provider.display_name()
                ),
            };
        }

        // No more options - fail the request
        error!(
            request_id = %request_id,
            provider = %provider.display_name(),
            "No intelligent fallback options available, failing request"
        );

        FallbackAction::Fail {
            reason: format!(
                "All intelligent fallback providers exhausted. Last provider {} failed with {:?}",
                provider.display_name(),
                error
            ),
            last_errors: vec![(provider, error)],
        }
    }

    /// Record successful completion of a request
    #[instrument(skip(self))]
    pub async fn record_success(
        &self,
        request_id: &str,
        provider: Provider,
        response_time: Duration,
    ) {
        debug!(
            request_id = %request_id,
            provider = %provider.display_name(),
            response_time_ms = response_time.as_millis(),
            "Recording successful request completion"
        );

        // Record success in health monitor
        self.health_monitor.record_success(provider, response_time).await;

        // Update request metrics
        if let Some(mut metrics) = self.active_requests.write().await.get_mut(request_id) {
            metrics.complete_success();
        }

        // Update adaptive retry strategy patterns
        // Note: This would need a mutable reference to retry_strategy
        // For now, we'll skip this update in the record_success method
    }

    /// Get available providers that meet requirements
    pub async fn get_available_providers(&self) -> Vec<Provider> {
        self.health_monitor.get_available_providers().await
    }

    /// Update provider health manually (for testing or manual intervention)
    pub async fn update_provider_health(&self, provider: Provider, success: bool, response_time: Option<Duration>) {
        if success {
            let time = response_time.unwrap_or(Duration::from_millis(1000));
            self.health_monitor.record_success(provider, time).await;
        } else {
            self.health_monitor.record_failure(provider, ProviderErrorType::InternalError).await;
        }
    }

    /// Get current system health metrics
    pub async fn get_health_metrics(&self) -> super::provider_health::HealthMetrics {
        self.health_monitor.get_metrics().await
    }

    /// Update routing strategy dynamically
    pub async fn update_routing_strategy(&mut self, strategy: RoutingStrategy) {
        info!(
            old_strategy = ?self.config.routing_strategy,
            new_strategy = ?strategy,
            "Updating routing strategy"
        );
        self.config.routing_strategy = strategy;
        self.config.provider_priority = strategy.fallback_chain();
    }

    /// Extract requirements from a chat completion request with routing strategy context
    fn extract_requirements(&self, request: &ChatCompletionRequest, strategy: RoutingStrategy) -> ProviderRequirements {
        let mut requirements = ProviderRequirements::default();

        // Check if streaming is required
        requirements.streaming = request.stream.unwrap_or(false);

        // Check if tool calling is required
        requirements.tool_calling = request.tools.is_some() && !request.tools.as_ref().unwrap().is_empty();

        // Check if vision is required (simplified - would need to analyze message content)
        requirements.vision = false; // TODO: Implement vision detection from messages

        // Set token requirements
        requirements.min_max_tokens = request.max_tokens.unwrap_or(1000);
        requirements.min_context_length = self.estimate_context_length(request);

        // Apply strategy-specific requirements
        match strategy {
            RoutingStrategy::Enterprise => {
                requirements.enterprise_grade = true;
            }
            RoutingStrategy::European => {
                requirements.european_compliance = true;
            }
            _ => {}
        }

        requirements
    }

    /// Estimate context length needed for the request
    fn estimate_context_length(&self, request: &ChatCompletionRequest) -> u32 {
        // Rough estimation based on message content
        let message_tokens: usize = request.messages
            .iter()
            .map(|msg| msg.content.len() / 4) // Rough token estimation
            .sum();

        (message_tokens + 1000) as u32 // Add buffer for response
    }

    /// Get intelligent candidate providers based on routing strategy and requirements
    async fn get_intelligent_candidates(
        &self,
        preferred_providers: Option<Vec<Provider>>,
        requirements: &ProviderRequirements,
        strategy: RoutingStrategy,
    ) -> Vec<Provider> {
        let available_providers = self.health_monitor.get_available_providers().await;
        
        let base_candidates: Vec<Provider> = if let Some(preferred) = preferred_providers {
            // Use preferred providers if they're available and meet requirements
            preferred
                .into_iter()
                .filter(|p| {
                    available_providers.contains(p) &&
                    self.provider_capabilities[p].meets_requirements(requirements)
                })
                .collect()
        } else {
            // Use strategy-based fallback chain
            strategy.fallback_chain()
                .into_iter()
                .filter(|p| {
                    available_providers.contains(p) &&
                    self.provider_capabilities[p].meets_requirements(requirements)
                })
                .collect()
        };

        // Apply intelligent filtering based on configuration
        let mut candidates = base_candidates;

        // European compliance filtering
        if self.config.intelligent_routing.regional_compliance && requirements.european_compliance {
            candidates.retain(|p| p.is_european_compliant());
        }

        // Enterprise grade filtering
        if requirements.enterprise_grade {
            candidates.retain(|p| self.provider_performance[p].enterprise_grade);
        }

        // Dynamic ordering based on health if enabled
        if self.config.intelligent_routing.dynamic_ordering {
            candidates = self.apply_dynamic_health_ordering(candidates).await;
        }

        candidates
    }

    /// Apply dynamic ordering based on current health metrics
    async fn apply_dynamic_health_ordering(&self, mut candidates: Vec<Provider>) -> Vec<Provider> {
        let provider_priorities = self.health_monitor.get_providers_by_priority().await;
        let priority_map: HashMap<Provider, f64> = provider_priorities.into_iter().collect();

        // Sort candidates by current health priority
        candidates.sort_by(|a, b| {
            let a_priority = priority_map.get(a).unwrap_or(&0.0);
            let b_priority = priority_map.get(b).unwrap_or(&0.0);
            b_priority.partial_cmp(a_priority).unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    /// Select the best provider using intelligent scoring algorithms
    async fn select_best_provider_intelligent(
        &self,
        candidates: Vec<Provider>,
        request: &ChatCompletionRequest,
        requirements: &ProviderRequirements,
        strategy: RoutingStrategy,
    ) -> Result<ProviderChoice, FallbackManagerError> {
        if candidates.is_empty() {
            return Err(FallbackManagerError::NoAvailableProviders {
                requirements: requirements.clone(),
            });
        }

        // Get provider priorities from health monitor
        let provider_priorities = self.health_monitor.get_providers_by_priority().await;
        let priority_map: HashMap<Provider, f64> = provider_priorities.into_iter().collect();

        // Get scoring weights for the strategy
        let weights = strategy.scoring_weights();

        // Score each candidate using intelligent multi-factor analysis
        let mut scored_candidates = Vec::new();
        
        for provider in candidates {
            let total_score = self.calculate_intelligent_provider_score(
                provider,
                request,
                requirements,
                &weights,
                &priority_map,
            ).await;
            
            if total_score >= self.config.intelligent_routing.min_confidence_threshold * 100.0 {
                scored_candidates.push((provider, total_score));
            }
        }

        if scored_candidates.is_empty() {
            return Err(FallbackManagerError::NoSuitableProvider {
                candidates: candidates,
                requirements: requirements.clone(),
            });
        }

        // Sort by score (highest first)
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_provider, score) = scored_candidates[0];
        let model = self.select_model_for_provider(best_provider, &request.model);
        
        let health = self.health_monitor.get_health(best_provider).await.unwrap();
        
        Ok(ProviderChoice {
            provider: best_provider,
            model,
            reason: format!(
                "Intelligently selected with score {:.2} using {:?} strategy (health: {:.2}, status: {:?})",
                score, strategy, health.success_rate, health.status
            ),
            confidence: score / 100.0, // Normalize to 0-1 range
            estimated_response_time: health.average_response_time,
            routing_strategy: strategy,
        })
    }

    /// Calculate intelligent provider score using multi-factor analysis
    async fn calculate_intelligent_provider_score(
        &self,
        provider: Provider,
        request: &ChatCompletionRequest,
        requirements: &ProviderRequirements,
        weights: &ScoringWeights,
        priority_map: &HashMap<Provider, f64>,
    ) -> f64 {
        let capabilities = &self.provider_capabilities[&provider];
        let performance = &self.provider_performance[&provider];
        let health_score = priority_map.get(&provider).unwrap_or(&0.0);

        // Base capability score
        let mut capability_score = self.calculate_capability_score(provider, request, requirements);
        
        // Performance-based scoring
        let reliability_score = performance.reliability_score * 100.0;
        let speed_score = performance.speed_score * 100.0;
        let quality_score = performance.quality_score * 100.0;
        let cost_score = if self.config.intelligent_routing.cost_aware {
            performance.cost_efficiency * 100.0
        } else {
            50.0 // Neutral score if cost awareness disabled
        };
        let enterprise_score = if performance.enterprise_grade { 100.0 } else { 50.0 };

        // Model compatibility bonus
        let model_compatibility = self.calculate_model_score(provider, &request.model);

        // European compliance bonus
        let compliance_bonus = if requirements.european_compliance && provider.is_european_compliant() {
            20.0
        } else {
            0.0
        };

        // Calculate weighted total score
        let total_score = (reliability_score * weights.reliability) +
                         (speed_score * weights.speed) +
                         (quality_score * weights.quality) +
                         (cost_score * weights.cost) +
                         (enterprise_score * weights.enterprise) +
                         (health_score * 0.3) + // Health weight
                         (capability_score * 0.2) + // Capability weight
                         (model_compatibility * 0.1) + // Model weight
                         compliance_bonus;

        debug!(
            provider = %provider.display_name(),
            total_score = total_score,
            reliability = reliability_score,
            speed = speed_score,
            quality = quality_score,
            cost = cost_score,
            enterprise = enterprise_score,
            health = health_score,
            capability = capability_score,
            model_compat = model_compatibility,
            compliance_bonus = compliance_bonus,
            "Calculated intelligent provider score"
        );

        total_score.max(0.0)
    }

    /// Calculate capability score for a provider (existing logic enhanced)
    fn calculate_capability_score(
        &self,
        provider: Provider,
        request: &ChatCompletionRequest,
        requirements: &ProviderRequirements,
    ) -> f64 {
        let capabilities = &self.provider_capabilities[&provider];
        let mut score = 50.0; // Base score

        // Streaming capability
        if requirements.streaming {
            if capabilities.streaming {
                score += 15.0;
            } else {
                return 0.0; // Hard requirement
            }
        }

        // Tool calling capability
        if requirements.tool_calling {
            if capabilities.tool_calling {
                score += 25.0;
            } else {
                return 0.0; // Hard requirement
            }
        }

        // Vision capability
        if requirements.vision {
            if capabilities.vision {
                score += 20.0;
            } else {
                return 0.0; // Hard requirement
            }
        }

        // System message support
        if requirements.system_messages && !capabilities.supports_system_messages {
            score -= 10.0;
        }

        // JSON mode support
        if requirements.json_mode && capabilities.supports_json_mode {
            score += 10.0;
        }

        // Token limits
        if capabilities.max_tokens >= requirements.min_max_tokens {
            score += 10.0;
        } else {
            return 0.0; // Hard requirement
        }

        if capabilities.max_context_length >= requirements.min_context_length {
            score += 10.0;
        } else {
            return 0.0; // Hard requirement
        }

        score
    }

    /// Calculate model compatibility score (existing logic)
    fn calculate_model_score(&self, provider: Provider, requested_model: &str) -> f64 {
        if provider.supports_model(requested_model) {
            return 40.0; // Exact match
        }

        // Check for compatible models (simplified)
        let supported_models = provider.supported_models();
        for model in supported_models {
            if model.contains("gpt") && requested_model.contains("gpt") {
                return 20.0; // GPT family match
            }
            if model.contains("claude") && requested_model.contains("claude") {
                return 20.0; // Claude family match
            }
            if model.contains("llama") && requested_model.contains("llama") {
                return 15.0; // Llama family match
            }
            if model.contains("mistral") && requested_model.contains("mistral") {
                return 15.0; // Mistral family match
            }
            if model.contains("command") && requested_model.contains("command") {
                return 15.0; // Cohere family match
            }
        }

        10.0 // Default compatibility score
    }

    /// Select appropriate model for provider
    fn select_model_for_provider(&self, provider: Provider, requested_model: &str) -> String {
        if provider.supports_model(requested_model) {
            requested_model.to_string()
        } else {
            // Use default model for provider
            provider.default_model().to_string()
        }
    }

    /// Get intelligently selected fallback provider when current provider fails
    async fn get_intelligent_fallback_provider(
        &self,
        failed_provider: Provider,
        error: &ProviderErrorType,
        request_id: &str,
    ) -> Option<Provider> {
        let available_providers = self.health_monitor.get_available_providers().await;
        
        // Filter out the failed provider
        let fallback_candidates: Vec<Provider> = available_providers
            .into_iter()
            .filter(|p| *p != failed_provider)
            .collect();

        if fallback_candidates.is_empty() {
            return None;
        }

        // Get current routing strategy for this request
        let routing_strategy = if let Some(metrics) = self.active_requests.read().await.get(request_id) {
            metrics.routing_strategy
        } else {
            self.config.routing_strategy
        };

        // Prioritize based on error type and routing strategy
        let prioritized_candidates = self.prioritize_fallback_candidates(
            fallback_candidates,
            failed_provider,
            error,
            routing_strategy,
        ).await;

        // Get provider with best health score from prioritized list
        let provider_priorities = self.health_monitor.get_providers_by_priority().await;
        for (provider, score) in provider_priorities {
            if prioritized_candidates.contains(&provider) && score > 0.0 {
                debug!(
                    failed_provider = %failed_provider.display_name(),
                    fallback_provider = %provider.display_name(),
                    error = ?error,
                    routing_strategy = ?routing_strategy,
                    health_score = score,
                    "Selected intelligent fallback provider"
                );
                return Some(provider);
            }
        }

        None
    }

    /// Prioritize fallback candidates based on failure type and routing strategy
    async fn prioritize_fallback_candidates(
        &self,
        mut candidates: Vec<Provider>,
        failed_provider: Provider,
        error: &ProviderErrorType,
        routing_strategy: RoutingStrategy,
    ) -> Vec<Provider> {
        // Apply strategy-specific prioritization
        match routing_strategy {
            RoutingStrategy::Speed => {
                candidates.sort_by(|a, b| {
                    let a_speed = self.provider_performance[a].speed_score;
                    let b_speed = self.provider_performance[b].speed_score;
                    b_speed.partial_cmp(&a_speed).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            RoutingStrategy::Enterprise => {
                // For enterprise strategy, prioritize enterprise-grade providers
                candidates.retain(|p| self.provider_performance[p].enterprise_grade);
            }
            RoutingStrategy::European => {
                // For European strategy, prioritize compliant providers
                candidates.retain(|p| p.is_european_compliant());
            }
            RoutingStrategy::Cost => {
                // For cost strategy, prioritize cost-efficient providers
                candidates.sort_by(|a, b| {
                    let a_cost = self.provider_performance[a].cost_efficiency;
                    let b_cost = self.provider_performance[b].cost_efficiency;
                    b_cost.partial_cmp(&a_cost).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            _ => {
                // For balanced and quality strategies, use default fallback chain
                let fallback_chain = routing_strategy.fallback_chain();
                candidates.sort_by(|a, b| {
                    let a_pos = fallback_chain.iter().position(|p| p == a).unwrap_or(usize::MAX);
                    let b_pos = fallback_chain.iter().position(|p| p == b).unwrap_or(usize::MAX);
                    a_pos.cmp(&b_pos)
                });
            }
        }

        // Apply error-type specific logic
        match error {
            ProviderErrorType::RateLimit { .. } => {
                // For rate limiting, avoid providers known for strict limits
                if failed_provider == Provider::Groq {
                    // Groq has strict rate limits, prioritize providers with higher limits
                    candidates.retain(|p| *p != Provider::Groq);
                }
            }
            ProviderErrorType::ModelNotAvailable => {
                // For model availability issues, prioritize providers with broader model support
                candidates.sort_by(|a, b| {
                    let a_models = a.supported_models().len();
                    let b_models = b.supported_models().len();
                    b_models.cmp(&a_models)
                });
            }
            _ => {
                // For other errors, use default prioritization
            }
        }

        candidates
    }

    /// Clean up completed requests (should be called periodically)
    pub async fn cleanup_completed_requests(&self) {
        let mut active_requests = self.active_requests.write().await;
        let now = Instant::now();
        
        active_requests.retain(|_, metrics| {
            // Keep requests that are less than 5 minutes old
            now.duration_since(metrics.start_time) < Duration::from_secs(300)
        });
    }

    /// Get routing statistics for monitoring
    pub async fn get_routing_statistics(&self) -> RoutingStatistics {
        let active_requests = self.active_requests.read().await;
        
        let mut strategy_counts = HashMap::new();
        let mut provider_counts = HashMap::new();
        let mut total_requests = 0;
        let mut successful_requests = 0;
        let mut fallback_used_count = 0;

        for metrics in active_requests.values() {
            total_requests += 1;
            
            if metrics.success {
                successful_requests += 1;
            }
            
            if metrics.fallback_used {
                fallback_used_count += 1;
            }
            
            *strategy_counts.entry(metrics.routing_strategy).or_insert(0) += 1;
            *provider_counts.entry(metrics.provider).or_insert(0) += 1;
        }

        let success_rate = if total_requests > 0 {
            successful_requests as f64 / total_requests as f64
        } else {
            1.0
        };

        let fallback_rate = if total_requests > 0 {
            fallback_used_count as f64 / total_requests as f64
        } else {
            0.0
        };

        RoutingStatistics {
            total_requests,
            successful_requests,
            success_rate,
            fallback_used_count,
            fallback_rate,
            strategy_distribution: strategy_counts,
            provider_distribution: provider_counts,
        }
    }
}

/// Error types for fallback manager operations
#[derive(Debug, thiserror::Error)]
pub enum FallbackManagerError {
    #[error("No available providers meet the requirements: {requirements:?}")]
    NoAvailableProviders {
        requirements: ProviderRequirements,
    },
    
    #[error("No suitable provider found among candidates {candidates:?} for requirements: {requirements:?}")]
    NoSuitableProvider {
        candidates: Vec<Provider>,
        requirements: ProviderRequirements,
    },
    
    #[error("Provider configuration error: {message}")]
    ConfigurationError {
        message: String,
    },
    
    #[error("Health monitoring error: {source}")]
    HealthMonitorError {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Routing statistics for monitoring and observability
#[derive(Debug, Clone)]
pub struct RoutingStatistics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub success_rate: f64,
    pub fallback_used_count: usize,
    pub fallback_rate: f64,
    pub strategy_distribution: HashMap<RoutingStrategy, usize>,
    pub provider_distribution: HashMap<Provider, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::{ChatMessage, MessageRole};

    fn create_test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello, world!".to_string(),
                name: None,
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        }
    }

    #[tokio::test]
    async fn test_intelligent_fallback_manager_creation() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        // Verify all 8 providers are included
        assert_eq!(manager.provider_capabilities.len(), 8);
        assert_eq!(manager.provider_performance.len(), 8);
        
        // Verify all providers have capabilities and performance profiles
        for provider in Provider::all() {
            assert!(manager.provider_capabilities.contains_key(&provider));
            assert!(manager.provider_performance.contains_key(&provider));
        }
    }

    #[tokio::test]
    async fn test_intelligent_route_request() {
        let config = FallbackConfig {
            routing_strategy: RoutingStrategy::Speed,
            ..FallbackConfig::default()
        };
        let manager = FallbackManager::new(config);
        let request = create_test_request();
        
        // This would require mocking the health monitor
        // For now, just verify the function signature
        let result = manager.route_request(&request, None, Some(RoutingStrategy::Quality)).await;
        // In a real test, we'd mock the health monitor to return available providers
    }

    #[tokio::test]
    async fn test_intelligent_requirements_extraction() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        let mut request = create_test_request();
        request.stream = Some(true);
        request.tools = Some(vec![]); // Empty tools array
        
        let requirements = manager.extract_requirements(&request, RoutingStrategy::European);
        
        assert!(requirements.streaming);
        assert!(!requirements.tool_calling); // Empty tools array
        assert!(requirements.european_compliance); // European strategy sets this
    }

    #[tokio::test]
    async fn test_intelligent_provider_scoring() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        let request = create_test_request();
        let requirements = ProviderRequirements::default();
        let weights = RoutingStrategy::Speed.scoring_weights();
        let priority_map = HashMap::new();
        
        let score = manager.calculate_intelligent_provider_score(
            Provider::Groq, // Should score high for speed
            &request,
            &requirements,
            &weights,
            &priority_map,
        ).await;
        
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_routing_strategy_fallback_chains() {
        // Test that each strategy returns the expected provider order
        let speed_chain = RoutingStrategy::Speed.fallback_chain();
        assert_eq!(speed_chain[0], Provider::Groq); // Fastest should be first
        
        let enterprise_chain = RoutingStrategy::Enterprise.fallback_chain();
        assert_eq!(enterprise_chain[0], Provider::AzureOpenAI); // Enterprise should lead with Azure
        
        let european_chain = RoutingStrategy::European.fallback_chain();
        assert_eq!(european_chain[0], Provider::Mistral); // European should lead with Mistral
    }

    #[tokio::test]
    async fn test_provider_failure_handling() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        let action = manager.handle_provider_failure(
            "test-request-id",
            Provider::OpenAI,
            ProviderErrorType::RateLimit { 
                reset_time: std::time::SystemTime::now() + Duration::from_secs(60) 
            },
            1
        ).await;
        
        // Should either retry or switch to fallback
        match action {
            FallbackAction::Retry { provider, attempt, .. } => {
                assert_eq!(provider, Provider::OpenAI);
                assert_eq!(attempt, 2);
            }
            FallbackAction::Switch { from, to, .. } => {
                assert_eq!(from, Provider::OpenAI);
                assert_ne!(to, Provider::OpenAI);
            }
            FallbackAction::Fail { .. } => {
                // This could happen if no providers are available
            }
        }
    }

    #[tokio::test]
    async fn test_success_recording() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        // Record a successful request
        manager.record_success(
            "test-request-id",
            Provider::VertexAI,
            Duration::from_millis(500)
        ).await;
        
        // Verify the health monitor was updated (would need mocking in real test)
    }

    #[tokio::test]
    async fn test_routing_statistics() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        // Add some mock request metrics
        {
            let mut active_requests = manager.active_requests.write().await;
            let mut metrics1 = RequestMetrics::new(
                "req1".to_string(),
                Provider::OpenAI,
                "gpt-4o".to_string()
            );
            metrics1.success = true;
            metrics1.routing_strategy = RoutingStrategy::Quality;
            
            let mut metrics2 = RequestMetrics::new(
                "req2".to_string(),
                Provider::Groq,
                "llama-3.1-70b-versatile".to_string()
            );
            metrics2.fallback_used = true;
            metrics2.routing_strategy = RoutingStrategy::Speed;
            
            active_requests.insert("req1".to_string(), metrics1);
            active_requests.insert("req2".to_string(), metrics2);
        }
        
        let stats = manager.get_routing_statistics().await;
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.fallback_used_count, 1);
        assert_eq!(stats.success_rate, 0.5);
        assert_eq!(stats.fallback_rate, 0.5);
    }

    #[tokio::test]
    async fn test_dynamic_strategy_update() {
        let config = FallbackConfig {
            routing_strategy: RoutingStrategy::Balanced,
            ..FallbackConfig::default()
        };
        let mut manager = FallbackManager::new(config);
        
        assert_eq!(manager.config.routing_strategy, RoutingStrategy::Balanced);
        
        manager.update_routing_strategy(RoutingStrategy::Speed).await;
        
        assert_eq!(manager.config.routing_strategy, RoutingStrategy::Speed);
        assert_eq!(manager.config.provider_priority, RoutingStrategy::Speed.fallback_chain());
    }

    #[test]
    fn test_capability_scoring() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        let request = create_test_request();
        
        let requirements = ProviderRequirements {
            streaming: true,
            tool_calling: true,
            ..ProviderRequirements::default()
        };
        
        // Test provider that meets all requirements
        let openai_score = manager.calculate_capability_score(
            Provider::OpenAI,
            &request,
            &requirements
        );
        assert!(openai_score > 50.0);
        
        // Test provider that doesn't support vision when required
        let vision_requirements = ProviderRequirements {
            vision: true,
            ..requirements
        };
        
        let groq_score = manager.calculate_capability_score(
            Provider::Groq,
            &request,
            &vision_requirements
        );
        assert_eq!(groq_score, 0.0); // Should fail hard requirement
    }

    #[test]
    fn test_model_compatibility_scoring() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        // Test exact model match
        let exact_score = manager.calculate_model_score(Provider::OpenAI, "gpt-4o");
        assert_eq!(exact_score, 40.0);
        
        // Test family match
        let family_score = manager.calculate_model_score(Provider::OpenAI, "gpt-3.5-turbo");
        assert!(family_score >= 20.0);
        
        // Test no match
        let no_match_score = manager.calculate_model_score(Provider::OpenAI, "claude-3-sonnet");
        assert_eq!(no_match_score, 10.0);
    }

    #[test]
    fn test_european_compliance_filtering() {
        // Test that European strategy properly filters providers
        assert!(Provider::Mistral.is_european_compliant());
        assert!(Provider::Cohere.is_european_compliant());
        assert!(Provider::AzureOpenAI.is_european_compliant());
        assert!(!Provider::OpenAI.is_european_compliant());
        assert!(!Provider::Groq.is_european_compliant());
        
        let european_chain = RoutingStrategy::European.fallback_chain();
        // First few providers should be European compliant
        assert!(european_chain[0].is_european_compliant());
        assert!(european_chain[1].is_european_compliant());
    }

    #[test]
    fn test_performance_characteristics() {
        // Test that performance profiles are consistent with expectations
        let groq_perf = Provider::Groq.performance_characteristics();
        assert!(groq_perf.speed_score > 0.95); // Groq should be fastest
        
        let openai_perf = Provider::OpenAI.performance_characteristics();
        assert!(openai_perf.reliability_score > 0.95); // OpenAI should be most reliable
        
        let mistral_perf = Provider::Mistral.performance_characteristics();
        assert!(mistral_perf.cost_efficiency > 0.90); // Mistral should be cost effective
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::{ChatMessage, MessageRole};

    fn create_test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello, world!".to_string(),
                name: None,
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        }
    }

    #[tokio::test]
    async fn test_intelligent_fallback_manager_creation() {
        let config = FallbackConfig::default();
        let manager = FallbackManager::new(config);
        
        // Verify all 8 providers are included
        assert_eq!(manager.provider_capabilities.len(), 8);
        assert_eq!(manager.provider_performance.len(), 8);
        
        // Verify all providers have capabilities and performance profiles
        for provider in Provider::all() {
            
            assert!(manager.provider_capabilities.contains_key(&provider));
            assert!(manager.provider_performance.contains_key(&provider));
        }
    }
}