# Performance Benchmarks and Optimization Guide

**Project:** llm-supabase-rs  
**Version:** 3.0 - Performance Optimized  
**Date:** October 3, 2025

---

## Benchmark Methodology

### Test Environment Specifications

```yaml
hardware:
  cpu: "Intel Xeon or AMD EPYC (16+ cores)"
  memory: "64GB DDR4"
  storage: "NVMe SSD (1TB+)"
  network: "10Gbps Ethernet"
  gpu: "NVIDIA A100 or H100 (optional for local models)"

software:
  os: "Ubuntu 22.04 LTS"
  rust: "1.75+"
  docker: "24.0+"
  kubernetes: "1.28+"
```

### Baseline Performance Targets

| Metric | Current | Target | Stretch Goal |
|--------|---------|--------|--------------|
| **Latency (P95)** |
| Chat completion (first token) | 1000ms | 500ms | 200ms |
| Embedding generation | 500ms | 200ms | 100ms |
| Tool execution (MCP) | 3000ms | 1000ms | 500ms |
| Agent join (LiveKit) | 2000ms | 1000ms | 500ms |
| **Throughput** |
| Requests per second | 500 | 2500 | 5000 |
| Concurrent requests | 1000 | 5000 | 10000 |
| Concurrent agents | 100 | 500 | 1000 |
| **Resource Efficiency** |
| Memory usage | 4GB | 8GB | 16GB |
| CPU utilization | 60% | 75% | 85% |
| Connection pool efficiency | 70% | 90% | 95% |

---

## Optimization Techniques

### 1. CPU-Bound Optimizations

```rust
// SIMD optimizations for audio processing
#[cfg(target_arch = "x86_64")]
mod simd_audio {
    use std::arch::x86_64::*;
    
    pub unsafe fn process_audio_avx2(input: &[f32], output: &mut [f32]) {
        let chunks = input.chunks_exact(8);
        let output_chunks = output.chunks_exact_mut(8);
        
        for (input_chunk, output_chunk) in chunks.zip(output_chunks) {
            let input_vec = _mm256_loadu_ps(input_chunk.as_ptr());
            
            // Apply noise reduction using SIMD
            let noise_threshold = _mm256_set1_ps(0.01);
            let mask = _mm256_cmp_ps(input_vec, noise_threshold, _CMP_GT_OQ);
            let filtered = _mm256_and_ps(input_vec, mask);
            
            _mm256_storeu_ps(output_chunk.as_mut_ptr(), filtered);
        }
    }
}

// Rayon for CPU-intensive parallel processing
use rayon::prelude::*;

pub async fn batch_process_embeddings(
    texts: Vec<String>,
    embedding_engine: &dyn EmbeddingEngine,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let batch_size = 32; // Optimal batch size for most embedding models
    
    let results: Vec<_> = texts
        .par_chunks(batch_size)
        .map(|chunk| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    embedding_engine.generate_batch(chunk.to_vec()).await
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(results.into_iter().flatten().collect())
}
```

### 2. I/O Optimizations

```rust
// Connection pooling with circuit breaker
pub struct OptimizedHttpClient {
    pools: DashMap<String, Arc<ConnectionPool>>,
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
}

impl OptimizedHttpClient {
    pub async fn execute_request(
        &self,
        provider: &str,
        request: ProviderRequest,
    ) -> anyhow::Result<ProviderResponse> {
        let pool = self.get_or_create_pool(provider).await;
        let circuit_breaker = self.get_or_create_circuit_breaker(provider).await;
        
        // Check circuit breaker
        circuit_breaker.call(async {
            let client = pool.get_client().await?;
            
            let start = std::time::Instant::now();
            let response = client.execute(request).await?;
            let duration = start.elapsed();
            
            // Update metrics
            self.update_metrics(provider, duration, true);
            
            Ok(response)
        }).await
    }
}

// Database query optimization
pub struct OptimizedDatabase {
    write_pool: sqlx::PgPool,
    read_pool: sqlx::PgPool,
    prepared_statements: DashMap<String, sqlx::postgres::PgStatement>,
}

impl OptimizedDatabase {
    pub async fn execute_query<T>(
        &self,
        query: &str,
        params: &[&dyn sqlx::Encode<sqlx::Postgres>],
        read_only: bool,
    ) -> anyhow::Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
    {
        let pool = if read_only { &self.read_pool } else { &self.write_pool };
        
        // Use prepared statement if available
        if let Some(stmt) = self.prepared_statements.get(query) {
            let row = stmt.query_with(params).fetch_one(pool).await?;
            Ok(T::from_row(&row)?)
        } else {
            // Prepare and cache statement
            let stmt = pool.prepare(query).await?;
            self.prepared_statements.insert(query.to_string(), stmt.clone());
            
            let row = stmt.query_with(params).fetch_one(pool).await?;
            Ok(T::from_row(&row)?)
        }
    }
}
```

