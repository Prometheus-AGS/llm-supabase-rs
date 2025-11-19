//! Langfuse LLM Observability Integration
//! 
//! This module provides comprehensive LLM monitoring and observability using Langfuse,
//! tracking all Codex CLI interactions, performance metrics, and business insights.

use anyhow::{Result, Context};
use langfuse::{Langfuse, CreateTraceRequest, CreateGenerationRequest, CreateSpanRequest};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
    common::{ChatMessage, ToolCall},
};

/// Langfuse configuration for LLM observability
#[derive(Debug, Clone)]
pub struct LangfuseConfig {
    /// Langfuse API public key
    pub public_key: String,
    /// Langfuse API secret key  
    pub secret_key: String,
    /// Langfuse API base URL
    pub base_url: String,
    /// Enable detailed tracing
    pub detailed_tracing: bool,
    /// Enable performance monitoring
    pub performance_monitoring: bool,
    /// Enable business metrics
    pub business_metrics: bool,
    /// Batch size for sending traces
    pub batch_size: usize,
    /// Flush interval in seconds
    pub flush_interval_seconds: u64,
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self {
            public_key: std::env::var("LANGFUSE_PUBLIC_KEY")
                .unwrap_or_else(|_| "pk_lf_test_public_key".to_string()),
            secret_key: std::env::var("LANGFUSE_SECRET_KEY")
                .unwrap_or_else(|_| "sk_lf_test_secret_key".to_string()),
            base_url: std::env::var("LANGFUSE_BASE_URL")
                .unwrap_or_else(|_| Self::detect_langfuse_instance()),
            detailed_tracing: true,
            performance_monitoring: true,
            business_metrics: true,
            batch_size: 100,
            flush_interval_seconds: 30,
        }
    }
}

impl LangfuseConfig {
    /// Detect the appropriate Langfuse instance to use
    fn detect_langfuse_instance() -> String {
        // Check for local development environment
        if std::env::var("TEST_MODE").unwrap_or_default() == "docker" {
            return "http://langfuse-server:3000".to_string();
        }
        
        // Check for local Langfuse instance
        if std::env::var("LANGFUSE_LOCAL_INSTANCE").unwrap_or_default() == "true" {
            return "http://localhost:3000".to_string();
        }
        
        // Default to cloud instance
        "https://cloud.langfuse.com".to_string()
    }

    /// Create configuration for private Langfuse instance
    pub fn for_private_instance(base_url: &str, public_key: &str, secret_key: &str) -> Self {
        Self {
            public_key: public_key.to_string(),
            secret_key: secret_key.to_string(),
            base_url: base_url.to_string(),
            detailed_tracing: true,
            performance_monitoring: true,
            business_metrics: true,
            batch_size: 50, // Smaller batches for private instances
            flush_interval_seconds: 15, // More frequent flushing
        }
    }

    /// Create configuration for testing environment
    pub fn for_testing() -> Self {
        Self {
            public_key: "pk_lf_test_public_key".to_string(),
            secret_key: "sk_lf_test_secret_key".to_string(),
            base_url: "http://localhost:3000".to_string(),
            detailed_tracing: true,
            performance_monitoring: true,
            business_metrics: true,
            batch_size: 10, // Small batches for testing
            flush_interval_seconds: 5, // Fast flushing for testing
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.public_key.is_empty() {
            return Err(anyhow::anyhow!("Langfuse public key is required"));
        }
        
        if self.secret_key.is_empty() {
            return Err(anyhow::anyhow!("Langfuse secret key is required"));
        }
        
        if self.base_url.is_empty() {
            return Err(anyhow::anyhow!("Langfuse base URL is required"));
        }

        // Validate URL format
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(anyhow::anyhow!("Langfuse base URL must start with http:// or https://"));
        }

        Ok(())
    }
}

/// Comprehensive LLM observability system using Langfuse
pub struct LangfuseObservability {
    client: Arc<Langfuse>,
    config: LangfuseConfig,
    active_traces: Arc<RwLock<HashMap<String, TraceContext>>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    business_metrics: Arc<RwLock<BusinessMetrics>>,
}

/// Context for tracking individual request traces
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub start_time: Instant,
    pub request: ChatCompletionRequest,
    pub spans: Vec<SpanContext>,
}

/// Context for tracking spans within a trace
#[derive(Debug, Clone)]
pub struct SpanContext {
    pub span_id: String,
    pub name: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub metadata: HashMap<String, Value>,
}

/// Performance metrics for LLM operations
#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_response_time_ms: u64,
    pub total_tokens_processed: u64,
    pub total_tool_calls: u64,
    pub provider_performance: HashMap<String, ProviderMetrics>,
}

