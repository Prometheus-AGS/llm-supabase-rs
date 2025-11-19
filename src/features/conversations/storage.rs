//! Conversation storage implementation
//!
//! Provides in-memory storage for conversation state with thread safety,
//! cleanup mechanisms, and efficient lookups. This is a foundational
//! implementation that can be extended to use persistent storage later.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn, error, instrument};

use super::models::{ConversationState, ConversationResult};

/// Thread-safe in-memory conversation storage
#[derive(Debug, Clone)]
pub struct ConversationStorage {
    /// Internal storage with thread safety
    storage: Arc<RwLock<HashMap<String, ConversationState>>>,
    
    /// Index by response ID for quick lookups
    response_index: Arc<RwLock<HashMap<String, String>>>,
    
    /// Storage statistics
    stats: Arc<RwLock<StorageStats>>,
}

/// Storage statistics for monitoring and debugging
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total conversations created
    pub total_created: u64,
    
    /// Total conversations expired/cleaned up
    pub total_expired: u64,
    
    /// Current active conversations
    pub active_count: u64,
    
    /// Last cleanup timestamp
    pub last_cleanup: Option<DateTime<Utc>>,
    
    /// Total cleanup operations performed
    pub cleanup_count: u64,
    
    /// Total lookups performed
    pub total_lookups: u64,
    
    /// Total successful lookups
    pub successful_lookups: u64,
}

