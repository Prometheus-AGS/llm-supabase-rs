// Claude-specific Vertex AI integration
// This will contain Claude-specific logic for tool calling, etc.

use crate::shared::AppResult;

pub struct ClaudeVertexClient;

impl ClaudeVertexClient {
    pub fn new() -> AppResult<Self> {
        Ok(Self)
    }
}
