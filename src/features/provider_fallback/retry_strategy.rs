// src/features/provider_fallback/retry_strategy.rs
//
// Intelligent retry logic with exponential backoff and jitter

use std::time::Duration;
use rand::Rng;
use tracing::{debug, warn};

use super::models::{RetryConfig, ProviderErrorType, Provider, FallbackAction};

/// Retry strategy implementation with configurable backoff algorithms
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    /// Create a new retry strategy with the given configuration
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Create a retry strategy with default configuration
    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Determine if an error should be retried
    pub fn should_retry(&self, error: &ProviderErrorType, attempt: u32) -> bool {
        // Check if we've exceeded max attempts
        if attempt >= self.config.max_attempts {
            debug!("Max retry attempts ({}) exceeded", self.config.max_attempts);
            return false;
        }

        // Check if error type is retryable
        let retryable = error.is_retryable();
        if !retryable {
            debug!("Error type {:?} is not retryable", error);
            return false;
        }

        debug!(
            "Error {:?} is retryable, attempt {} of {}",
            error, attempt, self.config.max_attempts
        );
        true
    }

    /// Calculate the delay before the next retry attempt
    pub fn calculate_delay(&self, attempt: u32, error: &ProviderErrorType) -> Duration {
        let base_delay = match error {
            // Rate limit errors need special handling
            ProviderErrorType::RateLimit { .. } => {
                // Use linear backoff for rate limits to be more conservative
                Duration::from_millis(self.config.base_delay.as_millis() as u64 * attempt as u64)
            }
            // Service unavailable errors might recover quickly
            ProviderErrorType::ServiceUnavailable => {
                // Use shorter delays for service unavailable
                Duration::from_millis(self.config.base_delay.as_millis() as u64 / 2)
            }
            // Network errors and timeouts use exponential backoff
            ProviderErrorType::NetworkError | ProviderErrorType::Timeout => {
                let exponential_delay = self.config.base_delay.as_millis() as f64 
                    * self.config.exponential_base.powi(attempt as i32);
                Duration::from_millis(exponential_delay as u64)
            }
            // Internal errors use standard exponential backoff
            _ => {
                let exponential_delay = self.config.base_delay.as_millis() as f64 
                    * self.config.exponential_base.powi((attempt - 1) as i32);
                Duration::from_millis(exponential_delay as u64)
            }
        };

        // Apply jitter if enabled
        let delay_with_jitter = if self.config.jitter {
            self.add_jitter(base_delay)
        } else {
            base_delay
        };

        // Cap at max delay
        let final_delay = std::cmp::min(delay_with_jitter, self.config.max_delay);

        debug!(
            "Calculated retry delay for attempt {}: {:?} (base: {:?}, with_jitter: {:?})",
            attempt, final_delay, base_delay, delay_with_jitter
        );

        final_delay
    }

    /// Add jitter to prevent thundering herd effect
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.8..1.2); // ±20% jitter
        let jittered_ms = (delay.as_millis() as f64 * jitter_factor) as u64;
        Duration::from_millis(jittered_ms)
    }

    /// Create a retry action for a failed request
    pub fn create_retry_action(
        &self,
        provider: Provider,
        error: &ProviderErrorType,
        attempt: u32,
    ) -> FallbackAction {
        if self.should_retry(error, attempt) {
            let delay = self.calculate_delay(attempt, error);
            FallbackAction::Retry {
                provider,
                delay,
                attempt: attempt + 1,
            }
        } else {
            let reason = if attempt >= self.config.max_attempts {
                format!(
                    "Max retry attempts ({}) exceeded for provider {}",
                    self.config.max_attempts,
                    provider.display_name()
                )
            } else {
                format!(
                    "Error {:?} is not retryable for provider {}",
                    error,
                    provider.display_name()
                )
            };

            FallbackAction::Fail {
                reason,
                last_errors: vec![(provider, error.clone())],
            }
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Update the retry configuration
    pub fn update_config(&mut self, config: RetryConfig) {
        self.config = config;
    }
}

/// Specialized retry strategies for different scenarios
impl RetryStrategy {
    /// Create a fast retry strategy for interactive use cases
    pub fn fast() -> Self {
        Self::new(RetryConfig {
            max_attempts: 2,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(5),
            exponential_base: 1.5,
            jitter: true,
        })
    }

    /// Create a conservative retry strategy for rate-limited scenarios
    pub fn conservative() -> Self {
        Self::new(RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            exponential_base: 2.0,
            jitter: true,
        })
    }

    /// Create an aggressive retry strategy for high-availability scenarios
    pub fn aggressive() -> Self {
        Self::new(RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 1.2,
            jitter: true,
        })
    }

    /// Create a batch processing retry strategy
    pub fn batch() -> Self {
        Self::new(RetryConfig {
            max_attempts: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300),
            exponential_base: 2.0,
            jitter: true,
        })
    }
}

/// Retry context for tracking retry state across attempts
#[derive(Debug, Clone)]
pub struct RetryContext {
    pub request_id: String,
    pub provider: Provider,
    pub original_error: ProviderErrorType,
    pub attempt_count: u32,
    pub total_delay: Duration,
    pub start_time: std::time::Instant,
}

