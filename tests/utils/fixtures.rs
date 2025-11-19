
//! Test fixtures for integration testing across all 8 providers
//! 
//! This module provides predefined test data, sample files, and common test scenarios
//! used across different integration test suites for all supported providers.

use serde_json::{json, Value};
use std::collections::HashMap;

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition},
};

/// Provider-specific test configurations
#[derive(Debug, Clone)]
pub struct ProviderFixture {
    pub name: String,
    pub default_model: String,
    pub alternative_models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub expected_response_patterns: Vec<String>,
    pub typical_response_time_ms: u64,
}

/// Performance benchmark expectations
#[derive(Debug, Clone)]
pub struct PerformanceBenchmark {
    pub avg_response_time_ms: u64,
    pub max_acceptable_time_ms: u64,
    pub expected_success_rate: f64,
    pub tokens_per_second: u32,
}

/// Performance complexity levels for testing
#[derive(Debug, Clone, Copy)]
pub enum PerformanceComplexity {
    Simple,
    Medium,
    Complex,
}

/// Provider-aware test fixtures
pub struct TestFixtures;

impl TestFixtures {
    /// Get all provider configurations for testing
    pub fn all_provider_fixtures() -> HashMap<String, ProviderFixture> {
        let mut fixtures = HashMap::new();
        
        fixtures.insert("openai".to_string(), ProviderFixture {
            name: "openai".to_string(),
            default_model: "gpt-4".to_string(),
            alternative_models: vec!["gpt-3.5-turbo".to_string(), "gpt-4-turbo".to_string()],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "I'll help you".to_string(),
                "Let me".to_string(),
                "Here's".to_string(),
            ],
            typical_response_time_ms: 2000,
        });