/// Provider-specific performance metrics
#[derive(Debug, Default)]
pub struct ProviderMetrics {
    pub requests: u64,
    pub total_response_time_ms: u64,
    pub errors: u64,
    pub average_tokens_per_request: f64,
}

/// Business metrics for Codex CLI usage
#[derive(Debug, Default)]
pub struct BusinessMetrics {
    pub codex_sessions: u64,
    pub successful_code_generations: u64,
    pub tool_executions: HashMap<String, u64>,
    pub conversation_turns: u64,
    pub unique_users: std::collections::HashSet<String>,
    pub file_operations: u64,
    pub patch_applications: u64,
}

impl LangfuseObservability {
    /// Initialize Langfuse observability system
    pub async fn new(config: LangfuseConfig) -> Result<Self> {
        info!("Initializing Langfuse observability system");
        
        // Validate configuration
        if config.public_key.is_empty() || config.secret_key.is_empty() {
            return Err(anyhow::anyhow!(
                "Langfuse API keys not configured. Set LANGFUSE_PUBLIC_KEY and LANGFUSE_SECRET_KEY environment variables."
            ));
        }

        // Initialize Langfuse client
        let client = Arc::new(Langfuse::new(
            config.public_key.clone(),
            config.secret_key.clone(),
            config.base_url.clone(),
        ));

        // Test connection
        match client.health_check().await {
            Ok(_) => info!("Langfuse connection established successfully"),
            Err(e) => {
                warn!("Langfuse health check failed: {}. Monitoring will continue with limited functionality.", e);
            }
        }

        let observability = Self {
            client,
            config,
            active_traces: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            business_metrics: Arc::new(RwLock::new(BusinessMetrics::default())),
        };

        // Start background tasks
        observability.start_background_tasks().await;

        info!("Langfuse observability system initialized");
        Ok(observability)
    }

    /// Start a new trace for a Codex CLI request
    pub async fn start_trace(&self, request: &ChatCompletionRequest) -> Result<String> {
        let trace_id = Uuid::new_v4().to_string();
        
        // Create trace context
        let trace_context = TraceContext {
            trace_id: trace_id.clone(),
            session_id: self.extract_session_id(request),
            user_id: request.user.clone(),
            conversation_id: request.previous_response_id.clone(),
            start_time: Instant::now(),
            request: request.clone(),
            spans: Vec::new(),
        };

        // Store active trace
        {
            let mut traces = self.active_traces.write().await;
            traces.insert(trace_id.clone(), trace_context);
        }

        // Create Langfuse trace
        let trace_request = CreateTraceRequest {
            id: Some(trace_id.clone()),
            name: Some("codex_cli_request".to_string()),
            user_id: request.user.clone(),
            session_id: self.extract_session_id(request),
            input: Some(self.serialize_request_input(request)),
            metadata: Some(self.create_request_metadata(request)),
            tags: Some(vec!["codex-cli".to_string(), "chat-completion".to_string()]),
            ..Default::default()
        };

        if let Err(e) = self.client.create_trace(trace_request).await {
            warn!("Failed to create Langfuse trace: {}", e);
        }

        debug!("Started trace: {}", trace_id);
        Ok(trace_id)
    }

    /// Complete a trace with the response
    pub async fn complete_trace(
        &self, 
        trace_id: &str, 
        response: &ChatCompletionResponse,
        provider: &str
    ) -> Result<()> {
        let (trace_context, duration) = {
            let mut traces = self.active_traces.write().await;
            match traces.remove(trace_id) {
                Some(context) => {
                    let duration = context.start_time.elapsed();
                    (context, duration)
                }
                None => {
                    warn!("Trace not found: {}", trace_id);
                    return Ok(());
                }
            }
        };

        // Update Langfuse trace with completion
        if let Err(e) = self.client.update_trace(
            trace_id,
            Some(self.serialize_response_output(response)),
            None,
            Some(duration.as_millis() as u64),
            Some(json!({
                "provider": provider,
                "model": response.model,
                "usage": response.usage,
                "finish_reason": response.choices.first().and_then(|c| c.finish_reason.as_ref()),
            }))
        ).await {
            warn!("Failed to update Langfuse trace: {}", e);
        }

        // Update metrics
        self.update_performance_metrics(&trace_context, response, provider, duration).await;
        self.update_business_metrics(&trace_context, response).await;

        debug!("Completed trace: {} (duration: {:?})", trace_id, duration);
        Ok(())
    }

