// Example: Enhanced logging in chat handlers
// This shows how to improve your existing API handlers with better logging

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::{info, error, warn, debug, instrument};
use uuid::Uuid;

use crate::models::{ChatRequest, ChatResponse};
use crate::app::AppState;

/// Example of a well-instrumented handler
#[instrument(
    skip(state, request),  // Skip large or sensitive data
    fields(
        request_id = %Uuid::new_v4(),  // Generate request ID
        model = %request.model,         // Log the model name
    )
)]
pub async fn chat_completions_handler(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // The request_id and model are automatically added to all logs in this function
    info!("Processing chat completion request");
    
    // Log with additional context
    debug!(
        messages_count = request.messages.len(),
        max_tokens = request.max_tokens,
        temperature = request.temperature,
        "Request parameters"
    );

    // Validate request
    if request.messages.is_empty() {
        warn!("Empty messages array in request");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Call the AI service
    match state.vertex_client.predict(&request.model, request.into()).await {
        Ok(response) => {
            info!(
                completion_tokens = response.usage.completion_tokens,
                prompt_tokens = response.usage.prompt_tokens,
                "Request completed successfully"
            );
            
            Ok(Json(response))
        }
        Err(e) => {
            error!(
                error = %e,
                error_debug = ?e,
                "Failed to process chat completion"
            );
            
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Example of logging in a background task
#[instrument(skip(client))]
pub async fn token_refresh_task(client: Arc<VertexAuthClient>) {
    info!("Token refresh task started");
    
    loop {
        tokio::time::sleep(Duration::from_secs(3300)).await;
        
        match client.refresh_token().await {
            Ok(_) => debug!("Token refreshed successfully"),
            Err(e) => error!(error = %e, "Failed to refresh token"),
        }
    }
}

/// Example of span creation for complex operations
pub async fn process_batch(items: Vec<Item>) -> Result<()> {
    let batch_span = tracing::info_span!(
        "process_batch",
        batch_size = items.len(),
        batch_id = %Uuid::new_v4()
    );
    
    let _guard = batch_span.enter();
    info!("Starting batch processing");
    
    for (idx, item) in items.iter().enumerate() {
        // Create a sub-span for each item
        let item_span = tracing::debug_span!(
            "process_item",
            item_index = idx,
            item_id = %item.id
        );
        
        let _item_guard = item_span.enter();
        
        match process_item(item).await {
            Ok(_) => debug!("Item processed"),
            Err(e) => warn!(error = %e, "Item processing failed"),
        }
    }
    
    info!("Batch processing complete");
    Ok(())
}

/// Example of conditional debug logging
pub fn expensive_operation(data: &[u8]) -> Result<()> {
    // Only compute debug info if debug logging is enabled
    if tracing::enabled!(tracing::Level::DEBUG) {
        let stats = compute_statistics(data);
        debug!(?stats, "Operation statistics");
    }
    
    // Regular processing
    Ok(())
}

/// Example of error chain logging
pub async fn complex_operation() -> Result<()> {
    database_query()
        .await
        .map_err(|e| {
            error!(
                error = %e,
                context = "Failed to query database",
                "Database operation failed"
            );
            e
        })?;
    
    api_call()
        .await
        .map_err(|e| {
            error!(
                error = %e,
                context = "Failed to call external API",
                "API operation failed"  
            );
            e
        })?;
    
    Ok(())
}