impl ConversationStorage {
    /// Create a new conversation storage instance
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            response_index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(StorageStats::default())),
        }
    }
    
    /// Store a conversation
    #[instrument(skip(self, conversation), fields(conversation_id = %conversation.conversation_id))]
    pub fn store(&self, conversation: ConversationState) -> ConversationResult<()> {
        let conversation_id = conversation.conversation_id.clone();
        
        debug!("Storing conversation: {}", conversation_id);
        
        // Get write locks
        let mut storage = match self.storage.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire storage write lock: {}", e);
                return ConversationResult::Error(format!("Storage lock error: {}", e));
            }
        };
        
        let mut response_index = match self.response_index.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire response index write lock: {}", e);
                return ConversationResult::Error(format!("Index lock error: {}", e));
            }
        };
        
        let mut stats = match self.stats.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire stats write lock: {}", e);
                return ConversationResult::Error(format!("Stats lock error: {}", e));
            }
        };
        
        // Update response index if we have a last response ID
        if let Some(response_id) = &conversation.last_response_id {
            response_index.insert(response_id.clone(), conversation_id.clone());
        }
        
        // Check if this is a new conversation
        let is_new = !storage.contains_key(&conversation_id);
        
        // Store the conversation
        storage.insert(conversation_id.clone(), conversation);
        
        // Update stats
        if is_new {
            stats.total_created += 1;
            stats.active_count += 1;
        }
        
        debug!("Successfully stored conversation: {}", conversation_id);
        ConversationResult::Success(())
    }
    
    /// Retrieve a conversation by ID
    #[instrument(skip(self), fields(conversation_id = %conversation_id))]
    pub fn get(&self, conversation_id: &str) -> ConversationResult<ConversationState> {
        debug!("Looking up conversation: {}", conversation_id);
        
        // Update lookup stats
        {
            if let Ok(mut stats) = self.stats.write() {
                stats.total_lookups += 1;
            }
        }
        
        let storage = match self.storage.read() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire storage read lock: {}", e);
                return ConversationResult::Error(format!("Storage lock error: {}", e));
            }
        };
        
        match storage.get(conversation_id) {
            Some(conversation) => {
                if conversation.is_expired() {
                    warn!("Conversation {} is expired", conversation_id);
                    ConversationResult::Expired(conversation_id.to_string())
                } else {
                    debug!("Found conversation: {}", conversation_id);
                    
                    // Update successful lookup stats
                    if let Ok(mut stats) = self.stats.write() {
                        stats.successful_lookups += 1;
                    }
                    
                    ConversationResult::Success(conversation.clone())
                }
            }
            None => {
                debug!("Conversation not found: {}", conversation_id);
                ConversationResult::NotFound(conversation_id.to_string())
            }
        }
    }
    
    /// Retrieve a conversation by previous response ID
    #[instrument(skip(self), fields(response_id = %response_id))]
    pub fn get_by_response_id(&self, response_id: &str) -> ConversationResult<ConversationState> {
        debug!("Looking up conversation by response ID: {}", response_id);
        
        // First, find the conversation ID using the response index
        let conversation_id = {
            let response_index = match self.response_index.read() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("Failed to acquire response index read lock: {}", e);
                    return ConversationResult::Error(format!("Index lock error: {}", e));
                }
            };
            
            match response_index.get(response_id) {
                Some(id) => id.clone(),
                None => {
                    debug!("No conversation found for response ID: {}", response_id);
                    return ConversationResult::NotFound(format!("Response ID: {}", response_id));
                }
            }
        };
        
        // Now get the conversation using the conversation ID
        self.get(&conversation_id)
    }
    
    /// Update an existing conversation
    #[instrument(skip(self, conversation), fields(conversation_id = %conversation.conversation_id))]
    pub fn update(&self, conversation: ConversationState) -> ConversationResult<()> {
        let conversation_id = conversation.conversation_id.clone();
        
        debug!("Updating conversation: {}", conversation_id);
        
        // Check if conversation exists first
        {
            let storage = match self.storage.read() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("Failed to acquire storage read lock: {}", e);
                    return ConversationResult::Error(format!("Storage lock error: {}", e));
                }
            };
            
            if !storage.contains_key(&conversation_id) {
                return ConversationResult::NotFound(conversation_id);
            }
        }
        
        // Store the updated conversation (this will overwrite the existing one)
        self.store(conversation)
    }
    
    /// Remove a conversation
    #[instrument(skip(self), fields(conversation_id = %conversation_id))]
    pub fn remove(&self, conversation_id: &str) -> ConversationResult<ConversationState> {
        debug!("Removing conversation: {}", conversation_id);
        
        let mut storage = match self.storage.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire storage write lock: {}", e);
                return ConversationResult::Error(format!("Storage lock error: {}", e));
            }
        };
        
        let mut response_index = match self.response_index.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire response index write lock: {}", e);
                return ConversationResult::Error(format!("Index lock error: {}", e));
            }
        };
        
        let mut stats = match self.stats.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire stats write lock: {}", e);
                return ConversationResult::Error(format!("Stats lock error: {}", e));
            }
        };
        
        match storage.remove(conversation_id) {
            Some(conversation) => {
                // Remove from response index if applicable
                if let Some(response_id) = &conversation.last_response_id {
                    response_index.remove(response_id);
                }
                
                // Update stats
                stats.active_count = stats.active_count.saturating_sub(1);
                
                debug!("Successfully removed conversation: {}", conversation_id);
                ConversationResult::Success(conversation)
            }
            None => {
                debug!("Conversation not found for removal: {}", conversation_id);
                ConversationResult::NotFound(conversation_id.to_string())
            }
        }
    }
    
    /// Clean up expired conversations
    #[instrument(skip(self))]
    pub async fn cleanup_expired(&self) -> Result<u64, String> {
        debug!("Starting cleanup of expired conversations");
        
        let expired_ids = {
            let storage = self.storage.read()
                .map_err(|e| format!("Failed to acquire storage read lock: {}", e))?;
            
            let now = Utc::now();
            storage.iter()
                .filter(|(_, conversation)| conversation.is_expired())
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };
        
        let mut cleaned_count = 0;
        
        for conversation_id in expired_ids {
            match self.remove(&conversation_id) {
                ConversationResult::Success(_) => {
                    cleaned_count += 1;
                    debug!("Cleaned up expired conversation: {}", conversation_id);
                }
                ConversationResult::NotFound(_) => {
                    // Already removed, that's fine
                }
                result => {
                    warn!("Failed to remove expired conversation {}: {:?}", conversation_id, result);
                }
            }
        }
        
        // Update stats
        {
            if let Ok(mut stats) = self.stats.write() {
                stats.total_expired += cleaned_count;
                stats.last_cleanup = Some(Utc::now());
                stats.cleanup_count += 1;
            }
        }
        
        if cleaned_count > 0 {
            info!("Cleaned up {} expired conversations", cleaned_count);
        } else {
            debug!("No expired conversations to clean up");
        }
        
        Ok(cleaned_count)
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats, String> {
        self.stats.read()
            .map(|stats| stats.clone())
            .map_err(|e| format!("Failed to acquire stats read lock: {}", e))
    }
    
    /// Get the number of active conversations
    pub fn count(&self) -> Result<usize, String> {
        self.storage.read()
            .map(|storage| storage.len())
            .map_err(|e| format!("Failed to acquire storage read lock: {}", e))
    }
    
    /// List all conversation IDs (for debugging/admin purposes)
    pub fn list_ids(&self) -> Result<Vec<String>, String> {
        self.storage.read()
            .map(|storage| storage.keys().cloned().collect())
            .map_err(|e| format!("Failed to acquire storage read lock: {}", e))
    }
    
    /// Start automatic cleanup task
    pub async fn start_cleanup_task(storage: Arc<ConversationStorage>) {
        let mut cleanup_interval = interval(Duration::from_secs(300)); // Every 5 minutes
        
        info!("Starting automatic conversation cleanup task");
        
        loop {
            cleanup_interval.tick().await;
            
            match storage.cleanup_expired().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Automatic cleanup removed {} expired conversations", count);
                    }
                }
                Err(e) => {
                    error!("Automatic cleanup failed: {}", e);
                }
            }
        }
    }
}

