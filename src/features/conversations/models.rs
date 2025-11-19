//! Conversation data models
//!
//! Defines the core data structures for conversation state management,
//! including conversation state, metadata, and message history tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::common::ChatMessage;

/// Conversation state containing message history and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// Unique conversation identifier
    pub conversation_id: String,
    
    /// Complete message history for this conversation
    pub messages: Vec<ChatMessage>,
    
    /// ID of the last response in this conversation
    pub last_response_id: Option<String>,
    
    /// Metadata about the conversation
    pub metadata: ConversationMetadata,
    
    /// When this conversation was created
    pub created_at: DateTime<Utc>,
    
    /// When this conversation was last updated
    pub updated_at: DateTime<Utc>,
    
    /// When this conversation should expire (for cleanup)
    pub expires_at: DateTime<Utc>,
    
    /// Model used in this conversation
    pub model: String,
    
    /// Optional user identifier
    pub user_id: Option<String>,
    
    /// Total number of turns in this conversation
    pub turn_count: u32,
    
    /// Whether this conversation is active
    pub active: bool,
}

/// Conversation metadata for tracking and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    /// Total tokens used in this conversation
    pub total_tokens: u32,
    
    /// Total prompt tokens used
    pub total_prompt_tokens: u32,
    
    /// Total completion tokens used
    pub total_completion_tokens: u32,
    
    /// Number of requests in this conversation
    pub request_count: u32,
    
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
    
    /// Client type that initiated the conversation (e.g., "codex-cli")
    pub client_type: Option<String>,
    
    /// Client version information
    pub client_version: Option<String>,
}

/// Result of conversation operations
#[derive(Debug, Clone)]
pub enum ConversationResult<T> {
    /// Operation successful
    Success(T),
    
    /// Conversation not found
    NotFound(String),
    
    /// Conversation expired
    Expired(String),
    
    /// Invalid request
    Invalid(String),
    
    /// Internal error
    Error(String),
}

/// Conversation creation parameters
#[derive(Debug, Clone)]
pub struct CreateConversationParams {
    /// Initial messages (optional)
    pub messages: Vec<ChatMessage>,
    
    /// Model to use
    pub model: String,
    
    /// User identifier (optional)
    pub user_id: Option<String>,
    
    /// Custom expiration time (optional, defaults to 24 hours)
    pub expires_in_hours: Option<u32>,
    
    /// Client metadata
    pub client_type: Option<String>,
    pub client_version: Option<String>,
    
    /// Custom metadata
    pub custom_metadata: HashMap<String, String>,
}

impl ConversationState {
    /// Create a new conversation state
    pub fn new(params: CreateConversationParams) -> Self {
        let now = Utc::now();
        let expires_hours = params.expires_in_hours.unwrap_or(24);
        
        Self {
            conversation_id: Uuid::new_v4().to_string(),
            messages: params.messages,
            last_response_id: None,
            metadata: ConversationMetadata {
                total_tokens: 0,
                total_prompt_tokens: 0,
                total_completion_tokens: 0,
                request_count: 0,
                custom: params.custom_metadata,
                client_type: params.client_type,
                client_version: params.client_version,
            },
            created_at: now,
            updated_at: now,
            expires_at: now + chrono::Duration::hours(expires_hours as i64),
            model: params.model,
            user_id: params.user_id,
            turn_count: 0,
            active: true,
        }
    }
    
    /// Add a message to the conversation
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }
    
    /// Add messages to the conversation
    pub fn add_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages.extend(messages);
        self.updated_at = Utc::now();
    }
    
    /// Set the last response ID
    pub fn set_last_response_id(&mut self, response_id: String) {
        self.last_response_id = Some(response_id);
        self.turn_count += 1;
        self.updated_at = Utc::now();
    }
    
    /// Update usage statistics
    pub fn update_usage(&mut self, prompt_tokens: u32, completion_tokens: u32) {
        self.metadata.total_prompt_tokens += prompt_tokens;
        self.metadata.total_completion_tokens += completion_tokens;
        self.metadata.total_tokens += prompt_tokens + completion_tokens;
        self.metadata.request_count += 1;
        self.updated_at = Utc::now();
    }
    
    /// Check if the conversation is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Extend the conversation expiration time
    pub fn extend_expiration(&mut self, hours: u32) {
        self.expires_at = Utc::now() + chrono::Duration::hours(hours as i64);
        self.updated_at = Utc::now();
    }
    
    /// Mark conversation as inactive
    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }
    
    /// Get conversation age in minutes
    pub fn age_minutes(&self) -> i64 {
        (Utc::now() - self.created_at).num_minutes()
    }
    
    /// Get time since last update in minutes
    pub fn idle_minutes(&self) -> i64 {
        (Utc::now() - self.updated_at).num_minutes()
    }
}