        fixtures.insert("anthropic".to_string(), ProviderFixture {
            name: "anthropic".to_string(),
            default_model: "claude-3-sonnet-20240229".to_string(),
            alternative_models: vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "I'll analyze".to_string(),
                "Let me break down".to_string(),
                "I'd be happy to".to_string(),
            ],
            typical_response_time_ms: 2500,
        });

        fixtures.insert("vertex".to_string(), ProviderFixture {
            name: "vertex".to_string(),
            default_model: "gemini-pro".to_string(),
            alternative_models: vec![
                "claude-3-sonnet@20240229".to_string(),
                "gemini-1.5-pro".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "I can help".to_string(),
                "Using Google's".to_string(),
                "Let me assist".to_string(),
            ],
            typical_response_time_ms: 2200,
        });

        fixtures.insert("groq".to_string(), ProviderFixture {
            name: "groq".to_string(),
            default_model: "mixtral-8x7b-32768".to_string(),
            alternative_models: vec![
                "llama2-70b-4096".to_string(),
                "gemma-7b-it".to_string(),
            ],
            supports_streaming: true,
            supports_tools: false, // Limited tool support
            expected_response_patterns: vec![
                "Fast response".to_string(),
                "Quickly".to_string(),
                "⚡".to_string(),
            ],
            typical_response_time_ms: 500, // Very fast
        });

        fixtures.insert("azure_openai".to_string(), ProviderFixture {
            name: "azure_openai".to_string(),
            default_model: "gpt-4".to_string(),
            alternative_models: vec!["gpt-35-turbo".to_string()],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "Azure OpenAI".to_string(),
                "Enterprise-grade".to_string(),
                "Secure".to_string(),
            ],
            typical_response_time_ms: 2100,
        });

        fixtures.insert("aws_bedrock".to_string(), ProviderFixture {
            name: "aws_bedrock".to_string(),
            default_model: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            alternative_models: vec![
                "meta.llama2-70b-chat-v1".to_string(),
                "anthropic.claude-v2".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "AWS Bedrock".to_string(),
                "Secure and scalable".to_string(),
                "Amazon's infrastructure".to_string(),
            ],
            typical_response_time_ms: 2800,
        });

        fixtures.insert("cohere".to_string(), ProviderFixture {
            name: "cohere".to_string(),
            default_model: "command".to_string(),
            alternative_models: vec![
                "command-light".to_string(),
                "command-nightly".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "Cohere model".to_string(),
                "Enterprise AI".to_string(),
                "Advanced language".to_string(),
            ],
            typical_response_time_ms: 2300,
        });

        fixtures.insert("mistral".to_string(), ProviderFixture {
            name: "mistral".to_string(),
            default_model: "mistral-large-latest".to_string(),
            alternative_models: vec![
                "mistral-medium-latest".to_string(),
                "mistral-small-latest".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
            expected_response_patterns: vec![
                "Mistral AI".to_string(),
                "European-compliant".to_string(),
                "Cost-effective".to_string(),
            ],
            typical_response_time_ms: 1800,
        });

        fixtures
    }

    /// Create a basic Codex CLI request for a specific provider
    pub fn codex_request_for_provider(provider_name: &str) -> ChatCompletionRequest {
        let fixtures = Self::all_provider_fixtures();
        let provider = fixtures.get(provider_name).unwrap();
        
        ChatCompletionRequest::new(
            &provider.default_model,
            vec![ChatMessage::user("Add a new function to calculate the factorial of a number")]
        )
        .with_max_tokens(4096)
        .with_temperature(0.7)
        .with_tools(if provider.supports_tools { Self::codex_tools() } else { vec![] })
    }

    /// Create a basic Codex CLI request with default model
    pub fn codex_basic_request() -> ChatCompletionRequest {
        Self::codex_request_for_provider("anthropic")
    }

    /// Create multi-turn conversation requests for all providers
    pub fn multi_turn_requests_for_all_providers() -> HashMap<String, ChatCompletionRequest> {
        let fixtures = Self::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            let messages = vec![
                ChatMessage::system("You are a helpful coding assistant."),
                ChatMessage::user("Add a new function to main.rs"),
                ChatMessage::assistant_with_content("I'll help you add a new function. Let me create that for you."),
                ChatMessage::user("Now add tests for that function"),
            ];

            let mut request = ChatCompletionRequest::new(&fixture.default_model, messages)
                .with_max_tokens(4096)
                .with_temperature(0.7);
                
            if fixture.supports_tools {
                request = request.with_tools(Self::codex_tools());
            }
            
            requests.insert(provider_name, request);
        }
        
        requests
    }

    /// Create streaming requests for providers that support it
    pub fn streaming_requests_for_all_providers() -> HashMap<String, ChatCompletionRequest> {
        let fixtures = Self::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            if fixture.supports_streaming {
                let request = ChatCompletionRequest::new(
                    &fixture.default_model,
                    vec![ChatMessage::user("Explain how to implement a binary search algorithm step by step")]
                )
                .streaming(true)
                .with_max_tokens(2048);
                
                requests.insert(provider_name, request);
            }
        }
        
        requests
    }

    /// Create tool calling requests for providers that support tools
    pub fn tool_calling_requests_for_all_providers() -> HashMap<String, ChatCompletionRequest> {
        let fixtures = Self::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            if fixture.supports_tools {
                let request = ChatCompletionRequest::new(
                    &fixture.default_model,
                    vec![ChatMessage::user("Read the main.rs file and add error handling to it")]
                )
                .with_tools(Self::codex_tools())
                .with_max_tokens(4096);
                
                requests.insert(provider_name, request);
            }
        }
        
        requests
    }

    /// Create provider-specific performance test requests
    pub fn performance_requests_for_all_providers(complexity_level: PerformanceComplexity) -> HashMap<String, ChatCompletionRequest> {
        let fixtures = Self::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        let (message, max_tokens) = match complexity_level {
            PerformanceComplexity::Simple => (
                "Write a hello world function",
                1024
            ),
            PerformanceComplexity::Medium => (
                "Implement a binary search algorithm with comprehensive error handling and documentation",
                2048
            ),
            PerformanceComplexity::Complex => (
                "Design and implement a complete REST API with authentication, rate limiting, database integration, and comprehensive testing",
                4096
            ),
        };
        
        for (provider_name, fixture) in fixtures {
            let mut request = ChatCompletionRequest::new(
                &fixture.default_model,
                vec![ChatMessage::user(message)]
            )
            .with_max_tokens(max_tokens)
            .with_temperature(0.3); // Lower temperature for consistent performance testing
            
            if fixture.supports_tools {
                request = request.with_tools(Self::codex_tools());
            }
            
            requests.insert(provider_name, request);
        }
        
        requests
    }

    /// Get standard Codex CLI tools
    pub fn codex_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "apply_patch".to_string(),
                    description: Some("Apply a unified diff patch to a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to patch"
                            },
                            "patch": {
                                "type": "string",
                                "description": "Unified diff patch content"
                            }
                        },
                        "required": ["file_path", "patch"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "read_file".to_string(),
                    description: Some("Read the contents of a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to read"
                            }
                        },
                        "required": ["file_path"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "search_files".to_string(),
                    description: Some("Search for patterns in files".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "pattern": {
                                "type": "string",
                                "description": "Pattern to search for"
                            },
                            "directory": {
                                "type": "string", 
                                "description": "Directory to search in"
                            }
                        },
                        "required": ["pattern"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "write_file".to_string(),
                    description: Some("Write content to a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write to the file"
                            }
                        },
                        "required": ["file_path", "content"]
                    }),
                },
            },
        ]
    }

    /// Get provider-specific model variations for testing
    pub fn get_model_variations_for_provider(provider_name: &str) -> Vec<String> {
        let fixtures = Self::all_provider_fixtures();
        if let Some(fixture) = fixtures.get(provider_name) {
            let mut models = vec![fixture.default_model.clone()];
            models.extend(fixture.alternative_models.clone());
            models
        } else {
            vec![]
        }
    }
}

