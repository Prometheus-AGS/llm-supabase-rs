// src/api/handlers/chat.rs
//
// Chat completions handler implementation

use axum::{
    extract::{Request, State},
    response::{IntoResponse, Json, Response},
};
use serde_json::{self, Value, json};
use std::time::SystemTime;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use async_stream;

use crate::app::AppState;
use crate::infrastructure::{
    vertex::FormatConverter,
    common::{
        ToolCallManager,
    },
};
use crate::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
    common::MessageRole,
};
use crate::shared::AppError;

/// Unified chat completions handler that routes based on stream parameter
///
/// This handler examines the request body to determine whether to use streaming or non-streaming
/// and routes to the appropriate handler while maintaining OpenAI API compatibility
#[instrument(skip(state, request))]
pub async fn chat_completions_unified(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, AppError> {
    let request_id = Uuid::new_v4().to_string();

    // Split request to access parts and body separately
    let (parts, body) = request.into_parts();

    // Extract request body to examine stream parameter
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to read request body");
            return Err(AppError::validation("Failed to read request body"));
        }
    };

    // Parse just enough to check the stream parameter
    let parsed_request: ChatCompletionRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to parse request for routing");
            return Err(AppError::validation(format!("Invalid request format: {}", e)));
        }
    };

    // Reconstruct request with body for handler
    let reconstructed_request = Request::from_parts(
        parts,
        axum::body::Body::from(body_bytes)
    );

    // Route to appropriate handler based on stream parameter
    if parsed_request.stream.unwrap_or(false) {
        debug!(request_id = %request_id, "Routing to streaming handler");
        let sse_response = chat_completions_streaming(State(state), reconstructed_request).await?;
        Ok(sse_response.into_response())
    } else {
        debug!(request_id = %request_id, "Routing to non-streaming handler");
        let json_response = chat_completions(State(state), reconstructed_request).await?;
        Ok(json_response.into_response())
    }
}

/// Chat completions handler (non-streaming)
///
/// Handles OpenAI-compatible chat completion requests and routes them to Vertex AI
/// This is the core endpoint for generating chat completions
#[instrument(skip(state, request))]
pub async fn chat_completions(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ChatCompletionResponse>, AppError> {
    let start_time = SystemTime::now();
    let request_id = Uuid::new_v4().to_string();

    info!(request_id = %request_id, "Processing chat completion request");

    // Extract request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to read request body");
            return Err(AppError::validation("Failed to read request body"));
        }
    };

    // Parse chat completion request
    let chat_request: ChatCompletionRequest = match serde_json::from_slice::<ChatCompletionRequest>(&body_bytes) {
        Ok(req) => {
            debug!(
                request_id = %request_id,
                model = %req.model,
                messages_count = req.messages.len(),
                max_tokens = ?req.max_tokens,
                temperature = ?req.temperature,
                stream = req.stream,
                "Parsed chat completion request"
            );
            req
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to parse chat completion request");
            return Err(AppError::validation(format!("Invalid request format: {}", e)));
        }
    };

    // Validate request
    validate_chat_request(&chat_request, &request_id)?;

    // Initialize tool call manager if tools are provided
    let _tool_manager = if chat_request.tools.is_some() {
        debug!(request_id = %request_id, tools_count = chat_request.tools.as_ref().unwrap().len(), "Tools detected, initializing tool call manager");
        Some(ToolCallManager::claude())
    } else {
        None
    };

    // Check if streaming is requested (should be handled by different endpoint)
    if chat_request.stream.unwrap_or(false) {
        warn!(request_id = %request_id, "Streaming request received at non-streaming endpoint");
        return Err(AppError::validation("Streaming requests should use the streaming endpoint"));
    }

    // Convert OpenAI format to Vertex AI format
    let vertex_request = match FormatConverter::openai_to_vertex_v2(&chat_request) {
        Ok(req) => {
            debug!(request_id = %request_id, "Converted OpenAI request to Vertex AI format");
            req
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to convert request format");
            return Err(AppError::internal(format!("Request conversion failed: {}", e)));
        }
    };

    // Make request to Vertex AI
    let vertex_response = match state.vertex_client.predict(&chat_request.model, vertex_request).await {
        Ok(response) => {
            debug!(request_id = %request_id, "Received response from Vertex AI");
            response
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Vertex AI request failed");
            return Err(AppError::vertex_ai(format!("AI service error: {}", e)));
        }
    };

    // Convert Vertex AI response to OpenAI format
    let openai_response = match FormatConverter::vertex_to_openai_v2(
        &vertex_response,
        &request_id,
        &chat_request.model,
        &chat_request,
    ) {
        Ok(response) => {
            let duration = start_time.elapsed().unwrap_or_default();
            info!(
                request_id = %request_id,
                model = %response.model,
                usage_total_tokens = response.usage.total_tokens,
                duration_ms = duration.as_millis(),
                "Chat completion successful"
            );
            response
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to convert response format");
            return Err(AppError::internal(format!("Response conversion failed: {}", e)));
        }
    };

    Ok(Json(openai_response))
}

