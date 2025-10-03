// tests/integration/test_vertex_e2e.rs
//
// Real end-to-end integration tests for Vertex AI
// Tests the complete flow: OpenAI API → Vertex AI → OpenAI Response

use std::env;
use tokio;
use base64::{Engine as _, engine::general_purpose};

use llm_supabase_rs::{
    config::providers::VertexAiConfig,
    infrastructure::vertex::{
        VertexAIClient,
        FormatConverter,
    },
    models::{
        request::ChatCompletionRequest,
        common::{ChatMessage, MessageRole},
    },
};

/// Test the complete OpenAI → Vertex AI → OpenAI flow with real API calls
/// 
/// This test requires:
/// - Valid GCP credentials in .env file
/// - GOOGLE_APPLICATION_CREDENTIALS or service account key
/// - GCP_PROJECT_ID set in environment
/// - Vertex AI API enabled
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_e2e_flow() {
    println!("🚀 Starting real Vertex AI end-to-end test...");

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // 1. Create configuration from environment
    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    println!("📋 Configuration loaded:");
    println!("  Project ID: {}", config.project_id);
    println!("  Region: {}", config.region);
    println!("  Model: {}", config.default_model.model_name);

    // 2. Initialize Vertex AI client
    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    println!("✅ Vertex AI client initialized successfully");

    // 3. Create OpenAI-style request (what your API would receive)
    let openai_request = ChatCompletionRequest {
        model: config.default_model.model_name.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant. Keep responses brief.".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello! Please respond with exactly: 'Test successful'".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(50),
        temperature: Some(0.1), // Low temperature for predictable response
        top_p: None,
        n: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    println!("📤 OpenAI Request created:");
    println!("  Model: {}", openai_request.model);
    println!("  Messages: {} messages", openai_request.messages.len());
    println!("  Max tokens: {:?}", openai_request.max_tokens);

    // 4. Convert OpenAI request to Vertex AI format
    let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
        .expect("Failed to convert OpenAI request to Vertex AI format");

    println!("🔄 Converted to Vertex AI format:");
    println!("  Anthropic version: {}", vertex_request.anthropic_version);
    println!("  Max tokens: {}", vertex_request.max_tokens);
    println!("  System: {:?}", vertex_request.system);
    println!("  Messages: {} messages", vertex_request.messages.len());

    // 5. Make actual API call to Vertex AI
    println!("📡 Making API call to Vertex AI...");
    let vertex_response = vertex_client
        .predict(&config.default_model.model_name, vertex_request)
        .await
        .expect("Failed to get response from Vertex AI");

    println!("✅ Received response from Vertex AI:");
    println!("  Response ID: {}", vertex_response.id);
    println!("  Model: {}", vertex_response.model);
    println!("  Content blocks: {}", vertex_response.content.len());
    println!("  Usage - Input: {}, Output: {}", 
        vertex_response.usage.input_tokens, 
        vertex_response.usage.output_tokens
    );

    // 6. Convert Vertex AI response back to OpenAI format
    let openai_response = FormatConverter::vertex_to_openai_v2(
        &vertex_response,
        "chatcmpl-e2e-test",
        &config.default_model.model_name,
        &openai_request,
    ).expect("Failed to convert Vertex AI response to OpenAI format");

    println!("🔄 Converted back to OpenAI format:");
    println!("  Response ID: {}", openai_response.id);
    println!("  Object: {}", openai_response.object);
    println!("  Model: {}", openai_response.model);
    println!("  Choices: {}", openai_response.choices.len());

    // 7. Validate the complete flow
    assert_eq!(openai_response.object, "chat.completion");
    assert_eq!(openai_response.model, config.default_model.model_name);
    assert_eq!(openai_response.choices.len(), 1);
    assert_eq!(openai_response.choices[0].index, 0);
    assert_eq!(openai_response.choices[0].message.role, MessageRole::Assistant);
    assert!(!openai_response.choices[0].message.content.is_empty());

    // Validate usage statistics
    assert!(openai_response.usage.prompt_tokens > 0);
    assert!(openai_response.usage.completion_tokens > 0);
    assert_eq!(
        openai_response.usage.total_tokens,
        openai_response.usage.prompt_tokens + openai_response.usage.completion_tokens
    );

    let response_content = &openai_response.choices[0].message.content;
    println!("💬 Assistant response: '{}'", response_content);

    // 8. Serialize final response (what your API would return)
    let final_json = serde_json::to_string_pretty(&openai_response)
        .expect("Failed to serialize OpenAI response");

    println!("📋 Final OpenAI API response:");
    println!("{}", final_json);

    println!("🎉 End-to-end test completed successfully!");
    println!("✅ Complete flow validated: OpenAI Request → Vertex AI → OpenAI Response");
}

/// Test streaming end-to-end flow
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_streaming_e2e() {
    println!("🚀 Starting real Vertex AI streaming end-to-end test...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    // Create streaming request
    let openai_request = ChatCompletionRequest {
        model: config.default_model.model_name.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Count from 1 to 3, one number per response.".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(30),
        temperature: Some(0.1),
        stream: Some(true),
        top_p: None,
        n: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    // Convert to streaming request
    let vertex_request = FormatConverter::openai_to_vertex_streaming(&openai_request)
        .expect("Failed to convert to streaming request");

    println!("📡 Making streaming API call to Vertex AI...");

    // Make streaming call
    let mut stream = vertex_client
        .predict_streaming(&config.default_model.model_name, vertex_request)
        .await
        .expect("Failed to start streaming from Vertex AI");

    let mut chunk_count = 0;
    let mut total_content = String::new();

    // Process streaming chunks
    use tokio_stream::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(vertex_chunk) => {
                chunk_count += 1;
                println!("📦 Received chunk {}: {:?}", chunk_count, vertex_chunk.event_type);

                // Convert to OpenAI format
                if let Ok(Some(openai_chunk)) = FormatConverter::vertex_chunk_to_openai_v2(
                    &vertex_chunk,
                    "chatcmpl-stream-test",
                    &config.default_model.model_name,
                ) {
                    let content = &openai_chunk.choices[0].delta.content;
                    if !content.is_empty() {
                        total_content.push_str(content);
                        println!("💬 Content: '{}'", content);
                    }

                    // Validate chunk structure
                    assert_eq!(openai_chunk.object, "chat.completion.chunk");
                    assert_eq!(openai_chunk.model, config.default_model.model_name);
                }
            }
            Err(e) => {
                eprintln!("❌ Streaming error: {}", e);
                break;
            }
        }
    }

    println!("✅ Streaming completed:");
    println!("  Total chunks: {}", chunk_count);
    println!("  Total content: '{}'", total_content);

    assert!(chunk_count > 0, "Should have received at least one chunk");
    assert!(!total_content.is_empty(), "Should have received some content");

    println!("🎉 Streaming end-to-end test completed successfully!");
}

/// Test error handling with real API
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_error_handling() {
    println!("🚀 Testing error handling with real Vertex AI...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    // Test with invalid model name
    let openai_request = ChatCompletionRequest {
        model: "invalid-model-name".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "This should fail".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(10),
        temperature: None,
        top_p: None,
        n: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
        .expect("Should convert request");

    // This should fail with invalid model
    let result = vertex_client
        .predict("invalid-model-name", vertex_request)
        .await;

    match result {
        Ok(_) => panic!("Expected error with invalid model name"),
        Err(e) => {
            println!("✅ Correctly received error: {}", e);
            assert!(e.to_string().contains("error") || e.to_string().contains("invalid"));
        }
    }

    println!("🎉 Error handling test completed successfully!");
}

/// Test health check functionality
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_health_check() {
    println!("🚀 Testing Vertex AI health check...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    // Test health check
    let is_healthy = vertex_client
        .health_check()
        .await
        .expect("Health check should not fail");

    println!("🏥 Health check result: {}", if is_healthy { "✅ Healthy" } else { "❌ Unhealthy" });

    assert!(is_healthy, "Vertex AI client should be healthy");

    // Test model listing
    let models = vertex_client
        .list_models()
        .await
        .expect("Should be able to list models");

    println!("📋 Available models: {:?}", models);
    assert!(!models.is_empty(), "Should have at least one model available");

    println!("🎉 Health check test completed successfully!");
}

/// Helper function to create Vertex AI config from environment variables
fn create_vertex_config_from_env() -> Result<VertexAiConfig, Box<dyn std::error::Error>> {
    // Required environment variables
    let project_id = env::var("GCP_PROJECT_ID")
        .or_else(|_| env::var("GOOGLE_CLOUD_PROJECT"))
        .or_else(|_| env::var("GCLOUD_PROJECT"))
        .map_err(|_| "GCP_PROJECT_ID environment variable not set")?;

    let region = env::var("GCP_LOCATION")
        .or_else(|_| env::var("GCP_REGION"))
        .unwrap_or_else(|_| "us-east5".to_string());

    let credentials_path = env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .unwrap_or_else(|_| "./gcp-credentials.json".to_string());

    let model_name = env::var("DEFAULT_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-5@20250929".to_string());

    println!("🔧 Environment configuration:");
    println!("  GCP_PROJECT_ID: {}", project_id);
    println!("  GCP_LOCATION: {}", region);
    println!("  GOOGLE_APPLICATION_CREDENTIALS: {}", credentials_path);
    println!("  DEFAULT_MODEL: {}", model_name);

    let config = VertexAiConfig {
        project_id,
        region,
        credentials_path,
        default_model: llm_supabase_rs::config::providers::VertexModelConfig {
            model_name,
            ..Default::default()
        },
        ..Default::default()
    };

    Ok(config)
}

/// Test configuration loading
#[test]
fn test_env_config_loading() {
    println!("🧪 Testing environment configuration loading...");

    // Test with minimal environment
    env::set_var("GCP_PROJECT_ID", "test-project-123");
    env::set_var("GCP_LOCATION", "us-central1");

    let config = create_vertex_config_from_env()
        .expect("Should create config from environment");

    assert_eq!(config.project_id, "test-project-123");
    assert_eq!(config.region, "us-central1");
    assert_eq!(config.default_model.model_name, "claude-sonnet-4-5@20250929");

    println!("✅ Environment configuration test passed!");

    // Clean up
    env::remove_var("GCP_PROJECT_ID");
    env::remove_var("GCP_LOCATION");
}

/// Performance test with real API
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_performance() {
    println!("🚀 Testing Vertex AI performance...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    let openai_request = ChatCompletionRequest {
        model: config.default_model.model_name.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Say 'OK'".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(5),
        temperature: Some(0.1),
        top_p: None,
        n: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
        .expect("Should convert request");

    // Measure performance
    let start = std::time::Instant::now();
    
    let vertex_response = vertex_client
        .predict(&config.default_model.model_name, vertex_request)
        .await
        .expect("Should get response");

    let duration = start.elapsed();

    let openai_response = FormatConverter::vertex_to_openai_v2(
        &vertex_response,
        "chatcmpl-perf-test",
        &config.default_model.model_name,
        &openai_request,
    ).expect("Should convert response");

    println!("⏱️  Performance results:");
    println!("  Total time: {:?}", duration);
    println!("  Response length: {} chars", openai_response.choices[0].message.content.len());
    println!("  Tokens used: {}", openai_response.usage.total_tokens);

    // Validate performance expectations (adjust as needed)
    assert!(duration.as_secs() < 30, "Request should complete within 30 seconds");
    assert!(openai_response.usage.total_tokens > 0, "Should use some tokens");

    println!("🎉 Performance test completed successfully!");
}

/// Test image chat completion with real API
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_image_chat() {
    println!("🚀 Testing image chat completion with Vertex AI...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    // Read and encode the test image
    let image_path = "tests/media/screenshot.png";
    let image_data = std::fs::read(image_path)
        .expect("Failed to read test image");
    let image_base64 = general_purpose::STANDARD.encode(&image_data);

    println!("📷 Loaded test image: {} bytes", image_data.len());

    // Create OpenAI-style request with image
    let openai_request = ChatCompletionRequest {
        model: config.default_model.model_name.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: format!(
                    "I'm sharing a screenshot of some code. Please analyze this image and describe what you see. What programming language is this? What does the code appear to be doing?\n\nImage: data:image/png;base64,{}", 
                    image_base64
                ),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(300),
        temperature: Some(0.3), // Lower temperature for more focused analysis
        top_p: None,
        n: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    println!("📤 Created image chat request:");
    println!("  Model: {}", openai_request.model);
    println!("  Max tokens: {:?}", openai_request.max_tokens);
    println!("  Image size: {} KB", image_data.len() / 1024);

    // Convert to Vertex AI format
    let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
        .expect("Should convert image request to Vertex AI format");

    println!("🔄 Converted to Vertex AI format");

    // Make API call with image
    println!("📡 Making image analysis API call to Vertex AI...");
    let start_time = std::time::Instant::now();

    let vertex_response = vertex_client
        .predict(&config.default_model.model_name, vertex_request)
        .await
        .expect("Should get response for image analysis");

    let duration = start_time.elapsed();

    println!("✅ Received image analysis response:");
    println!("  Response ID: {}", vertex_response.id);
    println!("  Content blocks: {}", vertex_response.content.len());
    println!("  Processing time: {:?}", duration);
    println!("  Usage - Input: {}, Output: {}", 
        vertex_response.usage.input_tokens, 
        vertex_response.usage.output_tokens
    );

    // Convert back to OpenAI format
    let openai_response = FormatConverter::vertex_to_openai_v2(
        &vertex_response,
        "chatcmpl-image-test",
        &config.default_model.model_name,
        &openai_request,
    ).expect("Should convert image response to OpenAI format");

    // Validate response
    assert_eq!(openai_response.object, "chat.completion");
    assert_eq!(openai_response.model, config.default_model.model_name);
    assert_eq!(openai_response.choices.len(), 1);
    assert!(!openai_response.choices[0].message.content.is_empty());

    let analysis = &openai_response.choices[0].message.content;
    println!("🔍 Image Analysis Result:");
    println!("  Length: {} characters", analysis.len());
    println!("  Content preview: {}...", 
        if analysis.len() > 100 { &analysis[..100] } else { analysis }
    );

    // Validate that the response mentions programming/code concepts
    let analysis_lower = analysis.to_lowercase();
    let code_keywords = ["code", "javascript", "typescript", "function", "programming", "variable"];
    let found_keywords: Vec<_> = code_keywords.iter()
        .filter(|&keyword| analysis_lower.contains(keyword))
        .collect();

    assert!(!found_keywords.is_empty(), 
        "Response should mention programming concepts. Found keywords: {:?}", found_keywords);

    println!("✅ Found relevant keywords: {:?}", found_keywords);

    // Validate usage statistics
    assert!(openai_response.usage.prompt_tokens > 0);
    assert!(openai_response.usage.completion_tokens > 0);
    assert_eq!(
        openai_response.usage.total_tokens,
        openai_response.usage.prompt_tokens + openai_response.usage.completion_tokens
    );

    // Image processing should use more tokens due to image content
    assert!(openai_response.usage.prompt_tokens > 100, 
        "Image requests should use significant prompt tokens");

    println!("📊 Token Usage:");
    println!("  Prompt tokens: {}", openai_response.usage.prompt_tokens);
    println!("  Completion tokens: {}", openai_response.usage.completion_tokens);
    println!("  Total tokens: {}", openai_response.usage.total_tokens);

    // Serialize final response
    let _final_json = serde_json::to_string_pretty(&openai_response)
        .expect("Should serialize image response");

    println!("📋 Final image chat response structure validated");

    println!("🎉 Image chat completion test completed successfully!");
    println!("✅ Verified: Image → Vertex AI → Analysis → OpenAI Response");
}

/// Test streaming image chat completion
#[tokio::test]
#[ignore = "requires real GCP credentials and API access"]
async fn test_real_vertex_ai_streaming_image_chat() {
    println!("🚀 Testing streaming image chat completion...");

    dotenv::dotenv().ok();

    let config = create_vertex_config_from_env()
        .expect("Failed to create Vertex AI config from environment");

    let vertex_client = VertexAIClient::new(config.clone())
        .await
        .expect("Failed to initialize Vertex AI client");

    // Read and encode the test image
    let image_path = "tests/media/screenshot.png";
    let image_data = std::fs::read(image_path)
        .expect("Failed to read test image");
    let image_base64 = general_purpose::STANDARD.encode(&image_data);

    // Create streaming request with image
    let openai_request = ChatCompletionRequest {
        model: config.default_model.model_name.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: format!(
                    "Briefly describe what you see in this code screenshot in 2-3 sentences.\n\nImage: data:image/png;base64,{}", 
                    image_base64
                ),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(100),
        temperature: Some(0.3),
        stream: Some(true),
        top_p: None,
        n: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        logprobs: None,
        top_logprobs: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        seed: None,
        metadata: None,
        service_tier: None,
        store: None,
        stream_options: None,
    };

    // Convert to streaming request
    let vertex_request = FormatConverter::openai_to_vertex_streaming(&openai_request)
        .expect("Should convert to streaming image request");

    println!("📡 Making streaming image analysis call...");

    // Make streaming call
    let mut stream = vertex_client
        .predict_streaming(&config.default_model.model_name, vertex_request)
        .await
        .expect("Should start streaming image analysis");

    let mut chunk_count = 0;
    let mut total_content = String::new();

    // Process streaming chunks
    use tokio_stream::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(vertex_chunk) => {
                chunk_count += 1;
                println!("📦 Received chunk {}: {:?}", chunk_count, vertex_chunk.event_type);

                // Convert to OpenAI format
                if let Ok(Some(openai_chunk)) = FormatConverter::vertex_chunk_to_openai_v2(
                    &vertex_chunk,
                    "chatcmpl-stream-image",
                    &config.default_model.model_name,
                ) {
                    let content = &openai_chunk.choices[0].delta.content;
                    if !content.is_empty() {
                        total_content.push_str(content);
                        println!("💬 Content: '{}'", content);
                    }

                    // Validate chunk structure
                    assert_eq!(openai_chunk.object, "chat.completion.chunk");
                    assert_eq!(openai_chunk.model, config.default_model.model_name);
                }
            }
            Err(e) => {
                eprintln!("❌ Streaming error: {}", e);
                break;
            }
        }
    }

    println!("✅ Streaming image analysis completed:");
    println!("  Total chunks: {}", chunk_count);
    println!("  Total content: '{}'", total_content);

    assert!(chunk_count > 0, "Should have received at least one chunk");
    assert!(!total_content.is_empty(), "Should have received image analysis content");

    // Validate that streaming response mentions code/programming
    let content_lower = total_content.to_lowercase();
    let has_code_reference = content_lower.contains("code") || 
                            content_lower.contains("javascript") || 
                            content_lower.contains("typescript") ||
                            content_lower.contains("programming") ||
                            content_lower.contains("function");

    assert!(has_code_reference, 
        "Streaming response should reference code/programming: '{}'", total_content);

    println!("🎉 Streaming image chat test completed successfully!");
}

#[cfg(test)]
mod integration_tests {

    /// Instructions for running these tests
    #[test]
    fn test_instructions() {
        println!("📋 To run the real end-to-end tests:");
        println!();
        println!("1. Set up your .env file with:");
        println!("   GCP_PROJECT_ID=your-project-id");
        println!("   GCP_LOCATION=us-east5");
        println!("   GOOGLE_APPLICATION_CREDENTIALS=./path/to/credentials.json");
        println!("   DEFAULT_MODEL=claude-sonnet-4-5@20250929");
        println!();
        println!("2. Run the tests:");
        println!("   cargo test test_real_vertex_ai_e2e_flow --ignored");
        println!("   cargo test test_real_vertex_ai_streaming_e2e --ignored");
        println!("   cargo test test_real_vertex_ai_health_check --ignored");
        println!("   cargo test test_real_vertex_ai_image_chat --ignored");
        println!("   cargo test test_real_vertex_ai_streaming_image_chat --ignored");
        println!();
        println!("3. For all real tests:");
        println!("   cargo test test_real_vertex_ai --ignored");
        println!();
        println!("✅ Test instructions displayed!");
    }
}
