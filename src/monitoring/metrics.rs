//! Comprehensive metrics collection system for Codex CLI observability
//! 
//! This module provides detailed metrics collection for all aspects of the Codex CLI proxy,
//! including request processing, tool execution, conversation management, and system performance.

use anyhow::Result;
use prometheus::{
    Registry, Counter, Histogram, Gauge, HistogramOpts, Opts, CounterVec, HistogramVec, GaugeVec
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
    common::{ChatMessage, ToolCall},
};

/// Comprehensive metrics collector for Codex CLI operations
pub struct MetricsCollector {
    // Core metrics
    pub registry: Arc<Registry>,
    
    // Request metrics
    pub requests_total: CounterVec,
    pub request_duration: HistogramVec,
    pub request_size_bytes: HistogramVec,
    pub response_size_bytes: HistogramVec,
    
    // Conversation metrics
    pub conversations_active: Gauge,
    pub conversation_turns: CounterVec,
    pub conversation_duration: HistogramVec,
    
    // Tool execution metrics
    pub tool_calls_total: CounterVec,
    pub tool_execution_duration: HistogramVec,
    pub tool_success_rate: CounterVec,
    
    // Provider metrics
    pub provider_requests: CounterVec,
    pub provider_errors: CounterVec,
    pub provider_response_time: HistogramVec,
    pub provider_availability: GaugeVec,
    
    // Streaming metrics
    pub streaming_requests: Counter,
    pub streaming_chunks: HistogramVec,
    pub streaming_duration: Histogram,
    
    // System metrics
    pub system_memory_usage: Gauge,
    pub system_cpu_usage: Gauge,
    pub database_connections_active: Gauge,
    pub database_query_duration: Histogram,
    
    // Business metrics
    pub successful_code_generations: Counter,
    pub patch_applications: Counter,
    pub file_operations: CounterVec,
    pub unique_users: Gauge,
    
    // Internal state
    enabled: bool,
    start_time: Instant,
    active_requests: Arc<RwLock<HashMap<String, RequestMetrics>>>,
}

