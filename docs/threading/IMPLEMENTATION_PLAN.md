# Threading Architecture Implementation Plan

**Project:** llm-supabase-rs  
**Version:** 3.0 - Performance Optimized  
**Date:** October 3, 2025

---

## Implementation Roadmap

### Phase 1: Foundation Threading (Weeks 1-2)

#### Week 1: Core Threading Infrastructure
- [ ] Create `ThreadingManager` with multi-runtime architecture
- [ ] Implement runtime separation (HTTP, AI, AV, DB, MCP)
- [ ] Add semaphore-based backpressure control
- [ ] Create thread pool configuration system
- [ ] Add basic performance metrics collection

#### Week 2: Connection Pooling
- [ ] Implement `HttpClientPool` with round-robin selection
- [ ] Create `DatabasePool` with read/write separation
- [ ] Add connection health monitoring
- [ ] Implement connection pool metrics
- [ ] Add pool configuration management

### Phase 2: Memory Optimization (Weeks 3-4)

#### Week 3: Memory Pool System
- [ ] Create `MemoryPoolManager` for buffer reuse
- [ ] Implement audio buffer pools for real-time processing
- [ ] Add response buffer pools for HTTP operations
- [ ] Create embedding buffer pools for vector operations
- [ ] Add memory usage monitoring and alerts

#### Week 4: Zero-Copy Optimizations
- [ ] Implement zero-copy audio processing pipeline
- [ ] Add SIMD optimizations for audio operations
- [ ] Create streaming response optimizations
- [ ] Implement buffer recycling mechanisms
- [ ] Add memory leak detection and prevention

### Phase 3: Real-Time Processing (Weeks 5-6)

#### Week 5: Audio/Video Pipeline
- [ ] Create `RealTimeProcessor` for audio streams
- [ ] Implement streaming transcription pipeline
- [ ] Add voice activity detection
- [ ] Create speaker diarization system
- [ ] Implement audio format conversion

#### Week 6: LiveKit Integration
- [ ] Create LiveKit agent connection manager
- [ ] Implement multi-room support
- [ ] Add real-time audio streaming
- [ ] Create agent state management
- [ ] Implement WebRTC optimization

### Phase 4: MCP Performance (Weeks 7-8)

#### Week 7: Microsandbox Integration
- [ ] Create `SandboxManager` with pre-warming
- [ ] Implement resource monitoring and limits
- [ ] Add sandbox instance pooling
- [ ] Create secure execution environment
- [ ] Implement sandbox lifecycle management

#### Week 8: MCP Optimization
- [ ] Add MCP request batching
- [ ] Implement schema caching
- [ ] Create streaming MCP communication
- [ ] Add MCP server health monitoring
- [ ] Implement failover and retry logic

---

## Performance Targets

### Latency Improvements
| Operation | Current | Target | Improvement |
|-----------|---------|--------|-------------|
| Chat completion (first token) | 1s | 500ms | 50% |
| Embedding generation | 500ms | 200ms | 60% |
| Tool execution | 3s | 1s | 67% |
| Agent join | 2s | 1s | 50% |
| Transcription delay | 500ms | 200ms | 60% |

### Throughput Improvements
| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Concurrent requests | 1,000 | 5,000 | 5x |
| Requests per second | 500 | 2,500 | 5x |
| Concurrent agents | 100 | 500 | 5x |
| Concurrent rooms | 50 | 200 | 4x |
| MCP servers | 50 | 200 | 4x |

---

## Dependencies to Add

```toml
[dependencies]
# Threading and async
num_cpus = "1.16"
rayon = "1.8"
crossbeam = "0.8"
tokio-util = { version = "0.7", features = ["full"] }

# Memory management
bytes = "1.5"
memmap2 = "0.9"

# Audio processing
symphonia = { version = "0.5", features = ["all"] }
rubato = "0.15"
hound = "3.5"

# SIMD optimizations
wide = "0.7"

# Performance monitoring
sysinfo = "0.30"
procfs = "0.16"

# Microsandbox integration
microsandbox = "0.1.2"

# LiveKit integration
livekit = "0.3"
livekit-api = "0.3"
webrtc = "0.7"

# Vector operations
faiss = "0.12"  # For high-performance vector search
ndarray = "0.15"

# Connection pooling
deadpool = "0.10"
deadpool-postgres = "0.12"

# Caching
moka = "0.12"  # High-performance cache
```

---

## Configuration Extensions