### 3. Memory Optimizations

```rust
// Zero-copy streaming response
pub struct ZeroCopyResponse {
    chunks: Vec<Bytes>,
    total_size: usize,
}

impl ZeroCopyResponse {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            total_size: 0,
        }
    }
    
    pub fn add_chunk(&mut self, chunk: Bytes) {
        self.total_size += chunk.len();
        self.chunks.push(chunk);
    }
    
    pub fn into_stream(self) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
        futures::stream::iter(
            self.chunks.into_iter().map(Ok)
        )
    }
}

// Memory-mapped file handling for large models
pub struct MemoryMappedModel {
    mmap: memmap2::Mmap,
    metadata: ModelMetadata,
}

impl MemoryMappedModel {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        
        // Parse model metadata from header
        let metadata = ModelMetadata::from_bytes(&mmap[0..1024])?;
        
        Ok(Self { mmap, metadata })
    }
    
    pub fn get_layer_weights(&self, layer_index: usize) -> &[f32] {
        let offset = self.metadata.layer_offsets[layer_index];
        let size = self.metadata.layer_sizes[layer_index];
        
        // Zero-copy slice into memory-mapped data
        unsafe {
            std::slice::from_raw_parts(
                self.mmap.as_ptr().add(offset) as *const f32,
                size / std::mem::size_of::<f32>(),
            )
        }
    }
}
```

---

## Profiling and Debugging

### Performance Profiling Tools

```bash
# CPU profiling with perf
perf record --call-graph dwarf cargo run --release
perf report

# Memory profiling with valgrind
valgrind --tool=massif --stacks=yes cargo run --release

# Rust-specific profiling
cargo install flamegraph
cargo flamegraph --bin llm-supabase-rs

# Tokio console for async debugging
cargo install tokio-console
RUSTFLAGS="--cfg tokio_unstable" cargo run --features tokio-console
```

### Custom Profiling Integration

```rust
// src/infrastructure/profiling/mod.rs

pub struct PerformanceProfiler {
    cpu_profiler: CpuProfiler,
    memory_profiler: MemoryProfiler,
    async_profiler: AsyncProfiler,
}

impl PerformanceProfiler {
    pub fn start_profiling(&self, session_id: &str) -> ProfilingSession {
        ProfilingSession {
            id: session_id.to_string(),
            start_time: std::time::Instant::now(),
            cpu_samples: Vec::new(),
            memory_samples: Vec::new(),
            async_samples: Vec::new(),
        }
    }
    
    pub async fn profile_operation<F, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> (T, OperationProfile)
    where
        F: Future<Output = T>,
    {
        let start_cpu = self.cpu_profiler.current_usage();
        let start_memory = self.memory_profiler.current_usage();
        let start_time = std::time::Instant::now();
        
        let result = operation.await;
        
        let duration = start_time.elapsed();
        let cpu_usage = self.cpu_profiler.current_usage() - start_cpu;
        let memory_usage = self.memory_profiler.current_usage() - start_memory;
        
        let profile = OperationProfile {
            operation_name: operation_name.to_string(),
            duration,
            cpu_usage,
            memory_usage,
            timestamp: chrono::Utc::now(),
        };
        
        (result, profile)
    }
}
```

---

## Deployment Considerations

### Container Optimization

```dockerfile
# Multi-stage build with performance optimizations
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests and build dependencies first (for caching)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source and build application
COPY src ./src
RUN cargo build --release

# Runtime stage with performance tuning
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/llm-supabase-rs /usr/local/bin/

# Performance tuning
ENV RUST_LOG=info
ENV TOKIO_WORKER_THREADS=8
ENV MALLOC_ARENA_MAX=2

# Resource limits
EXPOSE 8080
USER 1000:1000

CMD ["llm-supabase-rs"]
```

### Kubernetes Performance Configuration

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-proxy-performance
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    spec:
      containers:
      - name: llm-proxy
        image: llm-proxy:performance
        resources:
          requests:
            memory: "8Gi"
            cpu: "4"
          limits:
            memory: "16Gi"
            cpu: "8"
        env:
        - name: THREADING_HTTP_THREADS
          value: "4"
        - name: THREADING_AI_THREADS
          value: "8"
        - name: THREADING_AV_THREADS
          value: "4"
        - name: CONNECTION_POOL_SIZE
          value: "50"
        - name: MEMORY_POOL_ENABLED
          value: "true"
        livenessProbe:
          httpGet:
            path: /admin/health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
        readinessProbe:
          httpGet:
            path: /admin/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - llm-proxy
              topologyKey: kubernetes.io/hostname
