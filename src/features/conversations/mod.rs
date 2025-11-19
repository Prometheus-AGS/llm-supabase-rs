//! Conversation management module
//!
//! This module provides multi-turn conversation support for Codex CLI compatibility.
//! It handles conversation state, message history, and previous_response_id tracking.

pub mod models;
pub mod storage;
pub mod conversation_manager;

// Re-export main types for easier access
pub use conversation_manager::ConversationManager;
pub use models::{ConversationState, ConversationMetadata};
pub use storage::ConversationStorage;