/// Metrics for tracking individual requests
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub request_id: String,
    pub start_time: Instant,
    pub model: String,
    pub provider: Option<String>,
    pub conversation_id: Option<String>,
    pub user_id: Option<String>,
    pub tool_calls_count: usize,
    pub streaming: bool,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(registry: Arc<Registry>, enabled: bool) -> Result<Self> {
        info!("Initializing metrics collector (enabled: {})", enabled);

        // Request metrics
        let requests_total = CounterVec::new(
            Opts::new("codex_requests_total", "Total number of Codex CLI requests")
                .namespace("codex_cli"),
            &["model", "provider", "status", "client_type"]
        )?;

        let request_duration = HistogramVec::new(
            HistogramOpts::new("codex_request_duration_seconds", "Request processing duration")
                .namespace("codex_cli")
                .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
            &["model", "provider", "operation_type"]
        )?;

        let request_size_bytes = HistogramVec::new(
            HistogramOpts::new("codex_request_size_bytes", "Request payload size in bytes")
                .namespace("codex_cli")
                .buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0]),
            &["model", "content_type"]
        )?;

        let response_size_bytes = HistogramVec::new(
            HistogramOpts::new("codex_response_size_bytes", "Response payload size in bytes")
                .namespace("codex_cli")
                .buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0]),
            &["model", "provider", "has_tools"]
        )?;

        // Conversation metrics
        let conversations_active = Gauge::new(
            "codex_conversations_active", "Number of active conversations"
        )?;

        let conversation_turns = CounterVec::new(
            Opts::new("codex_conversation_turns_total", "Total conversation turns")
                .namespace("codex_cli"),
            &["conversation_type", "turn_number_bucket"]
        )?;

        let conversation_duration = HistogramVec::new(
            HistogramOpts::new("codex_conversation_duration_seconds", "Conversation duration")
                .namespace("codex_cli")
                .buckets(vec![60.0, 300.0, 900.0, 1800.0, 3600.0]),
            &["conversation_type", "completion_reason"]
        )?;

        // Tool execution metrics
        let tool_calls_total = CounterVec::new(
            Opts::new("codex_tool_calls_total", "Total tool calls executed")
                .namespace("codex_cli"),
            &["tool_name", "provider", "status"]
        )?;

        let tool_execution_duration = HistogramVec::new(
            HistogramOpts::new("codex_tool_execution_duration_seconds", "Tool execution duration")
                .namespace("codex_cli")
                .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]),
            &["tool_name", "operation_type"]
        )?;

        let tool_success_rate = CounterVec::new(
            Opts::new("codex_tool_success_total", "Tool execution success/failure counts")
                .namespace("codex_cli"),
            &["tool_name", "success"]
        )?;

        // Provider metrics
        let provider_requests = CounterVec::new(
            Opts::new("codex_provider_requests_total", "Requests per provider")
                .namespace("codex_cli"),
            &["provider", "model", "status"]
        )?;

        let provider_errors = CounterVec::new(
            Opts::new("codex_provider_errors_total", "Provider errors")
                .namespace("codex_cli"),
            &["provider", "error_type", "retryable"]
        )?;

        let provider_response_time = HistogramVec::new(
            HistogramOpts::new("codex_provider_response_time_seconds", "Provider response time")
                .namespace("codex_cli")
                .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]),
            &["provider", "model"]
        )?;

        let provider_availability = GaugeVec::new(
            Opts::new("codex_provider_availability_ratio", "Provider availability ratio")
                .namespace("codex_cli"),
            &["provider"]
        )?;

        // Streaming metrics
        let streaming_requests = Counter::new(
            "codex_streaming_requests_total", "Total streaming requests"
        )?;

        let streaming_chunks = HistogramVec::new(
            HistogramOpts::new("codex_streaming_chunks_total", "Number of chunks per streaming request")
                .namespace("codex_cli")
                .buckets(vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0]),
            &["model", "content_type"]
        )?;

        let streaming_duration = Histogram::new(
            HistogramOpts::new("codex_streaming_duration_seconds", "Streaming response duration")
                .namespace("codex_cli")
                .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0])
        )?;

        // System metrics
        let system_memory_usage = Gauge::new(
            "codex_system_memory_usage_bytes", "System memory usage in bytes"
        )?;

        let system_cpu_usage = Gauge::new(
            "codex_system_cpu_usage_ratio", "System CPU usage ratio"
        )?;

        let database_connections_active = Gauge::new(
            "codex_database_connections_active", "Active database connections"
        )?;

        let database_query_duration = Histogram::new(
            HistogramOpts::new("codex_database_query_duration_seconds", "Database query duration")
                .namespace("codex_cli")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
        )?;

        // Business metrics
        let successful_code_generations = Counter::new(
            "codex_successful_code_generations_total", "Successful code generations"
        )?;

        let patch_applications = Counter::new(
            "codex_patch_applications_total", "Total patch applications"
        )?;

        let file_operations = CounterVec::new(
            Opts::new("codex_file_operations_total", "File operations by type")
                .namespace("codex_cli"),
            &["operation_type", "file_extension", "success"]
        )?;

        let unique_users = Gauge::new(
            "codex_unique_users_active", "Number of unique active users"
        )?;

        // Register all metrics
        if enabled {
            registry.register(Box::new(requests_total.clone()))?;
            registry.register(Box::new(request_duration.clone()))?;
            registry.register(Box::new(request_size_bytes.clone()))?;
            registry.register(Box::new(response_size_bytes.clone()))?;
            registry.register(Box::new(conversations_active.clone()))?;
            registry.register(Box::new(conversation_turns.clone()))?;
            registry.register(Box::new(conversation_duration.clone()))?;
            registry.register(Box::new(tool_calls_total.clone()))?;
            registry.register(Box::new(tool_execution_duration.clone()))?;
            registry.register(Box::new(tool_success_rate.clone()))?;
            registry.register(Box::new(provider_requests.clone()))?;
            registry.register(Box::new(provider_errors.clone()))?;
            registry.register(Box::new(provider_response_time.clone()))?;
            registry.register(Box::new(provider_availability.clone()))?;
            registry.register(Box::new(streaming_requests.clone()))?;
            registry.register(Box::new(streaming_chunks.clone()))?;
            registry.register(Box::new(streaming_duration.clone()))?;
            registry.register(Box::new(system_memory_usage.clone()))?;
            registry.register(Box::new(system_cpu_usage.clone()))?;
            registry.register(Box::new(database_connections_active.clone()))?;
            registry.register(Box::new(database_query_duration.clone()))?;
            registry.register(Box::new(successful_code_generations.clone()))?;
            registry.register(Box::new(patch_applications.clone()))?;
            registry.register(Box::new(file_operations.clone()))?;
            registry.register(Box::new(unique_users.clone()))?;
        }

        Ok(Self {
            registry,
            requests_total,
            request_duration,
            request_size_bytes,
            response_size_bytes,
            conversations_active,
            conversation_turns,
            conversation_duration,
            tool_calls_total,
            tool_execution_duration,
            tool_success_rate,
            provider_requests,
            provider_errors,
            provider_response_time,
            provider_availability,
            streaming_requests,
            streaming_chunks,
            streaming_duration,
            system_memory_usage,
            system_cpu_usage,
            database_connections_active,
            database_query_duration,
            successful_code_generations,
            patch_applications,
            file_operations,
            unique_users,
            enabled,
            start_time: Instant::now(),
            active_requests: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Track the start of a request
    pub async fn track_request_start(&self, request_id: String, request: &ChatCompletionRequest, provider: &str) {
        if !self.enabled {
            return;
        }

        let metrics = RequestMetrics {
            request_id: request_id.clone(),
            start_time: Instant::now(),
            model: request.model.clone(),
            provider: Some(provider.to_string()),
            conversation_id: request.previous_response_id.clone(),
            user_id: request.user.clone(),
            tool_calls_count: request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            streaming: request.stream.unwrap_or(false),
        };

        // Store active request
        {
            let mut active = self.active_requests.write().await;
            active.insert(request_id.clone(), metrics);
        }

        // Update request size metrics
        if let Ok(request_json) = serde_json::to_string(request) {
            self.request_size_bytes
                .with_label_values(&[&request.model, "json"])
                .observe(request_json.len() as f64);
        }

        debug!("Started tracking request: {}", request_id);
    }

    /// Track the completion of a request
    pub async fn track_request_completion(
        &self,
        request_id: &str,
        response: &ChatCompletionResponse,
        provider: &str,
        success: bool
    ) {
        if !self.enabled {
            return;
        }

        let duration = {
            let mut active = self.active_requests.write().await;
            if let Some(metrics) = active.remove(request_id) {
                metrics.start_time.elapsed()
            } else {
                warn!("Request metrics not found for ID: {}", request_id);
                return;
            }
        };

        // Update core request metrics
        let status = if success { "success" } else { "error" };
        self.requests_total
            .with_label_values(&[&response.model, provider, status, "codex_cli"])
            .inc();

        self.request_duration
            .with_label_values(&[&response.model, provider, "chat_completion"])
            .observe(duration.as_secs_f64());

        // Update provider metrics
        self.provider_requests
            .with_label_values(&[provider, &response.model, status])
            .inc();

        self.provider_response_time
            .with_label_values(&[provider, &response.model])
            .observe(duration.as_secs_f64());

        // Update response size metrics
        if let Ok(response_json) = serde_json::to_string(response) {
            let has_tools = response.choices.iter()
                .any(|c| c.message.tool_calls.is_some());
            
            self.response_size_bytes
                .with_label_values(&[&response.model, provider, &has_tools.to_string()])
                .observe(response_json.len() as f64);
        }

        // Update business metrics based on response content
        self.update_business_metrics(response).await;

        debug!("Completed tracking request: {} (duration: {:?})", request_id, duration);
    }

    /// Track tool execution
    pub async fn track_tool_execution(
        &self,
        tool_call: &ToolCall,
        execution_time: Duration,
        success: bool,
        result_size: usize
    ) {
        if !self.enabled {
            return;
        }

        let status = if success { "success" } else { "error" };
        
        self.tool_calls_total
            .with_label_values(&[&tool_call.function.name, "local", status])
            .inc();

        self.tool_execution_duration
            .with_label_values(&[&tool_call.function.name, "execution"])
            .observe(execution_time.as_secs_f64());

        self.tool_success_rate
            .with_label_values(&[&tool_call.function.name, &success.to_string()])
            .inc();

        // Track specific tool types
        match tool_call.function.name.as_str() {
            "apply_patch" => {
                self.patch_applications.inc();
                self.file_operations
                    .with_label_values(&["patch", "unknown", status])
                    .inc();
            }
            "read_file" => {
                self.file_operations
                    .with_label_values(&["read", "unknown", status])
                    .inc();
            }
            "write_file" => {
                self.file_operations
                    .with_label_values(&["write", "unknown", status])
                    .inc();
            }
            "search_files" => {
                self.file_operations
                    .with_label_values(&["search", "unknown", status])
                    .inc();
            }
            _ => {}
        }

        debug!("Tracked tool execution: {} (duration: {:?}, success: {})", 
               tool_call.function.name, execution_time, success);
    }

    /// Track streaming request
    pub async fn track_streaming_request(&self, chunk_count: usize, total_duration: Duration, model: &str) {
        if !self.enabled {
            return;
        }

        self.streaming_requests.inc();
        
        self.streaming_chunks
            .with_label_values(&[model, "text"])
            .observe(chunk_count as f64);

        self.streaming_duration
            .observe(total_duration.as_secs_f64());

        debug!("Tracked streaming request: {} chunks in {:?}", chunk_count, total_duration);
    }

    /// Track conversation activity
    pub async fn track_conversation_activity(&self, conversation_id: &str, turn_count: u32, active: bool) {
        if !self.enabled {
            return;
        }

        if active {
            self.conversations_active.inc();
        } else {
            self.conversations_active.dec();
        }

        let turn_bucket = match turn_count {
            1..=5 => "1-5",
            6..=10 => "6-10", 
            11..=20 => "11-20",
            _ => "20+"
        };

        self.conversation_turns
            .with_label_values(&["codex_cli", turn_bucket])
            .inc();

        debug!("Tracked conversation activity: {} (turns: {}, active: {})", 
               conversation_id, turn_count, active);
    }

    /// Track provider availability
    pub async fn track_provider_availability(&self, provider: &str, availability: f64) {
        if !self.enabled {
            return;
        }

        self.provider_availability
            .with_label_values(&[provider])
            .set(availability);

        debug!("Updated provider availability: {} = {:.2}", provider, availability);
    }

    /// Track provider error
    pub async fn track_provider_error(&self, provider: &str, error_type: &str, retryable: bool) {
        if !self.enabled {
            return;
        }

        self.provider_errors
            .with_label_values(&[provider, error_type, &retryable.to_string()])
            .inc();

        debug!("Tracked provider error: {} - {} (retryable: {})", provider, error_type, retryable);
    }

    /// Collect system metrics
    pub async fn collect_system_metrics(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Collect memory usage
        if let Ok(memory_info) = self.get_memory_usage().await {
            self.system_memory_usage.set(memory_info as f64);
        }

        // Collect CPU usage
        if let Ok(cpu_usage) = self.get_cpu_usage().await {
            self.system_cpu_usage.set(cpu_usage);
        }

        // Update uptime
        let uptime_seconds = self.start_time.elapsed().as_secs();
        
        debug!("Collected system metrics (uptime: {}s)", uptime_seconds);
        Ok(())
    }

    /// Get diagnostic information
    pub async fn get_diagnostic_info(&self) -> Result<serde_json::Value> {
        let active_count = self.active_requests.read().await.len();
        let uptime = self.start_time.elapsed();

        Ok(json!({
            "metrics_enabled": self.enabled,
            "uptime_seconds": uptime.as_secs(),
            "active_requests": active_count,
            "total_metrics_registered": self.registry.gather().len(),
            "system_info": {
                "memory_usage_bytes": self.get_memory_usage().await.unwrap_or(0),
                "cpu_usage_ratio": self.get_cpu_usage().await.unwrap_or(0.0),
            }
        }))
    }

    /// Check if metrics collection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Flush any pending metrics
    pub async fn flush(&self) -> Result<()> {
        debug!("Flushing metrics (no-op for Prometheus)");
        Ok(())
    }

    // Private helper methods

    async fn update_business_metrics(&self, response: &ChatCompletionResponse) {
        // Check for successful code generation
        let has_code = response.choices.iter().any(|choice| {
            choice.message.content.as_ref().map(|content| {
                content.contains("```") || content.contains("fn ") || content.contains("function")
            }).unwrap_or(false)
        });

        if has_code {
            self.successful_code_generations.inc();
        }
    }

    async fn get_memory_usage(&self) -> Result<u64> {
        // Platform-specific memory usage collection
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status = fs::read_to_string("/proc/self/status")?;
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let kb: u64 = line.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    return Ok(kb * 1024); // Convert KB to bytes
                }
            }
        }
        
        // Fallback for other platforms
        Ok(0)
    }

    async fn get_cpu_usage(&self) -> Result<f64> {
        // Simplified CPU usage (would need more sophisticated implementation)
        Ok(0.0)
    }
}