```

---

## Continuous Performance Testing

### Automated Benchmarking Pipeline

```yaml
# .github/workflows/performance.yml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  performance:
    runs-on: ubuntu-latest-16-cores
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Build release
      run: cargo build --release
    
    - name: Run performance tests
      run: |
        cargo test --release --test performance_tests -- --nocapture
        
    - name: Generate performance report
      run: |
        cargo run --release --bin performance_benchmark > performance_report.json
        
    - name: Upload performance results
      uses: actions/upload-artifact@v3
      with:
        name: performance-results
        path: performance_report.json
    
    - name: Performance regression check
      run: |
        cargo run --release --bin regression_detector performance_report.json
```

### Load Testing Scenarios

```rust
// tests/performance/scenarios.rs

pub struct LoadTestScenarios;

impl LoadTestScenarios {
    /// Simulate real-world mixed workload
    pub async fn mixed_workload_test() -> anyhow::Result<TestResults> {
        let client = TestClient::new("http://localhost:8080").await?;
        
        // 70% chat completions, 20% embeddings, 10% tool calls
        let chat_weight = 70;
        let embedding_weight = 20;
        let tool_weight = 10;
        
        let total_requests = 10000;
        let concurrent_users = 100;
        
        let mut handles = Vec::new();
        
        for user_id in 0..concurrent_users {
            let client = client.clone();
            
            let handle = tokio::spawn(async move {
                let requests_per_user = total_requests / concurrent_users;
                let mut results = Vec::new();
                
                for i in 0..requests_per_user {
                    let request_type = match i % 100 {
                        0..=69 => RequestType::ChatCompletion,
                        70..=89 => RequestType::Embedding,
                        _ => RequestType::ToolCall,
                    };
                    
                    let start = std::time::Instant::now();
                    let result = match request_type {
                        RequestType::ChatCompletion => {
                            client.chat_completion(&format!("User {} message {}", user_id, i)).await
                        }
                        RequestType::Embedding => {
                            client.generate_embedding(&format!("Text to embed {}", i)).await
                        }
                        RequestType::ToolCall => {
                            client.execute_tool("web_search", &format!("query {}", i)).await
                        }
                    };
                    
                    results.push(RequestResult {
                        request_type,
                        duration: start.elapsed(),
                        success: result.is_ok(),
                        error: result.err().map(|e| e.to_string()),
                    });
                }
                
                results
            });
            
            handles.push(handle);
        }
        
        let all_results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        
        Ok(TestResults::analyze(all_results))
    }
    
    /// Test real-time audio processing under load
    pub async fn realtime_audio_stress_test() -> anyhow::Result<TestResults> {
        let client = TestClient::new("http://localhost:8080").await?;
        let concurrent_streams = 50;
        let stream_duration = Duration::from_secs(60);
        
        let mut handles = Vec::new();
        
        for stream_id in 0..concurrent_streams {
            let client = client.clone();
            
            let handle = tokio::spawn(async move {
                let audio_generator = AudioGenerator::new(48000, stream_duration);
                let mut results = Vec::new();
                
                let mut stream = audio_generator.generate_stream();
                while let Some(audio_chunk) = stream.next().await {
                    let start = std::time::Instant::now();
                    
                    let result = client.process_audio_realtime(
                        stream_id,
                        audio_chunk,
                    ).await;
                    
                    results.push(AudioProcessingResult {
                        stream_id,
                        chunk_duration: audio_chunk.duration,
                        processing_latency: start.elapsed(),
                        success: result.is_ok(),
                    });
                }
                
                results
            });
            
            handles.push(handle);
        }
        
        let all_results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        
        Ok(TestResults::from_audio_results(all_results))
    }
}
```

---

## Performance Tuning Guide

### 1. Runtime Configuration

```rust
// Optimal runtime configuration based on workload
pub fn configure_optimal_runtime(workload_type: WorkloadType) -> ThreadingConfig {
    let cpu_count = num_cpus::get();
    
    match workload_type {
        WorkloadType::HighThroughputApi => ThreadingConfig {
            http_threads: cpu_count / 2,
            ai_threads: cpu_count / 4,
            av_threads: 2,
            db_threads: 2,
            mcp_threads: cpu_count / 4,
            max_concurrent_http: 2000,
            max_concurrent_ai: 200,
            max_concurrent_av: 50,
        },
        
        WorkloadType::RealTimeAudio => ThreadingConfig {
            http_threads: cpu_count / 4,
            ai_threads: cpu_count / 4,
            av_threads: cpu_count / 2, // Prioritize audio processing
            db_threads: 2,
            mcp_threads: 2,
            max_concurrent_http: 500,
            max_concurrent_ai: 100,
            max_concurrent_av: 200, // High audio concurrency
        },
        
        WorkloadType::MixedWorkload => ThreadingConfig::default(),
    }
}
```

### 2. Memory Tuning

```rust
// Memory allocation strategies
pub struct MemoryTuning;