/// Sample file contents for testing diff/patch operations
pub struct SampleFiles;

impl SampleFiles {
    /// Sample main.rs content
    pub fn main_rs() -> &'static str {
        r#"// main.rs
fn main() {
    println!("Hello, world!");
}

fn helper_function(x: i32) -> i32 {
    x * 2
}
"#
    }

    /// Sample lib.rs content
    pub fn lib_rs() -> &'static str {
        r#"//! Sample library crate

pub mod utils;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#
    }

    /// Sample utils.rs content
    pub fn utils_rs() -> &'static str {
        r#"//! Utility functions

pub fn format_number(n: i32) -> String {
    format!("Number: {}", n)
}

pub fn calculate_sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}
"#
    }

    /// Sample Cargo.toml content
    pub fn cargo_toml() -> &'static str {
        r#"[package]
name = "sample-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
"#
    }

    /// Provider-specific sample code for testing
    pub fn provider_specific_sample(provider_name: &str) -> &'static str {
        match provider_name {
            "openai" => r#"// OpenAI integration example
use openai_api_rs::*;

async fn query_openai() -> Result<String, Box<dyn std::error::Error>> {
    // OpenAI-specific implementation
    Ok("OpenAI response".to_string())
}
"#,
            "anthropic" => r#"// Anthropic Claude integration example
use anthropic::*;

async fn query_claude() -> Result<String, Box<dyn std::error::Error>> {
    // Claude-specific implementation
    Ok("Claude response".to_string())
}
"#,
            "vertex" => r#"// Google Vertex AI integration example
use google_cloud_vertex::*;

async fn query_vertex() -> Result<String, Box<dyn std::error::Error>> {
    // Vertex AI-specific implementation
    Ok("Vertex AI response".to_string())
}
"#,
            _ => Self::main_rs(),
        }
    }
}

/// Sample unified diff patches
pub struct SamplePatches;

impl SamplePatches {
    /// Patch to add a new function to main.rs
    pub fn add_function_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,7 +1,11 @@
 // main.rs
+fn factorial(n: u64) -> u64 {
+    match n {
+        0 | 1 => 1,
+        _ => n * factorial(n - 1),
+    }
+}
+
 fn main() {
     println!("Hello, world!");
+    println!("5! = {}", factorial(5));
 }
 
 fn helper_function(x: i32) -> i32 {
"#
    }

    /// Provider-specific patches for testing
    pub fn provider_specific_patch(provider_name: &str) -> &'static str {
        match provider_name {
            "groq" => Self::add_fast_function_patch(),
            "vertex" => Self::add_vertex_specific_patch(),
            "aws_bedrock" => Self::add_aws_specific_patch(),
            _ => Self::add_function_patch(),
        }
    }

    fn add_fast_function_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
 // main.rs
+fn fast_compute(n: i32) -> i32 {
+    // Optimized for Groq's fast inference
+    n << 1  // Fast bit shift
+}
+
 fn main() {
     println!("Hello, world!");
"#
    }

    fn add_vertex_specific_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
 // main.rs
+fn vertex_ai_integration() {
+    // Google Vertex AI specific functionality
+    println!("Connected to Vertex AI");
+}
+
 fn main() {
     println!("Hello, world!");
"#
    }

    fn add_aws_specific_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
 // main.rs
+fn aws_bedrock_integration() {
+    // AWS Bedrock specific functionality
+    println!("Connected to AWS Bedrock");
+}
+
 fn main() {
     println!("Hello, world!");
"#
    }

    /// Patch to add error handling
    pub fn add_error_handling_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
+use std::error::Error;
+use std::io;

 // main.rs
-fn main() {
+fn main() -> Result<(), Box<dyn Error>> {
     println!("Hello, world!");
+    Ok(())
 }
"#
    }

    /// Patch to add tests
    pub fn add_tests_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -7,3 +7,15 @@ fn main() {
 fn helper_function(x: i32) -> i32 {
     x * 2
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn test_helper_function() {
+        assert_eq!(helper_function(5), 10);
+        assert_eq!(helper_function(0), 0);
+        assert_eq!(helper_function(-3), -6);
+    }
+}
"#
    }
}

/// Provider-aware conversation scenarios
pub struct ConversationScenarios;

