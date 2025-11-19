// src/infrastructure/common/types.rs
//
// Common types and enums used across all providers
// Re-exported from shared types to maintain backward compatibility

pub use crate::shared::types::{
    AIProvider, ProviderCapabilities, InputType, OutputFormat,
    ProviderConfig, ModelMapping, default_model_mappings
};