/// Chat completions handler (streaming)
///
/// Handles OpenAI-compatible chat completion requests with Server-Sent Events streaming
/// Streams real-time responses from Vertex AI Claude 4 Sonnet
#[instrument(skip(state, request))]
pub async fn chat_completions_streaming(
    State(state): State<AppState>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    let start_time = SystemTime::now();
    let request_id = Uuid::new_v4().to_string();

    info!(request_id = %request_id, "Processing streaming chat completion request");

    // Extract request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to read request body");
            return Err(AppError::validation("Failed to read request body"));
        }
    };

    // Parse chat completion request
    let chat_request: ChatCompletionRequest = match serde_json::from_slice::<ChatCompletionRequest>(&body_bytes) {
        Ok(req) => {
            debug!(
                request_id = %request_id,
                model = %req.model,
                messages_count = req.messages.len(),
                max_tokens = ?req.max_tokens,
                temperature = ?req.temperature,
                stream = req.stream,
                "Parsed streaming chat completion request"
            );
            req
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to parse chat completion request");
            return Err(AppError::validation(format!("Invalid request format: {}", e)));
        }
    };

    // Validate request
    validate_chat_request(&chat_request, &request_id)?;

    // Initialize tool call manager if tools are provided
    let mut tool_manager = if chat_request.tools.is_some() {
        debug!(request_id = %request_id, tools_count = chat_request.tools.as_ref().unwrap().len(), "Tools detected, initializing tool call manager");
        Some(ToolCallManager::claude())
    } else {
        None
    };

    // Convert OpenAI format to Vertex AI streaming format
    let vertex_request = match FormatConverter::openai_to_vertex_streaming(&chat_request) {
        Ok(req) => {
            debug!(request_id = %request_id, "Converted OpenAI request to Vertex AI streaming format");
            req
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to convert request format");
            return Err(AppError::internal(format!("Request conversion failed: {}", e)));
        }
    };

    // Start streaming from Vertex AI
    info!(request_id = %request_id, "Attempting to start streaming from Vertex AI...");
    let mut vertex_stream = match state.vertex_client.predict_streaming(&chat_request.model, vertex_request).await {
        Ok(stream) => {
            info!(request_id = %request_id, "✅ Started streaming from Vertex AI successfully!");
            stream
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "❌ Failed to start Vertex AI streaming");
            return Err(AppError::vertex_ai(format!("AI streaming error: {}", e)));
        }
    };

    // Create Server-Sent Events stream
    let stream = async_stream::stream! {
        use tokio_stream::StreamExt;
        
        info!(request_id = %request_id, "🌊 Starting to process Vertex AI stream chunks...");
        let mut chunk_count = 0;
        
        while let Some(chunk_result) = vertex_stream.next().await {
            chunk_count += 1;
            info!(request_id = %request_id, chunk_number = chunk_count, "📦 Received chunk from Vertex AI");
            match chunk_result {
                Ok(vertex_chunk) => {
                    debug!(request_id = %request_id, event_type = %vertex_chunk.event_type, "Processing Vertex AI chunk");
                    
                    // Skip non-content chunks
                    if vertex_chunk.event_type == "skip" {
                        continue;
                    }
                    
                    // Convert Vertex AI chunk to OpenAI format
                    match FormatConverter::vertex_chunk_to_openai_v2(
                        &vertex_chunk,
                        &request_id,
                        &chat_request.model,
                    ) {
                        Ok(Some(openai_chunk)) => {
                            debug!(request_id = %request_id, "Successfully converted chunk to OpenAI format");
                            
                            // Check for tool calls in the chunk
                            if let Some(ref mut tool_mgr) = tool_manager {
                                if let Some(choice) = openai_chunk.choices.first() {
                                    // Check if this chunk contains tool calls
                                    if let Some(tool_calls) = &choice.delta.tool_calls {
                                        debug!(request_id = %request_id, tool_calls_count = tool_calls.len(), "Tool calls detected in streaming chunk");
                                        
                                        // Convert tool calls to unified format
                                        let tool_calls_json = serde_json::to_value(tool_calls).unwrap_or_default();
                                        if let Ok(Some(_unified_calls)) = tool_mgr.process_tool_calls(&tool_calls_json, request_id.clone()) {
                                            debug!(request_id = %request_id, "Tool calls processed, waiting for client response");
                                            // The tool calls will be sent to the client in the normal chunk
                                            // The client should respond with tool results
                                        }
                                    }
                                    
                                    // Check if this indicates tool calls are complete
                                    if choice.finish_reason.as_ref().map(|r| r.to_string()) == Some("tool_calls".to_string()) {
                                        debug!(request_id = %request_id, "Tool calls completed, stream will pause for tool results");
                                    }
                                }
                            }
                            
                            // Format as Server-Sent Event
                            let chunk_json = match serde_json::to_string(&openai_chunk) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!(request_id = %request_id, error = %e, "Failed to serialize chunk");
                                    continue;
                                }
                            };
                            
                            let sse_data = format!("data: {}\n\n", chunk_json);
                            debug!(request_id = %request_id, "Sending SSE chunk: {}", sse_data.trim());
                            yield Ok::<_, axum::Error>(sse_data);
                        }
                        Ok(None) => {
                            debug!(request_id = %request_id, "Converter returned None, skipping chunk");
                            continue;
                        }
                        Err(e) => {
                            error!(request_id = %request_id, error = %e, "Failed to convert chunk");
                            let error_data = format!("data: {{\"error\": \"Chunk conversion failed: {}\"}}\n\n", e);
                            yield Ok::<_, axum::Error>(error_data);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!(request_id = %request_id, error = %e, "Vertex AI streaming error");
                    let error_data = format!("data: {{\"error\": \"Streaming error: {}\"}}\n\n", e);
                    yield Ok::<_, axum::Error>(error_data);
                    break;
                }
            }
        }
        
        if chunk_count == 0 {
            warn!(request_id = %request_id, "⚠️  No chunks received from Vertex AI - this explains the issue!");
        } else {
            info!(request_id = %request_id, total_chunks = chunk_count, "✅ Processed all chunks from Vertex AI");
        }
        
        // Send final [DONE] message
        yield Ok::<_, axum::Error>("data: [DONE]\n\n".to_string());
        
        let duration = start_time.elapsed().unwrap_or_default();
        info!(
            request_id = %request_id,
            duration_ms = duration.as_millis(),
            total_chunks_processed = chunk_count,
            "Completed streaming chat completion"
        );
    };

    // Return Server-Sent Events response
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .body(axum::body::Body::from_stream(stream))
        .map_err(|e| AppError::internal(format!("Failed to create streaming response: {}", e)))?;

    Ok(response)
}

