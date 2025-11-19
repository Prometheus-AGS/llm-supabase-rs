//! Conversation manager
//!
//! High-level interface for managing multi-turn conversations.
//! Handles conversation creation, retrieval, updating, and context stitching
//! for Codex CLI compatibility.

use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;
use tracing::{debug, info, warn, error, instrument};

use crate::models::common::ChatMessage;
use crate::models::request::ChatCompletionRequest;
use crate::models::response::ChatCompletionResponse;
use crate::shared::AppError;

use super::models::{ConversationState, ConversationResult, CreateConversationParams, ConversationMetadata};
use super::storage::ConversationStorage;

/// High-level conversation manager
#[derive(Debug, Clone)]
pub struct ConversationManager {
    /// Underlying storage
    storage: Arc<ConversationStorage>,
}

/// Parameters for creating or continuing a conversation
#[derive(Debug, Clone)]
pub struct ConversationRequest {
    /// Original chat completion request
    pub chat_request: ChatCompletionRequest,
    
    /// Optional user identifier
    pub user_id: Option<String>,
    
    /// Client type information
    pub client_type: Option<String>,
    pub client_version: Option<String>,
    
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Result of conversation processing
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// Conversation ID
    pub conversation_id: String,
    
    /// Complete message history to send to the AI provider
    pub messages: Vec<ChatMessage>,
    
    /// Whether this is a new conversation
    pub is_new_conversation: bool,
    
    /// Previous turn count
    pub previous_turn_count: u32,
    
    /// Conversation metadata
    pub metadata: ConversationMetadata,
}

/// Parameters for updating conversation after response
#[derive(Debug, Clone)]
pub struct UpdateConversationParams {
    /// Conversation ID
    pub conversation_id: String,
    
    /// Response ID generated for this turn
    pub response_id: String,
    
    /// Assistant response message
    pub response_message: ChatMessage,
    
    /// Token usage information
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new() -> Self {
        Self {
            storage: Arc::new(ConversationStorage::new()),
        }
    }
    
    /// Create conversation manager with custom storage
    pub fn with_storage(storage: Arc<ConversationStorage>) -> Self {
        Self { storage }
    }
    
