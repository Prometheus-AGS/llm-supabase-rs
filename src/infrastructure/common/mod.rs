// src/infrastructure/common/mod.rs
//
// Common infrastructure traits and types shared across all providers

pub mod adaptive_formatter;
pub mod client_detection;
pub mod client;
pub mod converter;
pub mod streaming;
pub mod streaming_v2;
pub mod types;
pub mod tools;

pub use adaptive_formatter::*;
pub use client_detection::*;
pub use client::GenericLLMClient;
pub use converter::{ProviderConverter, StreamingConverter, ProviderErrorHandler};
pub use converter::utils as converter_utils;
pub use streaming::*;
pub use streaming_v2::{
    StreamNormalizer, StreamProcessorFactory, StreamProcessor,
    StreamingBehavior, NormalizedChunk
};
pub use types::*;
pub use tools::{
    UnifiedToolCall, ToolCallResult, ToolCallState, ToolCallConverter,
    OpenAIToolConverter, ClaudeToolConverter, ToolCallManager
};
pub use tools::utils as tool_utils;
