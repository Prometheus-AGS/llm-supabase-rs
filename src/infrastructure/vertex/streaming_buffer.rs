// src/infrastructure/vertex/streaming_buffer.rs
//
// Enhanced JSON streaming buffer for handling incomplete fragments from Vertex AI
// with integrated tool_use detection and proper chunk accumulation

use serde_json::Value;
use tracing::{debug, warn};

/// Represents different types of processed chunks from the buffer
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedChunk {
    /// Regular content chunk with text
    Content(String),
    /// Tool use chunk with structured data
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    /// Complete message chunk (usually final)
    Message(Value),
    /// Done signal
    Done,
}

/// Enhanced buffer for accumulating streaming JSON fragments until complete objects are available
/// Now includes tool_use detection and proper state management
pub struct JsonStreamingBuffer {
    /// Internal buffer for accumulating partial JSON
    buffer: String,
    
    /// Track brace depth to detect complete JSON objects
    brace_depth: i32,
    
    /// Track if we're inside a string (to ignore braces in strings)
    in_string: bool,
    
    /// Track if the previous character was an escape character
    escaped: bool,
    
    /// Track accumulated tool calls across chunks
    accumulated_tool_calls: Vec<ProcessedChunk>,
    
    /// Track if we're currently building a tool call
    building_tool_call: bool,
    
    /// Buffer for incomplete tool call data
    tool_call_buffer: String,
}