impl RetryContext {
    /// Create a new retry context
    pub fn new(request_id: String, provider: Provider, error: ProviderErrorType) -> Self {
        Self {
            request_id,
            provider,
            original_error: error,
            attempt_count: 1,
            total_delay: Duration::ZERO,
            start_time: std::time::Instant::now(),
        }
    }

    /// Update context for next retry attempt
    pub fn next_attempt(&mut self, delay: Duration) {
        self.attempt_count += 1;
        self.total_delay += delay;
    }

    /// Get total elapsed time since first attempt
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if retry context has exceeded time limits
    pub fn is_expired(&self, max_total_time: Duration) -> bool {
        self.elapsed_time() > max_total_time
    }
}

/// Advanced retry decision based on provider patterns and historical data
#[derive(Debug)]
pub struct AdaptiveRetryStrategy {
    base_strategy: RetryStrategy,
    provider_patterns: std::collections::HashMap<Provider, ProviderRetryPattern>,
}

/// Provider-specific retry patterns learned from historical data
#[derive(Debug, Clone)]
struct ProviderRetryPattern {
    provider: Provider,
    typical_recovery_time: Duration,
    success_rate_by_attempt: Vec<f64>,
    preferred_delay_multiplier: f64,
}

impl AdaptiveRetryStrategy {
    /// Create a new adaptive retry strategy
    pub fn new(base_strategy: RetryStrategy) -> Self {
        let mut provider_patterns = std::collections::HashMap::new();
        
        // Initialize with default patterns for known providers
        for provider in Provider::all() {
            provider_patterns.insert(provider, Self::default_pattern_for_provider(provider));
        }

        Self {
            base_strategy,
            provider_patterns,
        }
    }

    /// Get default retry pattern for a provider
    fn default_pattern_for_provider(provider: Provider) -> ProviderRetryPattern {
        match provider {
            Provider::VertexAI => ProviderRetryPattern {
                provider,
                typical_recovery_time: Duration::from_secs(30),
                success_rate_by_attempt: vec![0.3, 0.7, 0.9],
                preferred_delay_multiplier: 1.0,
            },
            Provider::Groq => ProviderRetryPattern {
                provider,
                typical_recovery_time: Duration::from_secs(15),
                success_rate_by_attempt: vec![0.5, 0.8, 0.95],
                preferred_delay_multiplier: 0.8, // Groq typically recovers faster
            },
        }
    }