    /// Start a span within a trace
    pub async fn start_span(&self, trace_id: &str, name: &str, metadata: HashMap<String, Value>) -> Result<String> {
        let span_id = Uuid::new_v4().to_string();
        
        // Add span to trace context
        {
            let mut traces = self.active_traces.write().await;
            if let Some(trace_context) = traces.get_mut(trace_id) {
                trace_context.spans.push(SpanContext {
                    span_id: span_id.clone(),
                    name: name.to_string(),
                    start_time: Instant::now(),
                    end_time: None,
                    metadata: metadata.clone(),
                });
            }
        }

        // Create Langfuse span
        let span_request = CreateSpanRequest {
            id: Some(span_id.clone()),
            trace_id: trace_id.to_string(),
            name: name.to_string(),
            input: Some(json!(metadata)),
            start_time: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
            ..Default::default()
        };

        if let Err(e) = self.client.create_span(span_request).await {
            warn!("Failed to create Langfuse span: {}", e);
        }

        debug!("Started span: {} in trace: {}", span_id, trace_id);
        Ok(span_id)
    }

    /// Complete a span
    pub async fn complete_span(&self, trace_id: &str, span_id: &str, output: Option<Value>) -> Result<()> {
        // Update span in trace context
        {
            let mut traces = self.active_traces.write().await;
            if let Some(trace_context) = traces.get_mut(trace_id) {
                for span in &mut trace_context.spans {
                    if span.span_id == span_id {
                        span.end_time = Some(Instant::now());
                        break;
                    }
                }
            }
        }

        // Update Langfuse span
        if let Err(e) = self.client.update_span(
            span_id,
            output,
            Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
            None
        ).await {
            warn!("Failed to update Langfuse span: {}", e);
        }

        debug!("Completed span: {} in trace: {}", span_id, trace_id);
        Ok(())
    }

    /// Track LLM generation with detailed metrics
    pub async fn track_generation(
        &self,
        trace_id: &str,
        model: &str,
        input: &[ChatMessage],
        output: &str,
        usage: Option<&crate::models::response::ChatCompletionUsage>,
        provider: &str
    ) -> Result<()> {
        let generation_id = Uuid::new_v4().to_string();

        let generation_request = CreateGenerationRequest {
            id: Some(generation_id.clone()),
            trace_id: Some(trace_id.to_string()),
            name: Some("chat_completion".to_string()),
            model: Some(model.to_string()),
            input: Some(json!(input)),
            output: Some(json!(output)),
            usage: usage.map(|u| json!({
                "promptTokens": u.prompt_tokens,
                "completionTokens": u.completion_tokens,
                "totalTokens": u.total_tokens
            })),
            metadata: Some(json!({
                "provider": provider,
                "streaming": false,
            })),
            ..Default::default()
        };

        if let Err(e) = self.client.create_generation(generation_request).await {
            warn!("Failed to create Langfuse generation: {}", e);
        }

        debug!("Tracked generation: {} in trace: {}", generation_id, trace_id);
        Ok(())
    }

    /// Track tool execution
    pub async fn track_tool_execution(
        &self,
        trace_id: &str,
        tool_call: &ToolCall,
        result: &str,
        success: bool,
        execution_time: Duration
    ) -> Result<()> {
        let metadata = json!({
            "tool_name": tool_call.function.name,
            "tool_arguments": tool_call.function.arguments,
            "execution_time_ms": execution_time.as_millis(),
            "success": success,
            "result_length": result.len(),
        });

        self.start_span(trace_id, &format!("tool_{}", tool_call.function.name), 
                       metadata.as_object().unwrap().clone().into_iter()
                       .map(|(k, v)| (k, v)).collect()).await?;

        // Update business metrics for tool usage
        {
            let mut metrics = self.business_metrics.write().await;
            *metrics.tool_executions.entry(tool_call.function.name.clone()).or_insert(0) += 1;
            metrics.file_operations += 1;
            
            if tool_call.function.name == "apply_patch" {
                metrics.patch_applications += 1;
            }
        }

        Ok(())
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance_metrics.read().await.clone()
    }

    /// Get current business metrics  
    pub async fn get_business_metrics(&self) -> BusinessMetrics {
        self.business_metrics.read().await.clone()
    }