impl ConversationScenarios {
    /// Basic code generation conversation
    pub fn code_generation() -> Vec<ChatMessage> {
        vec![
            ChatMessage::user("Create a function to validate email addresses"),
            ChatMessage::assistant_with_content("I'll create an email validation function for you."),
            ChatMessage::user("Now add unit tests for that function"),
            ChatMessage::assistant_with_content("I'll add comprehensive unit tests for the email validation function."),
        ]
    }

    /// Provider-specific conversation scenarios
    pub fn provider_specific_conversation(provider_name: &str) -> Vec<ChatMessage> {
        match provider_name {
            "openai" => vec![
                ChatMessage::user("Use OpenAI's best practices to optimize this code"),
                ChatMessage::assistant_with_content("I'll apply OpenAI's recommended patterns for code optimization."),
            ],
            "anthropic" => vec![
                ChatMessage::user("Analyze this code structure thoughtfully"),
                ChatMessage::assistant_with_content("I'll provide a thorough analysis of the code structure and architecture."),
            ],
            "groq" => vec![
                ChatMessage::user("Quickly optimize this for performance"),
                ChatMessage::assistant_with_content("⚡ I'll rapidly optimize this code for maximum performance."),
            ],
            "vertex" => vec![
                ChatMessage::user("Integrate this with Google Cloud services"),
                ChatMessage::assistant_with_content("I'll help integrate this with Google Cloud's AI and infrastructure services."),
            ],
            _ => Self::code_generation(),
        }
    }

    /// Debugging conversation
    pub fn debugging_session() -> Vec<ChatMessage> {
        vec![
            ChatMessage::user("I'm getting a compilation error in my Rust code"),
            ChatMessage::assistant_with_content("I'll help you fix the compilation error. Can you show me the code?"),
            ChatMessage::user("Here's the error: cannot borrow as mutable"),
            ChatMessage::assistant_with_content("This is a common borrowing issue. Let me help you fix it."),
        ]
    }

    /// Refactoring conversation
    pub fn refactoring_session() -> Vec<ChatMessage> {
        vec![
            ChatMessage::user("Can you help me refactor this function to be more efficient?"),
            ChatMessage::assistant_with_content("I'll analyze your function and suggest improvements."),
            ChatMessage::user("Also make it more readable"),
            ChatMessage::assistant_with_content("I'll refactor it for both efficiency and readability."),
        ]
    }
}

/// Error scenario fixtures for all providers
pub struct ErrorScenarios;

impl ErrorScenarios {
    /// Invalid request scenarios
    pub fn invalid_requests() -> Vec<ChatCompletionRequest> {
        vec![
            // Empty messages
            ChatCompletionRequest::new("gpt-4", vec![]),
            
            // Invalid temperature
            ChatCompletionRequest::new(
                "gpt-4", 
                vec![ChatMessage::user("test")]
            ).with_temperature(-1.0),
            
            // Invalid max_tokens
            ChatCompletionRequest::new(
                "gpt-4",
                vec![ChatMessage::user("test")]
            ).with_max_tokens(0),
        ]
    }

    /// Provider-specific failure scenarios
    pub fn provider_failures() -> HashMap<String, Vec<(&'static str, &'static str)>> {
        let mut failures = HashMap::new();
        
        // Common failures for all providers
        let common_failures = vec![
            ("auth_error", "Authentication failed"),
            ("rate_limit", "Rate limit exceeded"),
            ("service_unavailable", "Service temporarily unavailable"),
            ("timeout", "Request timeout"),
        ];
        
        // OpenAI specific failures
        failures.insert("openai".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("invalid_model", "Model not found"),
                ("quota_exceeded", "Quota exceeded"),
            ]);
            f
        });
        
        // Anthropic specific failures
        failures.insert("anthropic".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("invalid_request", "Invalid request format"),
                ("message_too_long", "Message exceeds length limit"),
            ]);
            f
        });
        
        // Vertex AI specific failures
        failures.insert("vertex".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("project_not_found", "GCP project not found"),
                ("region_not_available", "Region not available"),
            ]);
            f
        });
        
        // Add similar patterns for other providers
        for provider in &["groq", "azure_openai", "aws_bedrock", "cohere", "mistral"] {
            failures.insert(provider.to_string(), common_failures.clone());
        }
        
        failures
    }

    /// Get expected error patterns for each provider
    pub fn expected_error_patterns() -> HashMap<String, Vec<String>> {
        let mut patterns = HashMap::new();
        
        patterns.insert("openai".to_string(), vec![
            "OpenAI error:".to_string(),
            "Invalid API key".to_string(),
            "Rate limit".to_string(),
        ]);
        
        patterns.insert("anthropic".to_string(), vec![
            "Anthropic error:".to_string(),
            "Authentication failed".to_string(),
            "Invalid request".to_string(),
        ]);
        
        patterns.insert("vertex".to_string(), vec![
            "Vertex AI error:".to_string(),
            "Invalid credentials".to_string(),
            "Quota exceeded".to_string(),
        ]);
        
        patterns.insert("groq".to_string(), vec![
            "Groq error:".to_string(),
            "Service unavailable".to_string(),
        ]);
        
        // Add patterns for other providers
        for provider in &["azure_openai", "aws_bedrock", "cohere", "mistral"] {
            patterns.insert(provider.to_string(), vec![
                format!("{} error:", provider),
                "Service error".to_string(),
            ]);
        }
        
        patterns
    }
}