impl JsonStreamingBuffer {
    /// Create a new streaming buffer
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            brace_depth: 0,
            in_string: false,
            escaped: false,
            accumulated_tool_calls: Vec::new(),
            building_tool_call: false,
            tool_call_buffer: String::new(),
        }
    }

    /// Add a chunk to the buffer and return any complete processed chunks
    /// This enhanced version handles both content and tool_use detection
    pub fn add_chunk(&mut self, chunk: &str) -> Vec<ProcessedChunk> {
        debug!("Adding chunk to buffer: {} chars, content: {}", chunk.len(), chunk);
        
        let mut processed_chunks = Vec::new();
        let mut object_start = 0;
        
        // Add chunk to buffer
        self.buffer.push_str(chunk);
        
        // Process character by character to detect complete JSON objects
        let chars: Vec<char> = self.buffer.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            match ch {
                '"' if !self.escaped => {
                    self.in_string = !self.in_string;
                }
                '{' if !self.in_string => {
                    if self.brace_depth == 0 {
                        object_start = i;
                    }
                    self.brace_depth += 1;
                }
                '}' if !self.in_string => {
                    self.brace_depth -= 1;
                    
                    // Complete JSON object detected
                    if self.brace_depth == 0 && object_start < i {
                        let json_str = chars[object_start..=i].iter().collect::<String>();
                        debug!("Complete JSON object detected: {} chars", json_str.len());
                        
                        // Process the complete JSON object
                        if let Some(chunk) = self.process_complete_json(&json_str) {
                            processed_chunks.push(chunk);
                        }
                        
                        object_start = i + 1;
                    }
                }
                _ => {}
            }
            
            // Update escape status
            self.escaped = ch == '\\' && !self.escaped;
        }
        
        // Remove processed objects from buffer
        if !processed_chunks.is_empty() && object_start < chars.len() {
            self.buffer = chars[object_start..].iter().collect();
            // Reset state for remaining buffer
            self.reset_state_for_remaining_buffer();
        }
        
        debug!("Extracted {} processed chunks from buffer", processed_chunks.len());
        processed_chunks
    }

    /// Process a complete JSON string and determine its type
    fn process_complete_json(&mut self, json_str: &str) -> Option<ProcessedChunk> {
        // Try to parse as JSON first
        let json_value: Value = match serde_json::from_str(json_str) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse JSON: {}, content: {}", e, json_str);
                return None;
            }
        };

        // Check for different types of chunks
        if let Some(event_type) = json_value.get("type").and_then(|t| t.as_str()) {
            match event_type {
                "message_stop" => {
                    debug!("Detected message_stop chunk");
                    return Some(ProcessedChunk::Done);
                }
                "message" => {
                    // This is a complete message from Vertex AI - extract content
                    debug!("Detected complete message from Vertex AI");
                    if let Some(content_array) = json_value.get("content").and_then(|c| c.as_array()) {
                        return self.process_content_array(content_array, &json_value);
                    }
                    // If no content array, return as complete message
                    return Some(ProcessedChunk::Message(json_value));
                }
                "content_block_delta" => {
                    // Check if this contains text content
                    if let Some(delta) = json_value.get("delta") {
                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                debug!("Extracted text content: {} chars", text.len());
                                return Some(ProcessedChunk::Content(text.to_string()));
                            }
                        }
                    }
                }
                "content_block_start" => {
                    // Check if this is a tool_use block
                    if let Some(content_block) = json_value.get("content_block") {
                        if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            debug!("Detected tool_use block start");
                            self.building_tool_call = true;
                            self.tool_call_buffer = json_str.to_string();
                            return None; // Don't emit yet, wait for complete tool call
                        }
                    }
                }
                "content_block_stop" => {
                    if self.building_tool_call {
                        debug!("Tool call block completed");
                        self.building_tool_call = false;
                        // Try to extract tool call from accumulated buffer
                        if let Some(tool_chunk) = self.extract_tool_call_from_buffer() {
                            self.tool_call_buffer.clear();
                            return Some(tool_chunk);
                        }
                    }
                }
                _ => {
                    debug!("Unknown event type: {}", event_type);
                }
            }
        }

        // Check if this is a complete message with content array (Vertex AI format)
        if let Some(content_array) = json_value.get("content").and_then(|c| c.as_array()) {
            debug!("Processing complete message with content array");
            return self.process_content_array(content_array, &json_value);
        }

        None
    }

    /// Process content array from Vertex AI complete message format
    fn process_content_array(&mut self, content_array: &[Value], _full_message: &Value) -> Option<ProcessedChunk> {
        debug!("Processing content array with {} blocks", content_array.len());
        
        for (i, content_block) in content_array.iter().enumerate() {
            debug!("Processing content block {}: {:?}", i, content_block);
            
            if let Some(block_type) = content_block.get("type").and_then(|t| t.as_str()) {
                debug!("Content block type: {}", block_type);
                match block_type {
                    "text" => {
                        if let Some(text) = content_block.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                debug!("Extracted text from content array: {} chars", text.len());
                                // Split the text into smaller chunks to simulate streaming
                                return Some(ProcessedChunk::Content(text.to_string()));
                            } else {
                                debug!("Text content is empty");
                            }
                        } else {
                            debug!("No text field found in text block");
                        }
                    }
                    "tool_use" => {
                        debug!("Extracting tool_use from content array");
                        if let (Some(id), Some(name), Some(input)) = (
                            content_block.get("id").and_then(|i| i.as_str()),
                            content_block.get("name").and_then(|n| n.as_str()),
                            content_block.get("input")
                        ) {
                            let tool_chunk = ProcessedChunk::ToolUse {
                                id: id.to_string(),
                                name: name.to_string(),
                                input: input.clone(),
                            };
                            // Store for batch processing
                            self.accumulated_tool_calls.push(tool_chunk.clone());
                            return Some(tool_chunk);
                        }
                    }
                    _ => {
                        debug!("Unknown content block type: {}", block_type);
                    }
                }
            } else {
                debug!("No type field found in content block");
            }
        }

        debug!("No specific content found in array");
        None
    }

    /// Extract tool call from accumulated buffer
    fn extract_tool_call_from_buffer(&self) -> Option<ProcessedChunk> {
        if self.tool_call_buffer.is_empty() {
            return None;
        }

        if let Ok(json_value) = serde_json::from_str::<Value>(&self.tool_call_buffer) {
            if let Some(content_block) = json_value.get("content_block") {
                if let (Some(id), Some(name), Some(input)) = (
                    content_block.get("id").and_then(|i| i.as_str()),
                    content_block.get("name").and_then(|n| n.as_str()),
                    content_block.get("input")
                ) {
                    debug!("Extracted tool call: {} ({})", name, id);
                    return Some(ProcessedChunk::ToolUse {
                        id: id.to_string(),
                        name: name.to_string(),
                        input: input.clone(),
                    });
                }
            }
        }

        None
    }

    /// Get accumulated tool calls (for batch processing)
    pub fn get_accumulated_tool_calls(&self) -> &[ProcessedChunk] {
        &self.accumulated_tool_calls
    }

    /// Clear accumulated tool calls
    pub fn clear_accumulated_tool_calls(&mut self) {
        self.accumulated_tool_calls.clear();
    }
    
    /// Process any remaining buffer content when stream ends
    pub fn finalize(&mut self) -> Option<ProcessedChunk> {
        if !self.buffer.trim().is_empty() {
            debug!("Finalizing buffer with remaining content: {} chars", self.buffer.len());
            
            // Try to parse remaining content
            if let Ok(_json_value) = serde_json::from_str::<serde_json::Value>(&self.buffer) {
                let remaining = self.buffer.clone();
                self.buffer.clear();
                
                // Process the remaining JSON
                return self.process_complete_json(&remaining);
            } else {
                warn!("Buffer contains incomplete JSON at stream end: {}", self.buffer);
                self.buffer.clear();
            }
        }
        None
    }
    
    /// Reset parsing state for remaining buffer after extracting objects
    fn reset_state_for_remaining_buffer(&mut self) {
        self.brace_depth = 0;
        self.in_string = false;
        self.escaped = false;
        
        // Recalculate state for remaining buffer
        for ch in self.buffer.chars() {
            match ch {
                '"' if !self.escaped => {
                    self.in_string = !self.in_string;
                }
                '{' if !self.in_string => {
                    self.brace_depth += 1;
                }
                '}' if !self.in_string => {
                    self.brace_depth -= 1;
                }
                _ => {}
            }
            
            self.escaped = ch == '\\' && !self.escaped;
        }
    }
    
    /// Check if buffer has content
    pub fn has_content(&self) -> bool {
        !self.buffer.trim().is_empty()
    }
    
    /// Get current buffer size for debugging
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_single_object() {
        let mut buffer = JsonStreamingBuffer::new();
        
        // Add complete object in one chunk
        let objects = buffer.add_chunk(r#"{"id": "test", "content": "hello"}"#);
        assert_eq!(objects.len(), 1);
        // This should be processed as a Message chunk since it doesn't match streaming patterns
        match &objects[0] {
            ProcessedChunk::Message(json_val) => {
                assert_eq!(json_val.get("id").and_then(|v| v.as_str()), Some("test"));
                assert_eq!(json_val.get("content").and_then(|v| v.as_str()), Some("hello"));
            }
            _ => panic!("Expected Message chunk"),
        }
    }

    #[test]
    fn test_fragmented_object() {
        let mut buffer = JsonStreamingBuffer::new();
        
        // Add object in fragments
        let objects1 = buffer.add_chunk(r#"{"id": "test", "#);
        assert_eq!(objects1.len(), 0); // No complete objects yet
        
        let objects2 = buffer.add_chunk(r#""content": "hello"}"#);
        assert_eq!(objects2.len(), 1);
        // This should be processed as a Message chunk
        match &objects2[0] {
            ProcessedChunk::Message(json_val) => {
                assert_eq!(json_val.get("id").and_then(|v| v.as_str()), Some("test"));
                assert_eq!(json_val.get("content").and_then(|v| v.as_str()), Some("hello"));
            }
            _ => panic!("Expected Message chunk"),
        }
    }

    #[test]
    fn test_multiple_objects() {
        let mut buffer = JsonStreamingBuffer::new();
        
        // Add multiple objects in one chunk
        let objects = buffer.add_chunk(r#"{"id": "1"}{"id": "2"}{"id": "3"}"#);
        assert_eq!(objects.len(), 3);
        for (i, obj) in objects.iter().enumerate() {
            match obj {
                ProcessedChunk::Message(json_val) => {
                    let expected_id = (i + 1).to_string();
                    assert_eq!(json_val.get("id").and_then(|v| v.as_str()), Some(expected_id.as_str()));
                }
                _ => panic!("Expected Message chunk"),
            }
        }
    }

    #[test]
    fn test_nested_objects() {
        let mut buffer = JsonStreamingBuffer::new();
        
        let objects = buffer.add_chunk(r#"{"outer": {"inner": {"deep": "value"}}}"#);
        assert_eq!(objects.len(), 1);
        match &objects[0] {
            ProcessedChunk::Message(json_val) => {
                assert!(json_val.get("outer").is_some());
            }
            _ => panic!("Expected Message chunk"),
        }
    }

    #[test]
    fn test_strings_with_braces() {
        let mut buffer = JsonStreamingBuffer::new();
        
        let objects = buffer.add_chunk(r#"{"content": "This has {braces} in string"}"#);
        assert_eq!(objects.len(), 1);
        match &objects[0] {
            ProcessedChunk::Message(json_val) => {
                assert_eq!(json_val.get("content").and_then(|v| v.as_str()), Some("This has {braces} in string"));
            }
            _ => panic!("Expected Message chunk"),
        }
    }

    #[test]
    fn test_vertex_ai_complete_message() {
        let mut buffer = JsonStreamingBuffer::new();
        
        // Test with a Vertex AI complete message format
        let vertex_message = r#"{"id":"msg_test","type":"message","role":"assistant","content":[{"type":"text","text":"Hello world"}],"stop_reason":"end_turn"}"#;
        let objects = buffer.add_chunk(vertex_message);
        assert_eq!(objects.len(), 1);
        match &objects[0] {
            ProcessedChunk::Content(text) => {
                assert_eq!(text, "Hello world");
            }
            _ => panic!("Expected Content chunk with extracted text"),
        }
    }

    #[test]
    fn test_streaming_content_block_delta() {
        let mut buffer = JsonStreamingBuffer::new();
        
        // Test with streaming content block delta
        let delta_chunk = r#"{"type":"content_block_delta","delta":{"type":"text","text":"Hello "}}"#;
        let objects = buffer.add_chunk(delta_chunk);
        assert_eq!(objects.len(), 1);
        match &objects[0] {
            ProcessedChunk::Content(text) => {
                assert_eq!(text, "Hello ");
            }
            _ => panic!("Expected Content chunk"),
        }
    }

    #[test]
    fn test_finalize_incomplete() {
        let mut buffer = JsonStreamingBuffer::new();
        
        buffer.add_chunk(r#"{"incomplete": "obj"#);
        let remaining = buffer.finalize();
        assert!(remaining.is_none()); // Invalid JSON should be discarded
    }
}