// TODO: Implement streaming support properly
// Complex streaming implementation temporarily removed for compilation

// TODO: Streaming implementation commented out for compilation
// Will be reimplemented with proper Rig integration

/// Validate chat completion request parameters
fn validate_chat_request(request: &ChatCompletionRequest, request_id: &str) -> Result<(), AppError> {
    // Validate messages
    if request.messages.is_empty() {
        warn!(request_id = %request_id, "Empty messages array");
        return Err(AppError::validation("Messages array cannot be empty"));
    }

    // Validate message content
    for (i, message) in request.messages.iter().enumerate() {
        // Allow empty content for assistant messages that have tool calls
        let has_tool_calls = message.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty());
        let has_function_call = message.function_call.is_some();
        
        if message.content.is_empty() && !has_tool_calls && !has_function_call {
            warn!(request_id = %request_id, message_index = i, "Empty message content without tool calls");
            return Err(AppError::validation(format!("Message {} has empty content", i)));
        }

        // Validate role
        match message.role {
            MessageRole::System | MessageRole::User | MessageRole::Assistant => {}
            MessageRole::Tool => {
                if message.tool_call_id.is_none() {
                    warn!(request_id = %request_id, message_index = i, "Tool message missing tool_call_id");
                    return Err(AppError::validation("Tool messages must have tool_call_id"));
                }
            }
            MessageRole::Function => {
                // Function role is deprecated but still supported for backward compatibility
                debug!(request_id = %request_id, message_index = i, "Using deprecated Function role");
            }
        }
    }

    // Validate model
    if !request.model.contains("claude") {
        warn!(request_id = %request_id, model = %request.model, "Unsupported model");
        return Err(AppError::validation(format!("Unsupported model: {}", request.model)));
    }

    // Validate parameters
    if let Some(max_tokens) = request.max_tokens {
        if max_tokens == 0 || max_tokens > 4096 {
            warn!(request_id = %request_id, max_tokens = max_tokens, "Invalid max_tokens");
            return Err(AppError::validation("max_tokens must be between 1 and 4096"));
        }
    }

    if let Some(temperature) = request.temperature {
        if !(0.0..=2.0).contains(&temperature) {
            warn!(request_id = %request_id, temperature = temperature, "Invalid temperature");
            return Err(AppError::validation("temperature must be between 0.0 and 2.0"));
        }
    }

    if let Some(top_p) = request.top_p {
        if top_p <= 0.0 || top_p > 1.0 {
            warn!(request_id = %request_id, top_p = top_p, "Invalid top_p");
            return Err(AppError::validation("top_p must be between 0.0 and 1.0"));
        }
    }

    debug!(request_id = %request_id, "Request validation passed");
    Ok(())
}