/// Performance test fixtures for all providers
pub struct PerformanceFixtures;

impl PerformanceFixtures {
    /// Generate large request for performance testing
    pub fn large_request_for_provider(provider_name: &str, message_count: usize) -> ChatCompletionRequest {
        let fixtures = TestFixtures::all_provider_fixtures();
        let provider = fixtures.get(provider_name).unwrap();
        
        let messages: Vec<ChatMessage> = (0..message_count)
            .map(|i| {
                if i % 2 == 0 {
                    ChatMessage::user(&format!("User message number {} for {}", i, provider_name))
                } else {
                    ChatMessage::assistant_with_content(&format!("Assistant response number {} from {}", i, provider_name))
                }
            })
            .collect();

        ChatCompletionRequest::new(&provider.default_model, messages)
            .with_max_tokens(4096)
    }

    /// Generate concurrent requests for load testing across providers
    pub fn concurrent_requests_for_all_providers(count: usize) -> HashMap<String, Vec<ChatCompletionRequest>> {
        let fixtures = TestFixtures::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            let provider_requests: Vec<ChatCompletionRequest> = (0..count)
                .map(|i| {
                    ChatCompletionRequest::new(
                        &fixture.default_model,
                        vec![ChatMessage::user(&format!("Concurrent request {} for {}", i, provider_name))]
                    ).with_max_tokens(1024)
                })
                .collect();
                
            requests.insert(provider_name, provider_requests);
        }
        
        requests
    }

    /// Generate streaming requests for providers that support it
    pub fn streaming_requests_for_all_providers(count: usize) -> HashMap<String, Vec<ChatCompletionRequest>> {
        let fixtures = TestFixtures::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            if fixture.supports_streaming {
                let provider_requests: Vec<ChatCompletionRequest> = (0..count)
                    .map(|i| {
                        ChatCompletionRequest::new(
                            &fixture.default_model,
                            vec![ChatMessage::user(&format!("Streaming request {}: explain {} algorithms", i, provider_name))]
                        )
                        .streaming(true)
                        .with_max_tokens(2048)
                    })
                    .collect();
                    
                requests.insert(provider_name, provider_requests);
            }
        }
        
        requests
    }

    /// Get expected performance benchmarks for each provider
    pub fn expected_performance_benchmarks() -> HashMap<String, PerformanceBenchmark> {
        let mut benchmarks = HashMap::new();
        
        benchmarks.insert("openai".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2000,
            max_acceptable_time_ms: 10000,
            expected_success_rate: 0.95,
            tokens_per_second: 50,
        });
        
        benchmarks.insert("anthropic".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2500,
            max_acceptable_time_ms: 12000,
            expected_success_rate: 0.94,
            tokens_per_second: 45,
        });
        
        benchmarks.insert("vertex".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2200,
            max_acceptable_time_ms: 11000,
            expected_success_rate: 0.93,
            tokens_per_
                "gpt-4", 
                vec![ChatMessage::user("test")]
            ).with_temperature(-1.0),
            
            // Invalid max_tokens
            ChatCompletionRequest::new(
                "gpt-4",
                vec![ChatMessage::user("test")]
            ).with_max_tokens(0),
        ]
    }

    /// Provider-specific failure scenarios
    pub fn provider_failures() -> HashMap<String, Vec<(&'static str, &'static str)>> {
        let mut failures = HashMap::new();
        
        // Common failures for all providers
        let common_failures = vec![
            ("auth_error", "Authentication failed"),
            ("rate_limit", "Rate limit exceeded"),
            ("service_unavailable", "Service temporarily unavailable"),
            ("timeout", "Request timeout"),
        ];
        
        // OpenAI specific failures
        failures.insert("openai".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("invalid_model", "Model not found"),
                ("quota_exceeded", "Quota exceeded"),
            ]);
            f
        });
        
        // Anthropic specific failures
        failures.insert("anthropic".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("invalid_request", "Invalid request format"),
                ("message_too_long", "Message exceeds length limit"),
            ]);
            f
        });
        
        // Vertex AI specific failures
        failures.insert("vertex".to_string(), {
            let mut f = common_failures.clone();
            f.extend(vec![
                ("project_not_found", "GCP project not found"),
                ("region_not_available", "Region not available"),
            ]);
            f
        });
        
        // Add similar patterns for other providers
        for provider in &["groq", "azure_openai", "aws_bedrock", "cohere", "mistral"] {
            failures.insert(provider.to_string(), common_failures.clone());
        }
        
        failures
    }

    /// Get expected error patterns for each provider
    pub fn expected_error_patterns() -> HashMap<String, Vec<String>> {
        let mut patterns = HashMap::new();
        
        patterns.insert("openai".to_string(), vec![
            "OpenAI error:".to_string(),
            "Invalid API key".to_string(),
            "Rate limit".to_string(),
        ]);
        
        patterns.insert("anthropic".to_string(), vec![
            "Anthropic error:".to_string(),
            "Authentication failed".to_string(),
            "Invalid request".to_string(),
        ]);
        
        patterns.insert("vertex".to_string(), vec![
            "Vertex AI error:".to_string(),
            "Invalid credentials".to_string(),
            "Quota exceeded".to_string(),
        ]);
        
        patterns.insert("groq".to_string(), vec![
            "Groq error:".to_string(),
            "Service unavailable".to_string(),
        ]);
        
        // Add patterns for other providers
        for provider in &["azure_openai", "aws_bedrock", "cohere", "mistral"] {
            patterns.insert(provider.to_string(), vec![
                format!("{} error:", provider),
                "Service error".to_string(),
            ]);
        }
        
        patterns
    }
}

