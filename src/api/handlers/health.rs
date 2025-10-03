// src/api/handlers/health.rs
//
// Health endpoint handler implementation
// Provides system health status and service availability information

use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, instrument};

use crate::app::AppState;
use crate::shared::AppError;

/// Health check response structure
/// Matches OpenAI API health endpoint format with additional system information
#[derive(Debug, serde::Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,

    /// Timestamp when health check was performed
    pub timestamp: u64,

    /// Service version
    pub version: String,

    /// Environment information
    pub environment: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// System health details
    pub system: SystemHealth,

    /// Service dependencies status
    pub dependencies: DependenciesHealth,
}

/// System health metrics
#[derive(Debug, serde::Serialize)]
pub struct SystemHealth {
    /// Memory usage information
    pub memory: MemoryInfo,

    /// Process information
    pub process: ProcessInfo,
}

/// Memory usage information
#[derive(Debug, serde::Serialize)]
pub struct MemoryInfo {
    /// Used memory in bytes (if available)
    pub used_bytes: Option<u64>,

    /// Available memory in bytes (if available)
    pub available_bytes: Option<u64>,

    /// Memory usage percentage (if calculable)
    pub usage_percent: Option<f64>,
}

/// Process information
#[derive(Debug, serde::Serialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,

    /// Number of threads
    pub threads: Option<u32>,

    /// CPU usage percentage (if available)
    pub cpu_percent: Option<f64>,
}

/// Dependencies health status
#[derive(Debug, serde::Serialize)]
pub struct DependenciesHealth {
    /// Vertex AI service status
    pub vertex_ai: ServiceStatus,

    /// Supabase service status
    pub supabase: ServiceStatus,

    /// Database connection status
    pub database: ServiceStatus,
}

/// Individual service status
#[derive(Debug, serde::Serialize)]
pub struct ServiceStatus {
    /// Service status
    pub status: String,

    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,

    /// Last check timestamp
    pub last_check: u64,

    /// Error message if service is unhealthy
    pub error: Option<String>,
}

/// Health check handler
///
/// This endpoint provides comprehensive health information about the service
/// and its dependencies. It's used by load balancers and monitoring systems.
#[instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    let start_time = SystemTime::now();
    let timestamp = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    debug!("Processing health check request");

    // Calculate uptime (we don't have start_time in AppState, so use 0 for now)
    let uptime_seconds = 0;

    // Gather system information
    let system_health = gather_system_health().await;

    // Check service dependencies
    let dependencies_health = check_dependencies(&state).await;

    // Determine overall status based on dependencies
    let overall_status = determine_overall_status(&dependencies_health);

    let health_response = HealthResponse {
        status: overall_status,
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string()),
        uptime_seconds,
        system: system_health,
        dependencies: dependencies_health,
    };

    let duration = start_time.elapsed().unwrap_or_default();
    debug!(
        status = %health_response.status,
        duration_ms = duration.as_millis(),
        "Health check completed"
    );

    Ok(Json(health_response))
}

/// Simple health check handler (minimal response)
///
/// Provides a lightweight health check endpoint for basic availability monitoring
#[instrument(skip(state))]
pub async fn simple_health_check(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    debug!("Processing simple health check request");

    // Perform basic dependency checks
    let dependencies = check_dependencies(&state).await;
    let status = determine_overall_status(&dependencies);

    let response = json!({
        "status": status,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });

    Ok(Json(response))
}