impl MemoryTuning {
    /// Configure jemalloc for optimal performance
    pub fn configure_allocator() {
        std::env::set_var("MALLOC_CONF", 
            "background_thread:true,metadata_thp:auto,dirty_decay_ms:5000,muzzy_decay_ms:10000");
    }
    
    /// Set up memory pools based on expected workload
    pub fn configure_memory_pools(expected_load: ExpectedLoad) -> MemoryPoolConfig {
        match expected_load {
            ExpectedLoad::Light => MemoryPoolConfig {
                audio_buffer_count: 50,
                response_buffer_count: 100,
                embedding_buffer_count: 25,
                ..Default::default()
            },
            
            ExpectedLoad::Heavy => MemoryPoolConfig {
                audio_buffer_count: 200,
                response_buffer_count: 500,
                embedding_buffer_count: 100,
                audio_buffer_size: 96000, // 2 seconds at 48kHz
                response_buffer_size: 128 * 1024, // 128KB
                ..Default::default()
            },
            
            ExpectedLoad::RealTime => MemoryPoolConfig {
                audio_buffer_count: 500, // Large pool for real-time
                response_buffer_count: 200,
                embedding_buffer_count: 50,
                audio_buffer_size: 48000, // 1 second for low latency
                ..Default::default()
            },
        }
    }
}
```

### 3. Network Optimization

```rust
// Network performance tuning
pub struct NetworkOptimizer;

impl NetworkOptimizer {
    pub fn configure_http_client(workload: WorkloadType) -> reqwest::ClientBuilder {
        let mut builder = reqwest::Client::builder()
            .http2_prior_knowledge()
            .http2_adaptive_window(true)
            .tcp_nodelay(true);
            
        match workload {
            WorkloadType::HighThroughput => {
                builder = builder
                    .pool_max_idle_per_host(20)
                    .pool_idle_timeout(Duration::from_secs(90))
                    .timeout(Duration::from_secs(30));
            }
            
            WorkloadType::LowLatency => {
                builder = builder
                    .pool_max_idle_per_host(50)
                    .pool_idle_timeout(Duration::from_secs(30))
                    .timeout(Duration::from_secs(10))
                    .tcp_keepalive(Duration::from_secs(60));
            }
            
            WorkloadType::RealTime => {
                builder = builder
                    .pool_max_idle_per_host(100)
                    .pool_idle_timeout(Duration::from_secs(10))
                    .timeout(Duration::from_secs(5))
                    .tcp_keepalive(Duration::from_secs(30));
            }
        }
        
        builder
    }
}
```

---

## Success Metrics and KPIs

### Performance KPIs

```rust
pub struct PerformanceKPIs {
    // Latency KPIs
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    
    // Throughput KPIs
    pub requests_per_second: f64,
    pub concurrent_requests: usize,
    pub successful_request_rate: f64,
    
    // Resource efficiency KPIs
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub thread_pool_efficiency: f64,
    pub connection_pool_efficiency: f64,
    
    // Real-time KPIs
    pub audio_processing_latency_ms: f64,
    pub transcription_accuracy: f64,
    pub agent_join_latency_ms: f64,
    pub concurrent_audio_streams: usize,
    
    // Reliability KPIs
    pub uptime_percentage: f64,
    pub error_rate: f64,
    pub sandbox_failure_rate: f64,
    pub recovery_time_ms: f64,
}

impl PerformanceKPIs {
    pub fn meets_targets(&self) -> bool {
        self.p95_latency_ms < 500.0 &&
        self.requests_per_second > 2500.0 &&
        self.cpu_utilization < 85.0 &&
        self.memory_utilization < 80.0 &&
        self.error_rate < 0.01 &&
        self.uptime_percentage > 99.9
    }
    
    pub fn performance_grade(&self) -> PerformanceGrade {
        let score = self.calculate_performance_score();
        
        match score {
            90..=100 => PerformanceGrade::Excellent,
            80..=89 => PerformanceGrade::Good,
            70..=79 => PerformanceGrade::Acceptable,
            _ => PerformanceGrade::NeedsImprovement,
        }
    }
}
```

This comprehensive benchmarking and optimization guide provides the framework for measuring and improving the performance of the threading architecture implementation.