    /// Process a conversation request and return context for the AI provider
    #[instrument(skip(self, request), fields(
        model = %request.chat_request.model,
        previous_response_id = ?request.chat_request.previous_response_id,
        messages_count = request.chat_request.messages.len()
    ))]
    pub async fn process_request(&self, request: ConversationRequest) -> Result<ConversationContext, AppError> {
        let chat_request = &request.chat_request;
        
        // Check if this is a continuation of an existing conversation
        if let Some(previous_response_id) = &chat_request.previous_response_id {
            debug!("Processing conversation continuation with response ID: {}", previous_response_id);
            let response_id = previous_response_id.clone();
            self.continue_conversation(request, &response_id).await
        } else {
            debug!("Creating new conversation");
            self.start_new_conversation(request).await
        }
    }
    
    /// Start a new conversation
    #[instrument(skip(self, request))]
    async fn start_new_conversation(&self, request: ConversationRequest) -> Result<ConversationContext, AppError> {
        let chat_request = &request.chat_request;
        
        // Create conversation parameters
        let params = CreateConversationParams {
            messages: chat_request.messages.clone(),
            model: chat_request.model.clone(),
            user_id: request.user_id.clone(),
            expires_in_hours: Some(24), // Default 24-hour expiration
            client_type: request.client_type.clone(),
            client_version: request.client_version.clone(),
            custom_metadata: request.metadata.clone(),
        };
        
        // Create new conversation state
        let conversation = ConversationState::new(params);
        let conversation_id = conversation.conversation_id.clone();
        let metadata = conversation.metadata.clone();
        
        // Store the conversation
        match self.storage.store(conversation.clone()) {
            ConversationResult::Success(_) => {
                info!("Created new conversation: {}", conversation_id);
                
                Ok(ConversationContext {
                    conversation_id,
                    messages: chat_request.messages.clone(),
                    is_new_conversation: true,
                    previous_turn_count: 0,
                    metadata,
                })
            }
            result => {
                error!("Failed to store new conversation: {:?}", result);
                Err(AppError::internal(format!("Failed to create conversation: {:?}", result)))
            }
        }
    }
    
    /// Continue an existing conversation
    #[instrument(skip(self, request), fields(previous_response_id = %previous_response_id))]
    async fn continue_conversation(&self, request: ConversationRequest, previous_response_id: &str) -> Result<ConversationContext, AppError> {
        let chat_request = &request.chat_request;
        
        // Retrieve existing conversation
        let mut conversation = match self.storage.get_by_response_id(previous_response_id) {
            ConversationResult::Success(conv) => conv,
            ConversationResult::NotFound(_) => {
                warn!("Conversation not found for response ID: {}", previous_response_id);
                return Err(AppError::validation(format!("Invalid previous_response_id: {}", previous_response_id)));
            }
            ConversationResult::Expired(_) => {
                warn!("Conversation expired for response ID: {}", previous_response_id);
                return Err(AppError::validation("Conversation has expired"));
            }
            result => {
                error!("Error retrieving conversation: {:?}", result);
                return Err(AppError::internal(format!("Failed to retrieve conversation: {:?}", result)));
            }
        };
        
        let conversation_id = conversation.conversation_id.clone();
        let previous_turn_count = conversation.turn_count;
        let metadata = conversation.metadata.clone();
        
        // Add new messages from the request to the conversation history
        conversation.add_messages(chat_request.messages.clone());
        
        // Update the conversation in storage
        match self.storage.update(conversation.clone()) {
            ConversationResult::Success(_) => {
                info!("Updated conversation {} with {} new messages", conversation_id, chat_request.messages.len());
            }
            result => {
                error!("Failed to update conversation: {:?}", result);
                return Err(AppError::internal(format!("Failed to update conversation: {:?}", result)));
            }
        }
        
        // Return the complete message history for context
        Ok(ConversationContext {
            conversation_id,
            messages: conversation.messages.clone(),
            is_new_conversation: false,
            previous_turn_count,
            metadata,
        })
    }
    
    /// Update conversation after receiving a response from the AI provider
    #[instrument(skip(self, params), fields(
        conversation_id = %params.conversation_id,
        response_id = %params.response_id
    ))]
    pub async fn update_after_response(&self, params: UpdateConversationParams) -> Result<(), AppError> {
        // Retrieve conversation
        let mut conversation = match self.storage.get(&params.conversation_id) {
            ConversationResult::Success(conv) => conv,
            ConversationResult::NotFound(_) => {
                error!("Conversation not found: {}", params.conversation_id);
                return Err(AppError::internal("Conversation not found"));
            }
            ConversationResult::Expired(_) => {
                warn!("Attempting to update expired conversation: {}", params.conversation_id);
                return Err(AppError::validation("Conversation has expired"));
            }
            result => {
                error!("Error retrieving conversation for update: {:?}", result);
                return Err(AppError::internal(format!("Failed to retrieve conversation: {:?}", result)));
            }
        };
        
        // Add the response message to conversation history
        conversation.add_message(params.response_message);
        
        // Update response ID and turn count
        conversation.set_last_response_id(params.response_id.clone());
        
        // Update usage statistics
        conversation.update_usage(params.prompt_tokens, params.completion_tokens);
        
        // Store updated conversation
        match self.storage.update(conversation) {
            ConversationResult::Success(_) => {
                debug!("Updated conversation {} with response {}", params.conversation_id, params.response_id);
                Ok(())
            }
            result => {
                error!("Failed to update conversation after response: {:?}", result);
                Err(AppError::internal(format!("Failed to update conversation: {:?}", result)))
            }
        }
    }
    
    /// Get conversation by ID
    #[instrument(skip(self), fields(conversation_id = %conversation_id))]
    pub async fn get_conversation(&self, conversation_id: &str) -> Result<ConversationState, AppError> {
        match self.storage.get(conversation_id) {
            ConversationResult::Success(conversation) => Ok(conversation),
            ConversationResult::NotFound(_) => Err(AppError::validation("Conversation not found")),
            ConversationResult::Expired(_) => Err(AppError::validation("Conversation has expired")),
            result => Err(AppError::internal(format!("Failed to retrieve conversation: {:?}", result))),
        }
    }
    
    /// Get conversation by response ID
    #[instrument(skip(self), fields(response_id = %response_id))]
    pub async fn get_conversation_by_response_id(&self, response_id: &str) -> Result<ConversationState, AppError> {
        match self.storage.get_by_response_id(response_id) {
            ConversationResult::Success(conversation) => Ok(conversation),
            ConversationResult::NotFound(_) => Err(AppError::validation("Invalid previous_response_id")),
            ConversationResult::Expired(_) => Err(AppError::validation("Conversation has expired")),
            result => Err(AppError::internal(format!("Failed to retrieve conversation: {:?}", result))),
        }
    }
    
    /// Generate a unique response ID
    pub fn generate_response_id() -> String {
        format!("chatcmpl-{}", Uuid::new_v4().simple().to_string()[0..22].to_uppercase())
    }
    
    /// Clean up expired conversations
    #[instrument(skip(self))]
    pub async fn cleanup_expired(&self) -> Result<u64, AppError> {
        self.storage.cleanup_expired()
            .await
            .map_err(|e| AppError::internal(format!("Cleanup failed: {}", e)))
    }
    
    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<crate::features::conversations::storage::StorageStats, AppError> {
        self.storage.get_stats()
            .map_err(|e| AppError::internal(format!("Failed to get stats: {}", e)))
    }
    
    /// Validate conversation request
    fn validate_request(&self, request: &ConversationRequest) -> Result<(), AppError> {
        let chat_request = &request.chat_request;
        
        // Basic validation
        if chat_request.messages.is_empty() {
            return Err(AppError::validation("Messages cannot be empty"));
        }
        
        if chat_request.model.is_empty() {
            return Err(AppError::validation("Model cannot be empty"));
        }
        
        // Validate message content
        for (i, message) in chat_request.messages.iter().enumerate() {
            if message.content.is_empty() && message.tool_calls.is_none() && message.function_call.is_none() {
                return Err(AppError::validation(format!("Message {} has empty content", i)));
            }
        }
        
        Ok(())
    }
    
    /// Start the automatic cleanup task
    pub async fn start_cleanup_task(&self) {
        let storage = Arc::clone(&self.storage);
        tokio::spawn(async move {
            ConversationStorage::start_cleanup_task(storage).await;
        });
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::MessageRole;
    
    #[tokio::test]
    async fn test_new_conversation() {
        let manager = ConversationManager::new();
        
        let chat_request = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage::user("Hello, world!")],
        );
        
        let request = ConversationRequest {
            chat_request,
            user_id: Some("test-user".to_string()),
            client_type: Some("test-client".to_string()),
            client_version: Some("1.0.0".to_string()),
            metadata: HashMap::new(),
        };
        
        let context = manager.process_request(request).await.unwrap();
        
        assert!(context.is_new_conversation);
        assert_eq!(context.messages.len(), 1);
        assert_eq!(context.previous_turn_count, 0);
    }
    
    #[tokio::test]
    async fn test_continue_conversation() {
        let manager = ConversationManager::new();
        
        // Start new conversation
        let chat_request1 = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage::user("Hello")],
        );
        
        let request1 = ConversationRequest {
            chat_request: chat_request1,
            user_id: Some("test-user".to_string()),
            client_type: Some("test-client".to_string()),
            client_version: None,
            metadata: HashMap::new(),
        };
        
        let context1 = manager.process_request(request1).await.unwrap();
        
        // Simulate AI response
        let response_id = ConversationManager::generate_response_id();
        let update_params = UpdateConversationParams {
            conversation_id: context1.conversation_id.clone(),
            response_id: response_id.clone(),
            response_message: ChatMessage::assistant("Hi there!"),
            prompt_tokens: 10,
            completion_tokens: 5,
        };
        
        manager.update_after_response(update_params).await.unwrap();
        
        // Continue conversation
        let mut chat_request2 = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage::user("How are you?")],
        );
        chat_request2.previous_response_id = Some(response_id);
        
        let request2 = ConversationRequest {
            chat_request: chat_request2,
            user_id: Some("test-user".to_string()),
            client_type: Some("test-client".to_string()),
            client_version: None,
            metadata: HashMap::new(),
        };
        
        let context2 = manager.process_request(request2).await.unwrap();
        
        assert!(!context2.is_new_conversation);
        assert_eq!(context2.conversation_id, context1.conversation_id);
        assert_eq!(context2.messages.len(), 3); // Original user + assistant + new user
        assert_eq!(context2.previous_turn_count, 1);
    }
    
    #[tokio::test]
    async fn test_invalid_previous_response_id() {
        let manager = ConversationManager::new();
        
        let mut chat_request = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage::user("Hello")],
        );
        chat_request.previous_response_id = Some("invalid-id".to_string());
        
        let request = ConversationRequest {
            chat_request,
            user_id: None,
            client_type: None,
            client_version: None,
            metadata: HashMap::new(),
        };
        
        let result = manager.process_request(request).await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_response_id_generation() {
        let id1 = ConversationManager::generate_response_id();
        let id2 = ConversationManager::generate_response_id();
        
        assert_ne!(id1, id2);
        assert!(id1.starts_with("chatcmpl-"));
        assert_eq!(id1.len(), 29); // "chatcmpl-" + 22 chars
    }
    
    #[tokio::test]
    async fn test_conversation_retrieval() {
        let manager = ConversationManager::new();
        
        // Create a conversation
        let chat_request = ChatCompletionRequest::new(
            "claude-4-sonnet-20250514",
            vec![ChatMessage::user("Hello")],
        );
        
        let request = ConversationRequest {
            chat_request,
            user_id: Some("test-user".to_string()),
            client_type: None,
            client_version: None,
            metadata: HashMap::new(),
        };
        
        let context = manager.process_request(request).await.unwrap();
        
        // Retrieve by conversation ID
        let retrieved = manager.get_conversation(&context.conversation_id).await.unwrap();
        assert_eq!(retrieved.conversation_id, context.conversation_id);
        
        // Test non-existent conversation
        let result = manager.get_conversation("nonexistent").await;
        assert!(result.is_err());
    }
}