/// Performance test fixtures for all providers
pub struct PerformanceFixtures;

impl PerformanceFixtures {
    /// Generate large request for performance testing
    pub fn large_request_for_provider(provider_name: &str, message_count: usize) -> ChatCompletionRequest {
        let fixtures = TestFixtures::all_provider_fixtures();
        let provider = fixtures.get(provider_name).unwrap();
        
        let messages: Vec<ChatMessage> = (0..message_count)
            .map(|i| {
                if i % 2 == 0 {
                    ChatMessage::user(&format!("User message number {} for {}", i, provider_name))
                } else {
                    ChatMessage::assistant_with_content(&format!("Assistant response number {} from {}", i, provider_name))
                }
            })
            .collect();

        ChatCompletionRequest::new(&provider.default_model, messages)
            .with_max_tokens(4096)
    }

    /// Generate concurrent requests for load testing across providers
    pub fn concurrent_requests_for_all_providers(count: usize) -> HashMap<String, Vec<ChatCompletionRequest>> {
        let fixtures = TestFixtures::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            let provider_requests: Vec<ChatCompletionRequest> = (0..count)
                .map(|i| {
                    ChatCompletionRequest::new(
                        &fixture.default_model,
                        vec![ChatMessage::user(&format!("Concurrent request {} for {}", i, provider_name))]
                    ).with_max_tokens(1024)
                })
                .collect();
                
            requests.insert(provider_name, provider_requests);
        }
        
        requests
    }

    /// Generate streaming requests for providers that support it
    pub fn streaming_requests_for_all_providers(count: usize) -> HashMap<String, Vec<ChatCompletionRequest>> {
        let fixtures = TestFixtures::all_provider_fixtures();
        let mut requests = HashMap::new();
        
        for (provider_name, fixture) in fixtures {
            if fixture.supports_streaming {
                let provider_requests: Vec<ChatCompletionRequest> = (0..count)
                    .map(|i| {
                        ChatCompletionRequest::new(
                            &fixture.default_model,
                            vec![ChatMessage::user(&format!("Streaming request {}: explain {} algorithms", i, provider_name))]
                        )
                        .streaming(true)
                        .with_max_tokens(2048)
                    })
                    .collect();
                    
                requests.insert(provider_name, provider_requests);
            }
        }
        
        requests
    }

    /// Get expected performance benchmarks for each provider
    pub fn expected_performance_benchmarks() -> HashMap<String, PerformanceBenchmark> {
        let mut benchmarks = HashMap::new();
        
        benchmarks.insert("openai".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2000,
            max_acceptable_time_ms: 10000,
            expected_success_rate: 0.95,
            tokens_per_second: 50,
        });
        
        benchmarks.insert("anthropic".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2500,
            max_acceptable_time_ms: 12000,
            expected_success_rate: 0.94,
            tokens_per_second: 45,
        });
        
        benchmarks.insert("vertex".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2200,
            max_acceptable_time_ms: 11000,
            expected_success_rate: 0.93,
            tokens_per_second: 48,
        });
        
        benchmarks.insert("groq".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 500, // Very fast
            max_acceptable_time_ms: 3000,
            expected_success_rate: 0.90, // May have more variability
            tokens_per_second: 120, // Fastest
        });
        
        benchmarks.insert("azure_openai".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2100,
            max_acceptable_time_ms: 10500,
            expected_success_rate: 0.95,
            tokens_per_second: 52,
        });
        
        benchmarks.insert("aws_bedrock".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2800,
            max_acceptable_time_ms: 15000,
            expected_success_rate: 0.92,
            tokens_per_second: 40,
        });
        
        benchmarks.insert("cohere".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 2300,
            max_acceptable_time_ms: 12000,
            expected_success_rate: 0.93,
            tokens_per_second: 46,
        });
        
        benchmarks.insert("mistral".to_string(), PerformanceBenchmark {
            avg_response_time_ms: 1800,
            max_acceptable_time_ms: 9000,
            expected_success_rate: 0.94,
            tokens_per_second: 55, // Good performance
        });
        
        benchmarks
    }
}