```yaml
# config/threading.yaml
threading:
  http_runtime:
    threads: 4
    stack_size_mb: 2
    max_concurrent: 1000
    
  ai_runtime:
    threads: 8
    stack_size_mb: 4
    max_concurrent: 100
    
  av_runtime:
    threads: 4
    stack_size_mb: 8
    max_concurrent: 50
    priority: high
    
  db_runtime:
    threads: 2
    stack_size_mb: 2
    max_concurrent: 200
    
  mcp_runtime:
    threads: 4
    stack_size_mb: 4
    max_concurrent: 100

connection_pools:
  http_clients:
    pool_size: 10
    max_idle_per_host: 5
    idle_timeout_ms: 30000
    keepalive_duration_ms: 60000
    
  database:
    max_connections: 50
    min_connections: 5
    acquire_timeout_ms: 5000
    idle_timeout_ms: 600000
    max_lifetime_ms: 1800000
    read_replica_url: ${DATABASE_READ_URL}
    read_max_connections: 20

memory_pools:
  audio_buffers:
    count: 100
    size: 48000  # 1 second at 48kHz
    sample_rate: 48000
    
  response_buffers:
    count: 200
    size: 65536  # 64KB
    
  embedding_buffers:
    count: 50
    dimensions: 1536

microsandbox:
  max_concurrent: 50
  timeout_ms: 30000
  memory_limit_mb: 512
  cpu_limit_percent: 50.0
  network_enabled: true
  prewarmed_instances: 10
  cleanup_interval_ms: 30000
```

---

## Monitoring and Alerting

### Key Metrics to Track

```rust
// Performance metrics structure
pub struct ThreadingMetrics {
    // Runtime utilization
    pub http_runtime_utilization: Gauge,
    pub ai_runtime_utilization: Gauge,
    pub av_runtime_utilization: Gauge,
    
    // Queue depths
    pub http_queue_depth: IntGauge,
    pub ai_queue_depth: IntGauge,
    pub av_queue_depth: IntGauge,
    
    // Connection pools
    pub http_pool_active: IntGauge,
    pub http_pool_idle: IntGauge,
    pub db_pool_active: IntGauge,
    pub db_pool_wait_time: Histogram,
    
    // Memory pools
    pub audio_buffer_usage: Gauge,
    pub response_buffer_usage: Gauge,
    pub memory_pool_hits: IntCounter,
    pub memory_pool_misses: IntCounter,
    
    // Microsandbox
    pub sandbox_instances_active: IntGauge,
    pub sandbox_startup_time: Histogram,
    pub sandbox_execution_time: Histogram,
    pub sandbox_memory_usage: Gauge,
    
    // Real-time processing
    pub audio_processing_latency: Histogram,
    pub transcription_latency: Histogram,
    pub embedding_latency: Histogram,
    pub end_to_end_latency: Histogram,
}
```

### Alert Thresholds

```yaml
alerts:
  thread_pool_utilization:
    warning: 70%
    critical: 85%
    
  queue_depth:
    warning: 100
    critical: 500
    
  connection_pool_exhaustion:
    warning: 80%
    critical: 95%
    
  memory_pool_exhaustion:
    warning: 80%
    critical: 95%
    
  latency_degradation:
    p95_warning: 2000ms
    p95_critical: 5000ms
    p99_warning: 5000ms
    p99_critical: 10000ms
```

---

## Load Testing Framework

### Test Scenarios

