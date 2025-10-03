// src/infrastructure/vertex/format_converter.rs
//
// Legacy FormatConverter wrapper for backward compatibility
// This provides the old API while using the new VertexAIConverter internally

use anyhow::Result;
use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk},
};
use super::{VertexPredictRequest, VertexPredictResponse, VertexStreamChunk, VertexAIConverter};
use crate::infrastructure::common::ProviderConverter;

/// Legacy FormatConverter for backward compatibility
pub struct FormatConverter;

impl FormatConverter {
    /// Convert OpenAI request to Vertex AI format (non-streaming)
    pub fn openai_to_vertex_v2(request: &ChatCompletionRequest) -> Result<VertexPredictRequest> {
        let converter = VertexAIConverter::new();
        converter.openai_to_provider_request(request)
    }

    /// Convert OpenAI request to Vertex AI streaming format
    pub fn openai_to_vertex_streaming(request: &ChatCompletionRequest) -> Result<VertexPredictRequest> {
        let converter = VertexAIConverter::new();
        converter.openai_to_provider_streaming_request(request)
    }

    /// Convert Vertex AI response to OpenAI format
    pub fn vertex_to_openai_v2(
        response: &VertexPredictResponse,
        request_id: &str,
        model: &str,
        original_request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let converter = VertexAIConverter::new();
        converter.provider_to_openai_response(response, request_id, model, original_request)
    }

    /// Convert Vertex AI streaming chunk to OpenAI format
    pub fn vertex_chunk_to_openai_v2(
        chunk: &VertexStreamChunk,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
        let converter = VertexAIConverter::new();
        converter.provider_chunk_to_openai_chunk(chunk, request_id, model)
    }
}