/// Create error response in OpenAI format (for future use)
#[allow(dead_code)]
fn create_error_response(message: &str, error_type: &str, request_id: Option<String>) -> Json<Value> {
    let error_response = json!({
        "error": {
            "message": message,
            "type": error_type,
            "param": null,
            "code": null,
            "request_id": request_id
        }
    });

    Json(error_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    fn create_test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-sonnet-4@20250514".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello, how are you?".to_string(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            max_tokens: Some(150),
            temperature: Some(0.7),
            top_p: Some(1.0),
            stream: Some(false),
            stop: None,
            n: Some(1),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
            logit_bias: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            seed: None,
            logprobs: None,
            top_logprobs: None,
            metadata: None,
            parallel_tool_calls: None,
            stream_options: None,
            service_tier: None,
            store: None,
        }
    }

    #[test]
    fn test_validate_chat_request_valid() {
        let request = create_test_request();
        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chat_request_empty_messages() {
        let mut request = create_test_request();
        request.messages.clear();

        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_chat_request_invalid_model() {
        let mut request = create_test_request();
        request.model = "gpt-4".to_string();

        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported model"));
    }

    #[test]
    fn test_validate_chat_request_invalid_max_tokens() {
        let mut request = create_test_request();
        request.max_tokens = Some(5000);

        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_tokens"));
    }

    #[test]
    fn test_validate_chat_request_invalid_temperature() {
        let mut request = create_test_request();
        request.temperature = Some(-0.5);

        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("temperature"));
    }

    #[test]
    fn test_validate_chat_request_invalid_top_p() {
        let mut request = create_test_request();
        request.top_p = Some(1.5);

        let result = validate_chat_request(&request, "test-123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("top_p"));
    }

    #[test]
    fn test_create_error_response() {
        let response = create_error_response("Test error", "validation_error", Some("test-123".to_string()));
        let json_value = response.0;

        assert!(json_value["error"]["message"].as_str().unwrap().contains("Test error"));
        assert_eq!(json_value["error"]["type"].as_str().unwrap(), "validation_error");
        assert_eq!(json_value["error"]["request_id"].as_str().unwrap(), "test-123");
    }

    // Note: Authentication extraction tests would require proper request setup with extensions

    // Integration tests would require actual Vertex AI setup
    // #[tokio::test]
    // #[ignore]
    // async fn test_chat_completions_integration() {
    //     // Would test actual chat completion flow
    // }
}