impl Default for ConversationStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::conversations::models::CreateConversationParams;
    use std::time::Duration as StdDuration;
    
    #[test]
    fn test_storage_creation() {
        let storage = ConversationStorage::new();
        assert_eq!(storage.count().unwrap(), 0);
    }
    
    #[test]
    fn test_store_and_retrieve() {
        let storage = ConversationStorage::new();
        let conversation = ConversationState::new(CreateConversationParams::default());
        let conversation_id = conversation.conversation_id.clone();
        
        // Store conversation
        let result = storage.store(conversation);
        assert!(result.is_success());
        
        // Retrieve conversation
        let retrieved = storage.get(&conversation_id);
        assert!(retrieved.is_success());
        
        let retrieved_conversation = retrieved.unwrap();
        assert_eq!(retrieved_conversation.conversation_id, conversation_id);
    }
    
    #[test]
    fn test_response_id_lookup() {
        let storage = ConversationStorage::new();
        let mut conversation = ConversationState::new(CreateConversationParams::default());
        let conversation_id = conversation.conversation_id.clone();
        let response_id = "test-response-123".to_string();
        
        conversation.set_last_response_id(response_id.clone());
        storage.store(conversation).unwrap();
        
        // Look up by response ID
        let retrieved = storage.get_by_response_id(&response_id);
        assert!(retrieved.is_success());
        
        let retrieved_conversation = retrieved.unwrap();
        assert_eq!(retrieved_conversation.conversation_id, conversation_id);
    }
    
    #[test]
    fn test_not_found() {
        let storage = ConversationStorage::new();
        
        let result = storage.get("nonexistent");
        assert!(!result.is_success());
        
        if let ConversationResult::NotFound(_) = result {
            // Expected
        } else {
            panic!("Expected NotFound result");
        }
    }
    
    #[test]
    fn test_remove_conversation() {
        let storage = ConversationStorage::new();
        let conversation = ConversationState::new(CreateConversationParams::default());
        let conversation_id = conversation.conversation_id.clone();
        
        // Store and then remove
        storage.store(conversation).unwrap();
        assert_eq!(storage.count().unwrap(), 1);
        
        let removed = storage.remove(&conversation_id);
        assert!(removed.is_success());
        assert_eq!(storage.count().unwrap(), 0);
    }
    
    #[tokio::test]
    async fn test_expired_cleanup() {
        let storage = ConversationStorage::new();
        
        // Create an expired conversation
        let mut params = CreateConversationParams::default();
        params.expires_in_hours = Some(0); // Already expired
        let conversation = ConversationState::new(params);
        
        storage.store(conversation).unwrap();
        assert_eq!(storage.count().unwrap(), 1);
        
        // Run cleanup
        let cleaned = storage.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 1);
        assert_eq!(storage.count().unwrap(), 0);
    }
    
    #[test]
    fn test_statistics() {
        let storage = ConversationStorage::new();
        let conversation = ConversationState::new(CreateConversationParams::default());
        let conversation_id = conversation.conversation_id.clone();
        
        // Initial stats
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.active_count, 0);
        
        // Store conversation
        storage.store(conversation).unwrap();
        
        // Updated stats
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_created, 1);
        assert_eq!(stats.active_count, 1);
        
        // Test lookup stats
        storage.get(&conversation_id).unwrap();
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_lookups, 1);
        assert_eq!(stats.successful_lookups, 1);
    }
}