/// Helper functions for integration with the main application

/// Initialize metrics collection middleware
pub async fn create_metrics_middleware(
    metrics: Arc<MetricsCollector>
) -> impl axum::middleware::Layer<axum::Router> {
    axum::middleware::from_fn(move |req, next| {
        let metrics = metrics.clone();
        async move {
            let start = Instant::now();
            let response = next.run(req).await;
            let duration = start.elapsed();
            
            // Track HTTP metrics here if needed
            debug!("HTTP request processed in {:?}", duration);
            
            response
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::MessageRole;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let registry = Arc::new(Registry::new());
        let collector = MetricsCollector::new(registry, true).unwrap();
        
        assert!(collector.is_enabled());
    }

    #[tokio::test]
    async fn test_request_tracking() {
        let registry = Arc::new(Registry::new());
        let collector = MetricsCollector::new(registry, true).unwrap();
        
        let request = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage {
                role: MessageRole::User,
                content: "test".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }]
        );

        collector.track_request_start("test-123".to_string(), &request, "vertex").await;
        
        let active_count = collector.active_requests.read().await.len();
        assert_eq!(active_count, 1);
    }

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let registry = Arc::new(Registry::new());
        let collector = MetricsCollector::new(registry, true).unwrap();
        
        let result = collector.collect_system_metrics().await;
        assert!(result.is_ok());
    }
}