/// Helper extensions for ChatMessage
pub trait ChatMessageExt {
    fn user(content: &str) -> ChatMessage;
    fn system(content: &str) -> ChatMessage;
    fn assistant_with_content(content: &str) -> ChatMessage;
    fn tool_result(call_id: &str, result: &str) -> ChatMessage;
}

impl ChatMessageExt for ChatMessage {
    fn user(content: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::User,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn system(content: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::System,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn assistant_with_content(content: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::Assistant,
            content: Some(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn tool_result(call_id: &str, result: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::Tool,
            content: result.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(call_id.to_string()),
        }
    }
}

/// Cross-provider test utilities
pub struct CrossProviderTestUtils;

impl CrossProviderTestUtils {
    /// Get providers that support a specific feature
    pub fn get_providers_with_streaming() -> Vec<String> {
        TestFixtures::all_provider_fixtures()
            .into_iter()
            .filter(|(_, fixture)| fixture.supports_streaming)
            .map(|(name, _)| name)
            .collect()
    }

    pub fn get_providers_with_tools() -> Vec<String> {
        TestFixtures::all_provider_fixtures()
            .into_iter()
            .filter(|(_, fixture)| fixture.supports_tools)
            .map(|(name, _)| name)
            .collect()
    }

    /// Get providers sorted by expected response time
    pub fn get_providers_by_speed() -> Vec<(String, u64)> {
        let fixtures = TestFixtures::all_provider_fixtures();
        let mut providers: Vec<(String, u64)> = fixtures
            .into_iter()
            .map(|(name, fixture)| (name, fixture.typical_response_time_ms))
            .collect();
        
        providers.sort_by_key(|(_, time)| *time);
        providers
    }

    /// Create fallback chain for testing
    pub fn get_fallback_chain() -> Vec<String> {
        vec![
            "openai".to_string(),
            "anthropic".to_string(),
            "vertex".to_string(),
            "azure_openai".to_string(),
            "mistral".to_string(),
            "cohere".to_string(),
            "aws_bedrock".to_string(),
            "groq".to_string(), // Last because limited tool support
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_provider_fixtures() {
        let fixtures = TestFixtures::all_provider_fixtures();
        assert_eq!(fixtures.len(), 8);
        
        // Check that all expected providers are present
        let expected_providers = vec![
            "openai", "anthropic", "vertex", "groq",
            "azure_openai", "aws_bedrock", "cohere", "mistral"
        ];
        
        for provider in expected_providers {
            assert!(fixtures.contains_key(provider), "Missing provider: {}", provider);
        }
    }

    #[test]
    fn test_provider_capabilities() {
        let fixtures = TestFixtures::all_provider_fixtures();
        
        // Test that most providers support streaming
        let streaming_count = fixtures.values().filter(|f| f.supports_streaming).count();
        assert!(streaming_count >= 7, "Most providers should support streaming");
        
        // Test that Groq has limited tool support
        let groq = fixtures.get("groq").unwrap();
        assert_eq!(groq.supports_tools, false, "Groq should have limited tool support");
        
        // Test that most others support tools
        let tool_count = fixtures.values().filter(|f| f.supports_tools).count();
        assert!(tool_count >= 6, "Most providers should support tools");
    }

    #[test]
    fn test_performance_benchmarks() {
        let benchmarks = PerformanceFixtures::expected_performance_benchmarks();
        assert_eq!(benchmarks.len(), 8);
        
        // Test that Groq is fastest
        let groq_benchmark = benchmarks.get("groq").unwrap();
        assert!(groq_benchmark.avg_response_time_ms < 1000, "Groq should be very fast");
        
        // Test that all benchmarks have reasonable values
        for (provider, benchmark) in benchmarks {
            assert!(benchmark.avg_response_time_ms > 0, "{} should have positive response time", provider);
            assert!(benchmark.expected_success_rate > 0.8, "{} should have high success rate", provider);
            assert!(benchmark.tokens_per_second > 0, "{} should have positive token rate", provider);
        }
    }

    #[test]
    fn test_multi_turn_requests() {
        let requests = TestFixtures::multi_turn_requests_for_all_providers();
        assert!(!requests.is_empty());
        
        for (provider_name, request) in requests {
            assert!(!request.messages.is_empty(), "{} should have messages", provider_name);
            assert!(request.messages.len() >= 4, "{} should have multi-turn conversation", provider_name);
        }
    }

    #[test]
    fn test_streaming_requests() {
        let requests = TestFixtures::streaming_requests_for_all_providers();
        
        for (provider_name, request) in requests {
            assert!(request.stream.unwrap_or(false), "{} request should be streaming", provider_name);
        }
    }

    #[test]
    fn test_tool_calling_requests() {
        let requests = TestFixtures::tool_calling_requests_for_all_providers();
        
        for (provider_name, request) in requests {
            assert!(request.tools.is_some(), "{} should have tools", provider_name);
            assert!(!request.tools.as_ref().unwrap().is_empty(), "{} should have non-empty tools", provider_name);
        }
    }

    #[test]
    fn test_cross_provider_utils() {
        let streaming_providers = CrossProviderTestUtils::get_providers_with_streaming();
        let tool_providers = CrossProviderTestUtils::get_providers_with_tools();
        let speed_ranking = CrossProviderTestUtils::get_providers_by_speed();
        
        assert!(streaming_providers.len() >= 7, "Most providers should support streaming");
        assert!(tool_providers.len() >= 6, "Most providers should support tools");
        assert_eq!(speed_ranking.len(), 8, "Should have all providers in speed ranking");
        
        // Groq should be fastest
        assert_eq!(speed_ranking[0].0, "groq", "Groq should be fastest provider");
    }

    #[test]
    fn test_error_scenarios() {
        let invalid_requests = ErrorScenarios::invalid_requests();
        assert!(!invalid_requests.is_empty(), "Should have invalid request scenarios");
        
        let provider_failures = ErrorScenarios::provider_failures();
        assert_eq!(provider_failures.len(), 8, "Should have failures for all providers");
        
        let error_patterns = ErrorScenarios::expected_error_patterns();
        assert_eq!(error_patterns.len(), 8, "Should have error patterns for all providers");
    }

    #[test]
    fn test_performance_fixtures() {
        let concurrent_requests = PerformanceFixtures::concurrent_requests_for_all_providers(3);
        assert_eq!(concurrent_requests.len(), 8, "Should have requests for all providers");
        
        for (provider_name, requests) in concurrent_requests {
            assert_eq!(requests.len(), 3, "{} should have 3 concurrent requests", provider_name);
        }
        
        let streaming_requests = PerformanceFixtures::streaming_requests_for_all_providers(2);
        // Should only include providers that support streaming
        assert!(streaming_requests.len() >= 7, "Should have streaming requests for most providers");
    }

    #[test]
    fn test_sample_files() {
        let main_content = SampleFiles::main_rs();
        assert!(main_content.contains("fn main()"), "Should have main function");
        
        let lib_content = SampleFiles::lib_rs();
        assert!(lib_content.contains("pub fn add"), "Should have add function");
        
        let openai_sample = SampleFiles::provider_specific_sample("openai");
        assert!(openai_sample.contains("openai"), "Should have OpenAI-specific content");
    }

    #[test]
    fn test_conversation_scenarios() {
        let basic_conversation = ConversationScenarios::code_generation();
        assert!(basic_conversation.len() >= 2, "Should have multi-turn conversation");
        
        let provider_conversation = ConversationScenarios::provider_specific_conversation("anthropic");
        assert!(!provider_conversation.is_empty(), "Should have provider-specific conversation");
    }

    #[test]
    fn test_chat_message_extensions() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.content, "Hello");
        
        let system_msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(system_msg.role, MessageRole::System);
        
        let assistant_msg = ChatMessage::assistant_with_content("I'm here to help");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
        assert!(assistant_msg.content.is_some());
        
        let tool_msg = ChatMessage::tool_result("call_123", "Tool result");
        assert_eq!(tool_msg.role, MessageRole::Tool);
        assert_eq!(tool_msg.tool_call_id.as_ref().unwrap(), "call_123");
    }
}