```rust
// tests/performance/load_test.rs

use std::time::Duration;
use tokio::time::Instant;

pub struct LoadTestSuite {
    base_url: String,
    client: reqwest::Client,
}

impl LoadTestSuite {
    /// Test concurrent chat completions
    pub async fn test_concurrent_chat(&self, concurrent_users: usize) -> TestResults {
        let start = Instant::now();
        let mut handles = Vec::new();
        
        for i in 0..concurrent_users {
            let client = self.client.clone();
            let url = format!("{}/v1/chat/completions", self.base_url);
            
            let handle = tokio::spawn(async move {
                let request = serde_json::json!({
                    "model": "claude-sonnet-4-5@20250929",
                    "messages": [
                        {"role": "user", "content": format!("Test message {}", i)}
                    ],
                    "max_tokens": 100
                });
                
                let start = Instant::now();
                let response = client.post(&url)
                    .json(&request)
                    .send()
                    .await?;
                    
                let latency = start.elapsed();
                let status = response.status();
                
                Ok::<_, anyhow::Error>((status, latency))
            });
            
            handles.push(handle);
        }
        
        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
            
        TestResults::analyze(results, start.elapsed())
    }
    
    /// Test real-time audio processing
    pub async fn test_realtime_audio(&self, concurrent_streams: usize) -> TestResults {
        // Simulate multiple audio streams
        let mut handles = Vec::new();
        
        for i in 0..concurrent_streams {
            let client = self.client.clone();
            let url = format!("{}/v1/audio/process", self.base_url);
            
            let handle = tokio::spawn(async move {
                // Simulate audio stream
                let audio_data = generate_test_audio(Duration::from_secs(10));
                
                let start = Instant::now();
                let response = client.post(&url)
                    .header("Content-Type", "audio/wav")
                    .body(audio_data)
                    .send()
                    .await?;
                    
                let latency = start.elapsed();
                Ok::<_, anyhow::Error>((response.status(), latency))
            });
            
            handles.push(handle);
        }
        
        let results: Vec<_> = futures::future::join_all(handles).await;
        TestResults::from_results(results)
    }
    
    /// Test MCP server performance
    pub async fn test_mcp_performance(&self, concurrent_tools: usize) -> TestResults {
        let mut handles = Vec::new();
        
        for i in 0..concurrent_tools {
            let client = self.client.clone();
            let url = format!("{}/mcp/v1/invoke", self.base_url);
            
            let handle = tokio::spawn(async move {
                let request = serde_json::json!({
                    "method": "tools/call",
                    "params": {
                        "name": "web_search",
                        "arguments": {
                            "query": format!("test query {}", i)
                        }
                    }
                });
                
                let start = Instant::now();
                let response = client.post(&url)
                    .json(&request)
                    .send()
                    .await?;
                    
                Ok::<_, anyhow::Error>((response.status(), start.elapsed()))
            });
            
            handles.push(handle);
        }
        
        let results: Vec<_> = futures::future::join_all(handles).await;
        TestResults::from_results(results)
    }
}

pub struct TestResults {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_latency: Duration,
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub requests_per_second: f64,
    pub total_duration: Duration,
}

impl TestResults {
    pub fn analyze(results: Vec<(reqwest::StatusCode, Duration)>, total_duration: Duration) -> Self {
        let successful = results.iter().filter(|(status, _)| status.is_success()).count();
        let failed = results.len() - successful;
        
        let mut latencies: Vec<Duration> = results.iter().map(|(_, latency)| *latency).collect();
        latencies.sort();
        
        let average_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let p50_latency = latencies[latencies.len() / 2];
        let p95_latency = latencies[(latencies.len() as f64 * 0.95) as usize];
        let p99_latency = latencies[(latencies.len() as f64 * 0.99) as usize];
        
        let requests_per_second = results.len() as f64 / total_duration.as_secs_f64();
        
        Self {
            total_requests: results.len(),
            successful_requests: successful,
            failed_requests: failed,
            average_latency,
            p50_latency,
            p95_latency,
            p99_latency,
            requests_per_second,
            total_duration,
        }
    }
}
```

---

## Critical Implementation Details

### 1. Runtime Isolation Strategy

The key insight from the research is that different workload types should be isolated to prevent interference:

- **HTTP Runtime**: Fast request/response handling
- **AI Runtime**: Provider API calls and response processing
- **Audio/Video Runtime**: Real-time processing with strict latency requirements
- **Database Runtime**: Connection pooling and query optimization
- **MCP Runtime**: Sandboxed execution with resource monitoring

### 2. Backpressure Management

Use semaphores to prevent resource exhaustion:
- Limit concurrent operations per runtime
- Queue requests when capacity is reached
- Provide circuit breaker functionality
- Monitor queue depths and alert on buildup

### 3. Memory Pool Strategy

Pre-allocate buffers for hot paths:
- Audio buffers for real-time processing
- Response buffers for HTTP operations
- Embedding vectors for AI operations
- Reduce GC pressure and allocation overhead

### 4. Connection Optimization

- HTTP/2 multiplexing for provider connections
- Keep-alive connections with proper timeout
- Connection health monitoring
- Automatic connection recycling

---

## Monitoring Dashboard

### Key Performance Indicators

```rust
// Dashboard metrics structure
pub struct PerformanceDashboard {
    // Real-time metrics
    pub current_rps: f64,
    pub active_connections: usize,
    pub queue_depths: HashMap<String, usize>,
    pub memory_usage: MemoryUsage,
    
    // Latency percentiles
    pub latency_p50: Duration,
    pub latency_p95: Duration,
    pub latency_p99: Duration,
    
    // Resource utilization
    pub cpu_usage: f64,
    pub memory_usage_percent: f64,
    pub thread_pool_utilization: HashMap<String, f64>,
    
    // Error rates
    pub error_rate: f64,
    pub timeout_rate: f64,
    pub sandbox_failure_rate: f64,
}
```