    /// Make adaptive retry decision based on provider patterns
    pub fn should_retry_adaptive(
        &self,
        provider: Provider,
        error: &ProviderErrorType,
        attempt: u32,
        context: &RetryContext,
    ) -> bool {
        // First check base strategy
        if !self.base_strategy.should_retry(error, attempt) {
            return false;
        }

        // Check provider-specific patterns
        if let Some(pattern) = self.provider_patterns.get(&provider) {
            // Check if we've exceeded typical recovery time
            if context.elapsed_time() > pattern.typical_recovery_time * 2 {
                warn!(
                    "Retry context for {} has exceeded typical recovery time, stopping retries",
                    provider.display_name()
                );
                return false;
            }

            // Check success rate for this attempt
            let attempt_index = (attempt - 1) as usize;
            if let Some(success_rate) = pattern.success_rate_by_attempt.get(attempt_index) {
                if *success_rate < 0.1 {
                    debug!(
                        "Success rate for attempt {} with {} is too low ({:.2}), stopping retries",
                        attempt, provider.display_name(), success_rate
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Calculate adaptive delay based on provider patterns
    pub fn calculate_adaptive_delay(
        &self,
        provider: Provider,
        error: &ProviderErrorType,
        attempt: u32,
    ) -> Duration {
        let base_delay = self.base_strategy.calculate_delay(attempt, error);

        if let Some(pattern) = self.provider_patterns.get(&provider) {
            let adjusted_delay_ms = (base_delay.as_millis() as f64 * pattern.preferred_delay_multiplier) as u64;
            Duration::from_millis(adjusted_delay_ms)
        } else {
            base_delay
        }
    }

    /// Update provider patterns based on observed retry outcomes
    pub fn update_pattern(
        &mut self,
        provider: Provider,
        attempt: u32,
        success: bool,
        response_time: Duration,
    ) {
        if let Some(pattern) = self.provider_patterns.get_mut(&provider) {
            // Update success rate for this attempt
            let attempt_index = (attempt - 1) as usize;
            if attempt_index < pattern.success_rate_by_attempt.len() {
                // Use exponential moving average to update success rate
                let alpha = 0.1;
                let current_rate = pattern.success_rate_by_attempt[attempt_index];
                let new_rate = if success { 1.0 } else { 0.0 };
                pattern.success_rate_by_attempt[attempt_index] = 
                    alpha * new_rate + (1.0 - alpha) * current_rate;
            }

            // Update typical recovery time if this was a successful retry
            if success && attempt > 1 {
                let alpha = 0.05; // Slower adaptation for recovery time
                let current_time = pattern.typical_recovery_time.as_millis() as f64;
                let observed_time = response_time.as_millis() as f64;
                let new_time = alpha * observed_time + (1.0 - alpha) * current_time;
                pattern.typical_recovery_time = Duration::from_millis(new_time as u64);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_retry_strategy_creation() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.config().max_attempts, 3);
        assert!(strategy.config().jitter);
    }

    #[test]
    fn test_should_retry_logic() {
        let strategy = RetryStrategy::default();

        // Retryable errors
        assert!(strategy.should_retry(&ProviderErrorType::Timeout, 1));
        assert!(strategy.should_retry(&ProviderErrorType::ServiceUnavailable, 1));
        assert!(strategy.should_retry(&ProviderErrorType::NetworkError, 1));

        // Non-retryable errors
        assert!(!strategy.should_retry(&ProviderErrorType::Authentication, 1));
        assert!(!strategy.should_retry(&ProviderErrorType::Authorization, 1));
        assert!(!strategy.should_retry(&ProviderErrorType::InvalidRequest, 1));

        // Max attempts exceeded
        assert!(!strategy.should_retry(&ProviderErrorType::Timeout, 5));
    }

    #[test]
    fn test_delay_calculation() {
        let strategy = RetryStrategy::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: false, // Disable jitter for predictable testing
        });

        // Test exponential backoff
        let delay1 = strategy.calculate_delay(1, &ProviderErrorType::Timeout);
        let delay2 = strategy.calculate_delay(2, &ProviderErrorType::Timeout);
        
        // Second attempt should have longer delay (exponential backoff)
        assert!(delay2 >= delay1);

        // Rate limit should use linear backoff
        let rate_limit_error = ProviderErrorType::RateLimit {
            reset_time: SystemTime::now() + Duration::from_secs(60)
        };
        let rate_delay1 = strategy.calculate_delay(1, &rate_limit_error);
        let rate_delay2 = strategy.calculate_delay(2, &rate_limit_error);
        
        // Should be roughly linear (2x for attempt 2)
        assert_eq!(rate_delay2.as_millis(), rate_delay1.as_millis() * 2);
    }

    #[test]
    fn test_jitter_application() {
        let strategy = RetryStrategy::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: true,
        });

        // Test multiple delay calculations to ensure jitter varies
        let delays: Vec<_> = (0..10)
            .map(|_| strategy.calculate_delay(1, &ProviderErrorType::Timeout))
            .collect();

        // Should have some variation due to jitter
        let min_delay = delays.iter().min().unwrap();
        let max_delay = delays.iter().max().unwrap();
        assert!(max_delay > min_delay);
    }

    #[test]
    fn test_specialized_strategies() {
        let fast = RetryStrategy::fast();
        let conservative = RetryStrategy::conservative();
        let aggressive = RetryStrategy::aggressive();

        assert!(fast.config().max_attempts < conservative.config().max_attempts);
        assert!(aggressive.config().base_delay < conservative.config().base_delay);
        assert!(fast.config().base_delay < conservative.config().base_delay);
    }

    #[test]
    fn test_retry_context() {
        let mut context = RetryContext::new(
            "test-123".to_string(),
            Provider::VertexAI,
            ProviderErrorType::Timeout,
        );

        assert_eq!(context.attempt_count, 1);
        assert_eq!(context.total_delay, Duration::ZERO);

        context.next_attempt(Duration::from_millis(1000));
        assert_eq!(context.attempt_count, 2);
        assert_eq!(context.total_delay, Duration::from_millis(1000));
    }

    #[test]
    fn test_create_retry_action() {
        let strategy = RetryStrategy::default();

        // Should create retry action for retryable error
        let action = strategy.create_retry_action(
            Provider::VertexAI,
            &ProviderErrorType::Timeout,
            1,
        );
        
        if let FallbackAction::Retry { provider, attempt, .. } = action {
            assert_eq!(provider, Provider::VertexAI);
            assert_eq!(attempt, 2);
        } else {
            panic!("Expected retry action");
        }

        // Should create fail action for non-retryable error
        let action = strategy.create_retry_action(
            Provider::VertexAI,
            &ProviderErrorType::Authentication,
            1,
        );
        
        assert!(matches!(action, FallbackAction::Fail { .. }));
    }

    #[test]
    fn test_adaptive_retry_strategy() {
        let base_strategy = RetryStrategy::default();
        let mut adaptive = AdaptiveRetryStrategy::new(base_strategy);

        let context = RetryContext::new(
            "test-123".to_string(),
            Provider::VertexAI,
            ProviderErrorType::Timeout,
        );

        // Should initially allow retries
        assert!(adaptive.should_retry_adaptive(
            Provider::VertexAI,
            &ProviderErrorType::Timeout,
            1,
            &context
        ));

        // Update pattern with failure
        adaptive.update_pattern(Provider::VertexAI, 1, false, Duration::from_secs(1));

        // Should still allow retries after single failure
        assert!(adaptive.should_retry_adaptive(
            Provider::VertexAI,
            &ProviderErrorType::Timeout,
            1,
            &context
        ));
    }
}