/// Readiness probe handler
///
/// Indicates whether the service is ready to accept traffic
/// Used by Kubernetes readiness probes
#[instrument(skip(state))]
pub async fn readiness_check(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    debug!("Processing readiness check");

    // Check critical dependencies
    let vertex_status = check_vertex_ai_health(&state).await;
    let supabase_status = check_supabase_health(&state).await;

    if vertex_status.status == "healthy" && supabase_status.status == "healthy" {
        Ok(StatusCode::OK)
    } else {
        error!(
            vertex_status = %vertex_status.status,
            supabase_status = %supabase_status.status,
            "Service not ready - dependencies unhealthy"
        );
        Ok(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Liveness probe handler
///
/// Indicates whether the service is alive and should not be restarted
/// Used by Kubernetes liveness probes
#[instrument]
pub async fn liveness_check() -> StatusCode {
    // For a simple liveness check, we just return OK
    // In a more complex setup, this could check for deadlocks, memory leaks, etc.
    debug!("Processing liveness check");
    StatusCode::OK
}

/// Gather system health information
async fn gather_system_health() -> SystemHealth {
    let memory_info = get_memory_info().await;
    let process_info = get_process_info().await;

    SystemHealth {
        memory: memory_info,
        process: process_info,
    }
}

/// Get memory usage information
async fn get_memory_info() -> MemoryInfo {
    // In a production environment, you might use system APIs or libraries
    // like `sysinfo` to get actual memory usage. For now, we'll return None values.
    MemoryInfo {
        used_bytes: None,
        available_bytes: None,
        usage_percent: None,
    }
}

/// Get process information
async fn get_process_info() -> ProcessInfo {
    ProcessInfo {
        pid: std::process::id(),
        threads: None, // Could use thread count from system APIs
        cpu_percent: None, // Could calculate CPU usage over time
    }
}

/// Check all service dependencies
async fn check_dependencies(state: &AppState) -> DependenciesHealth {
    let vertex_ai = check_vertex_ai_health(state).await;
    let supabase = check_supabase_health(state).await;
    let database = check_database_health(state).await;

    DependenciesHealth {
        vertex_ai,
        supabase,
        database,
    }
}

/// Check Vertex AI service health
async fn check_vertex_ai_health(state: &AppState) -> ServiceStatus {
    let start_time = SystemTime::now();
    let timestamp = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Attempt a simple health check to Vertex AI
    match state.vertex_client.health_check().await {
        Ok(_) => {
            let response_time = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(response_time),
                last_check: timestamp,
                error: None,
            }
        }
        Err(e) => {
            error!(error = %e, "Vertex AI health check failed");
            ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: None,
                last_check: timestamp,
                error: Some(e.to_string()),
            }
        }
    }
}

/// Check Supabase service health
async fn check_supabase_health(state: &AppState) -> ServiceStatus {
    let start_time = SystemTime::now();
    let timestamp = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Attempt a simple health check to Supabase
    match state.supabase_client.health_check().await {
        Ok(_) => {
            let response_time = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(response_time),
                last_check: timestamp,
                error: None,
            }
        }
        Err(e) => {
            error!(error = %e, "Supabase health check failed");
            ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: None,
                last_check: timestamp,
                error: Some(e.to_string()),
            }
        }
    }
}

/// Check database connection health
async fn check_database_health(_state: &AppState) -> ServiceStatus {
    let start_time = SystemTime::now();
    let timestamp = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Database is not configured in AppState for now
    ServiceStatus {
        status: "not_configured".to_string(),
        response_time_ms: None,
        last_check: timestamp,
        error: None,
    }
}

/// Determine overall service status based on dependencies
fn determine_overall_status(dependencies: &DependenciesHealth) -> String {
    // Service is healthy if critical dependencies are healthy
    if dependencies.vertex_ai.status == "healthy" && dependencies.supabase.status == "healthy" {
        "healthy".to_string()
    } else if dependencies.vertex_ai.status == "unhealthy" || dependencies.supabase.status == "unhealthy" {
        "unhealthy".to_string()
    } else {
        "degraded".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_overall_status() {
        let healthy_deps = DependenciesHealth {
            vertex_ai: ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(100),
                last_check: 1234567890,
                error: None,
            },
            supabase: ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(50),
                last_check: 1234567890,
                error: None,
            },
            database: ServiceStatus {
                status: "not_configured".to_string(),
                response_time_ms: None,
                last_check: 1234567890,
                error: None,
            },
        };

        assert_eq!(determine_overall_status(&healthy_deps), "healthy");

        let unhealthy_deps = DependenciesHealth {
            vertex_ai: ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: None,
                last_check: 1234567890,
                error: Some("Connection failed".to_string()),
            },
            supabase: ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(50),
                last_check: 1234567890,
                error: None,
            },
            database: ServiceStatus {
                status: "not_configured".to_string(),
                response_time_ms: None,
                last_check: 1234567890,
                error: None,
            },
        };

        assert_eq!(determine_overall_status(&unhealthy_deps), "unhealthy");
    }

    #[tokio::test]
    async fn test_memory_info() {
        let memory_info = get_memory_info().await;

        // For now, we expect None values since we haven't implemented actual memory checking
        assert!(memory_info.used_bytes.is_none());
        assert!(memory_info.available_bytes.is_none());
        assert!(memory_info.usage_percent.is_none());
    }

    #[tokio::test]
    async fn test_process_info() {
        let process_info = get_process_info().await;

        // Process ID should always be available
        assert!(process_info.pid > 0);

        // These might be None depending on the system
        // assert!(process_info.threads.is_none()); // Could be implemented
        // assert!(process_info.cpu_percent.is_none()); // Could be implemented
    }
}