impl<T> ConversationResult<T> {
    /// Check if the result is successful
    pub fn is_success(&self) -> bool {
        matches!(self, ConversationResult::Success(_))
    }
    
    /// Check if the result is an error
    pub fn is_error(&self) -> bool {
        !self.is_success()
    }
    
    /// Get the success value or panic
    pub fn unwrap(self) -> T {
        match self {
            ConversationResult::Success(value) => value,
            ConversationResult::NotFound(msg) => panic!("Conversation not found: {}", msg),
            ConversationResult::Expired(msg) => panic!("Conversation expired: {}", msg),
            ConversationResult::Invalid(msg) => panic!("Invalid conversation: {}", msg),
            ConversationResult::Error(msg) => panic!("Conversation error: {}", msg),
        }
    }
    
    /// Get the success value or return a default
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            ConversationResult::Success(value) => value,
            _ => default,
        }
    }
    
    /// Convert to a standard Result
    pub fn into_result(self) -> Result<T, String> {
        match self {
            ConversationResult::Success(value) => Ok(value),
            ConversationResult::NotFound(msg) => Err(format!("Not found: {}", msg)),
            ConversationResult::Expired(msg) => Err(format!("Expired: {}", msg)),
            ConversationResult::Invalid(msg) => Err(format!("Invalid: {}", msg)),
            ConversationResult::Error(msg) => Err(format!("Error: {}", msg)),
        }
    }
}

impl Default for CreateConversationParams {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            model: "claude-4-sonnet-20250514".to_string(),
            user_id: None,
            expires_in_hours: Some(24),
            client_type: None,
            client_version: None,
            custom_metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::MessageRole;
    
    #[test]
    fn test_conversation_state_creation() {
        let params = CreateConversationParams {
            model: "test-model".to_string(),
            messages: vec![ChatMessage::user("Hello")],
            ..Default::default()
        };
        
        let state = ConversationState::new(params);
        
        assert!(!state.conversation_id.is_empty());
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.model, "test-model");
        assert!(state.active);
        assert_eq!(state.turn_count, 0);
    }
    
    #[test]
    fn test_add_message() {
        let mut state = ConversationState::new(CreateConversationParams::default());
        let initial_update = state.updated_at;
        
        // Wait a small amount to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        state.add_message(ChatMessage::user("Test message"));
        
        assert_eq!(state.messages.len(), 1);
        assert!(state.updated_at > initial_update);
    }
    
    #[test]
    fn test_usage_tracking() {
        let mut state = ConversationState::new(CreateConversationParams::default());
        
        state.update_usage(10, 5);
        
        assert_eq!(state.metadata.total_prompt_tokens, 10);
        assert_eq!(state.metadata.total_completion_tokens, 5);
        assert_eq!(state.metadata.total_tokens, 15);
        assert_eq!(state.metadata.request_count, 1);
    }
    
    #[test]
    fn test_conversation_result() {
        let success: ConversationResult<String> = ConversationResult::Success("test".to_string());
        assert!(success.is_success());
        assert!(!success.is_error());
        
        let error: ConversationResult<String> = ConversationResult::Error("test error".to_string());
        assert!(!error.is_success());
        assert!(error.is_error());
    }
    
    #[test]
    fn test_expiration() {
        let mut params = CreateConversationParams::default();
        params.expires_in_hours = Some(0); // Expired immediately for testing
        
        let state = ConversationState::new(params);
        
        // Should be expired since we set 0 hours
        assert!(state.is_expired());
    }
}