### Grafana Dashboard Configuration

```json
{
  "dashboard": {
    "title": "LLM Proxy Performance",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "legendFormat": "RPS"
          }
        ]
      },
      {
        "title": "Thread Pool Utilization",
        "type": "graph",
        "targets": [
          {
            "expr": "thread_pool_utilization",
            "legendFormat": "{{pool_name}}"
          }
        ]
      },
      {
        "title": "Latency Percentiles",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, http_request_duration_seconds_bucket)",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, http_request_duration_seconds_bucket)",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, http_request_duration_seconds_bucket)",
            "legendFormat": "P99"
          }
        ]
      }
    ]
  }
}
```

---

## Testing Strategy

### Performance Test Suite

```bash
# Load testing commands
cargo test --release --test performance_tests

# Specific test scenarios
cargo test --release --test load_test -- --test-threads=1 concurrent_chat_1000
cargo test --release --test load_test -- --test-threads=1 realtime_audio_100
cargo test --release --test load_test -- --test-threads=1 mcp_tools_200

# Memory profiling
cargo test --release --test memory_test -- --test-threads=1 memory_usage
```

### Continuous Performance Monitoring

```rust
// Automated performance regression detection
pub struct PerformanceRegression {
    baseline_metrics: PerformanceBaseline,
    current_metrics: PerformanceMetrics,
    thresholds: RegressionThresholds,
}

impl PerformanceRegression {
    pub fn detect_regressions(&self) -> Vec<RegressionAlert> {
        let mut alerts = Vec::new();
        
        // Check latency regressions
        if self.current_metrics.p95_latency > 
           self.baseline_metrics.p95_latency * self.thresholds.latency_regression {
            alerts.push(RegressionAlert::LatencyRegression {
                current: self.current_metrics.p95_latency,
                baseline: self.baseline_metrics.p95_latency,
                threshold: self.thresholds.latency_regression,
            });
        }
        
        // Check throughput regressions
        if self.current_metrics.requests_per_second < 
           self.baseline_metrics.requests_per_second * self.thresholds.throughput_regression {
            alerts.push(RegressionAlert::ThroughputRegression {
                current: self.current_metrics.requests_per_second,
                baseline: self.baseline_metrics.requests_per_second,
                threshold: self.thresholds.throughput_regression,
            });
        }
        
        alerts
    }
}
```

---

## Migration Strategy

### Phase 1: Non-Breaking Changes
1. Add threading infrastructure alongside existing code
2. Implement connection pooling with fallback to existing clients
3. Add memory pools with fallback to standard allocation
4. Implement metrics collection without changing APIs

### Phase 2: Runtime Migration
1. Migrate HTTP handlers to use dedicated runtime
2. Move AI operations to AI runtime
3. Implement database operation migration
4. Add MCP runtime with sandbox support

### Phase 3: Optimization
1. Enable zero-copy optimizations
2. Implement SIMD audio processing
3. Add advanced caching strategies
4. Optimize memory allocation patterns

### Phase 4: Real-Time Features
1. Add LiveKit integration with dedicated runtime
2. Implement real-time transcription pipeline
3. Add agent orchestration capabilities
4. Enable full real-time AI video platform features

---

## Success Criteria

### Performance Benchmarks
- [ ] 5,000+ concurrent requests sustained
- [ ] Sub-second response times for 95% of requests
- [ ] Zero memory leaks under sustained load
- [ ] CPU utilization < 80% under normal load
- [ ] Successful handling of 200+ concurrent LiveKit rooms

### Reliability Benchmarks
- [ ] 99.9% uptime under load
- [ ] Graceful degradation under resource pressure
- [ ] Automatic recovery from sandbox failures
- [ ] Zero data loss during high-load scenarios

### Scalability Benchmarks
- [ ] Linear scaling with additional CPU cores
- [ ] Horizontal scaling across multiple instances
- [ ] Efficient resource utilization across all thread pools
- [ ] Minimal cross-runtime communication overhead

---

This implementation plan provides a comprehensive roadmap for transforming the current single-threaded architecture into a high-performance, multi-runtime system capable of supporting the demanding requirements of the Real-Time AI Video Platform.