    /// Create dashboard data for monitoring
    pub async fn create_dashboard_data(&self) -> Result<Value> {
        let perf_metrics = self.get_performance_metrics().await;
        let business_metrics = self.get_business_metrics().await;

        let dashboard = json!({
            "performance": {
                "total_requests": perf_metrics.total_requests,
                "success_rate": if perf_metrics.total_requests > 0 {
                    perf_metrics.successful_requests as f64 / perf_metrics.total_requests as f64
                } else { 0.0 },
                "average_response_time_ms": if perf_metrics.successful_requests > 0 {
                    perf_metrics.total_response_time_ms as f64 / perf_metrics.successful_requests as f64
                } else { 0.0 },
                "total_tokens": perf_metrics.total_tokens_processed,
                "total_tool_calls": perf_metrics.total_tool_calls,
                "provider_performance": perf_metrics.provider_performance,
            },
            "business": {
                "codex_sessions": business_metrics.codex_sessions,
                "successful_generations": business_metrics.successful_code_generations,
                "tool_usage": business_metrics.tool_executions,
                "conversation_turns": business_metrics.conversation_turns,
                "unique_users": business_metrics.unique_users.len(),
                "file_operations": business_metrics.file_operations,
                "patch_applications": business_metrics.patch_applications,
            },
            "timestamp": chrono::Utc::now(),
        });

        Ok(dashboard)
    }

    /// Flush pending traces to Langfuse
    pub async fn flush(&self) -> Result<()> {
        if let Err(e) = self.client.flush().await {
            warn!("Failed to flush Langfuse data: {}", e);
        }
        Ok(())
    }

    /// Start background tasks for monitoring
    async fn start_background_tasks(&self) {
        // Periodic flush task
        let client = self.client.clone();
        let flush_interval = Duration::from_secs(self.config.flush_interval_seconds);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            loop {
                interval.tick().await;
                if let Err(e) = client.flush().await {
                    warn!("Background flush failed: {}", e);
                }
            }
        });

        info!("Started Langfuse background tasks");
    }

    // Helper methods

    fn extract_session_id(&self, request: &ChatCompletionRequest) -> Option<String> {
        request.metadata.as_ref()
            .and_then(|m| m.get("session_id"))
            .map(|s| s.to_string())
    }

    fn serialize_request_input(&self, request: &ChatCompletionRequest) -> Value {
        json!({
            "messages": request.messages,
            "model": request.model,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "tools": request.tools,
        })
    }

    fn serialize_response_output(&self, response: &ChatCompletionResponse) -> Value {
        json!({
            "choices": response.choices,
            "usage": response.usage,
            "model": response.model,
            "id": response.id,
        })
    }

    fn create_request_metadata(&self, request: &ChatCompletionRequest) -> Value {
        json!({
            "model": request.model,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": request.stream,
            "tools_count": request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            "messages_count": request.messages.len(),
            "has_conversation_id": request.previous_response_id.is_some(),
        })
    }

    async fn update_performance_metrics(
        &self,
        trace_context: &TraceContext,
        response: &ChatCompletionResponse,
        provider: &str,
        duration: Duration
    ) {
        let mut metrics = self.performance_metrics.write().await;
        
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_response_time_ms += duration.as_millis() as u64;
        
        if let Some(usage) = &response.usage {
            metrics.total_tokens_processed += usage.total_tokens as u64;
        }

        // Count tool calls
        for choice in &response.choices {
            if let Some(tool_calls) = &choice.message.tool_calls {
                metrics.total_tool_calls += tool_calls.len() as u64;
            }
        }

        // Update provider metrics
        let provider_metrics = metrics.provider_performance.entry(provider.to_string()).or_default();
        provider_metrics.requests += 1;
        provider_metrics.total_response_time_ms += duration.as_millis() as u64;
        
        if let Some(usage) = &response.usage {
            let current_avg = provider_metrics.average_tokens_per_request;
            let current_requests = provider_metrics.requests as f64;
            provider_metrics.average_tokens_per_request = 
                (current_avg * (current_requests - 1.0) + usage.total_tokens as f64) / current_requests;
        }
    }

    async fn update_business_metrics(&self, trace_context: &TraceContext, response: &ChatCompletionResponse) {
        let mut metrics = self.business_metrics.write().await;
        
        metrics.codex_sessions += 1;
        metrics.conversation_turns += 1;
        
        if let Some(user_id) = &trace_context.user_id {
            metrics.unique_users.insert(user_id.clone());
        }

        // Check for successful code generation
        if response.choices.iter().any(|c| {
            c.message.content.as_ref().map(|content| 
                content.contains("```") || content.len() > 100
            ).unwrap_or(false)
        }) {
            metrics.successful_code_generations += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::MessageRole;

    #[tokio::test]
    async fn test_langfuse_config_default() {
        let config = LangfuseConfig::default();
        assert!(!config.base_url.is_empty());
        assert!(config.detailed_tracing);
    }

    #[tokio::test] 
    async fn test_trace_context_creation() {
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

        let trace_context = TraceContext {
            trace_id: "test-trace".to_string(),
            session_id: None,
            user_id: None,
            conversation_id: None,
            start_time: Instant::now(),
            request,
            spans: Vec::new(),
        };

        assert_eq!(trace_context.trace_id, "test-trace");
        assert!(trace_context.spans.is_empty());
    }
}