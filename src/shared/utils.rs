use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    format!("chatcmpl-{}", Uuid::new_v4().simple())
}

/// Generate a unique tool call ID
pub fn generate_tool_call_id() -> String {
    format!("call_{}", Uuid::new_v4().simple())
}

/// Get current Unix timestamp
pub fn current_timestamp() -> i64 {
    Utc::now().timestamp()
}

/// Decode base64 image data
pub fn decode_base64_image(data_url: &str) -> anyhow::Result<(String, Vec<u8>)> {
    if let Some(comma_pos) = data_url.find(',') {
        let header = &data_url[..comma_pos];
        let data = &data_url[comma_pos + 1..];

        // Extract MIME type from data URL
        let mime_type = if header.contains("image/png") {
            "image/png"
        } else if header.contains("image/jpeg") || header.contains("image/jpg") {
            "image/jpeg"
        } else if header.contains("image/gif") {
            "image/gif"
        } else if header.contains("image/webp") {
            "image/webp"
        } else {
            return Err(anyhow::anyhow!("Unsupported image format"));
        };

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64: {}", e))?;

        Ok((mime_type.to_string(), decoded))
    } else {
        Err(anyhow::anyhow!("Invalid data URL format"))
    }
}

/// Encode binary data to base64
pub fn encode_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Extract text content from message parts
pub fn extract_text_from_message(content: &crate::shared::MessageContent) -> String {
    match content {
        crate::shared::MessageContent::Text(text) => text.clone(),
        crate::shared::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| part.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Check if message contains images
pub fn message_has_images(content: &crate::shared::MessageContent) -> bool {
    match content {
        crate::shared::MessageContent::Text(_) => false,
        crate::shared::MessageContent::Parts(parts) => {
            parts.iter().any(|part| part.image_url.is_some())
        }
    }
}

/// Extract image URLs from message
pub fn extract_image_urls(content: &crate::shared::MessageContent) -> Vec<String> {
    match content {
        crate::shared::MessageContent::Text(_) => vec![],
        crate::shared::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| part.image_url.as_ref())
            .map(|img| img.url.clone())
            .collect(),
    }
}

/// Sanitize model name for logging
pub fn sanitize_model_name(model: &str) -> String {
    model
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect()
}

/// Calculate token usage estimate (rough approximation)
pub fn estimate_tokens(text: &str) -> u32 {
    // Very rough estimate: ~4 characters per token
    (text.len() as f32 / 4.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id = generate_request_id();
        assert!(id.starts_with("chatcmpl-"));
        assert_eq!(id.len(), "chatcmpl-".len() + 32); // UUID simple format is 32 chars
    }

    #[test]
    fn test_decode_base64_image() {
        let data_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
        let result = decode_base64_image(data_url);
        assert!(result.is_ok());
        let (mime_type, _data) = result.unwrap();
        assert_eq!(mime_type, "image/png");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars / 4 = 1.25 -> 2
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars / 4 = 2.75 -> 3
    }
}
