use std::sync::Arc;
use std::pin::Pin;
use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{debug, info, warn};
use serde::de::DeserializeOwned;
use tokio_stream::Stream;

use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
use super::converter::ProviderConverter;
use super::types::ProviderConfig;
use super::tools::ToolCallManager;

/// Generic LLM client that works with any provider implementing the necessary traits
pub struct GenericLLMClient<C, Conf> {
    /// HTTP client
    client: Client,
    /// Provider configuration
    config: Conf,
    /// Format converter
    converter: Arc<C>,
    /// Tool manager
    tool_manager: Arc<ToolCallManager>,
}

impl<C, Conf> GenericLLMClient<C, Conf>
where
    C: ProviderConverter + Send + Sync + 'static,
    C::ProviderRequest: serde::Serialize,
    C::ProviderResponse: DeserializeOwned,
    Conf: ProviderConfig + Clone + Send + Sync,
{
    /// Create a new generic client
    pub fn new(
        config: Conf,
        converter: C,
        tool_manager: ToolCallManager,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // Default timeout, should come from config
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            converter: Arc::new(converter),
            tool_manager: Arc::new(tool_manager),
        })
    }

    /// Make a chat completion request
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let model = request.model.clone();
        debug!("Making {} chat completion request to model: {}", self.converter.provider_name(), model);

        // Convert request
        let provider_req = self.converter.openai_to_provider_request(&request)
            .context("Failed to convert request to provider format")?;

        let url = self.config.base_url();
        let auth_headers = self.config.auth_headers();

        // Build header map
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json")
        );
        
        for (k, v) in auth_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| anyhow::anyhow!("Invalid header name '{}': {}", k, e))?;
            let header_value = reqwest::header::HeaderValue::from_str(&v)
                .map_err(|e| anyhow::anyhow!("Invalid header value for '{}': {}", k, e))?;
            headers.insert(header_name, header_value);
        }

        // Send request
        let response = self.client.post(&url)
            .headers(headers)
            .json(&provider_req)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.converter.parse_error(status.as_u16(), &error_text));
        }

        // Parse response
        let provider_res: C::ProviderResponse = response.json().await
            .context("Failed to parse provider response")?;
        
        let request_id = uuid::Uuid::new_v4().to_string(); // Fallback

        self.converter.provider_to_openai_response(
            &provider_res,
            &request_id,
            &model,
            &request
        ).context("Failed to convert response to OpenAI format")
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, anyhow::Error>> + Send>>> 
    where
        C: super::converter::StreamingConverter,
    {
        let model = request.model.clone();
        debug!("Making {} streaming chat completion request to model: {}", self.converter.provider_name(), model);

        // Convert request
        let provider_req = self.converter.openai_to_provider_streaming_request(&request)
            .context("Failed to convert streaming request")?;

        let url = self.config.base_url();
        let auth_headers = self.config.auth_headers();

        // Build header map
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json")
        );
        
        for (k, v) in auth_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| anyhow::anyhow!("Invalid header name '{}': {}", k, e))?;
            let header_value = reqwest::header::HeaderValue::from_str(&v)
                .map_err(|e| anyhow::anyhow!("Invalid header value for '{}': {}", k, e))?;
            headers.insert(header_name, header_value);
        }

        // Send request
        let response = self.client.post(&url)
            .headers(headers)
            .json(&provider_req)
            .send()
            .await
            .context("Failed to send streaming request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.converter.parse_error(status.as_u16(), &error_text));
        }

        let request_id = uuid::Uuid::new_v4().to_string();

        // Convert to stream
        let provider_stream = self.converter.response_to_stream(response).await
            .context("Failed to convert response to stream")?;

        // Process stream
        let processed_stream = self.converter.process_stream(provider_stream, request_id, model).await;
        Ok(processed_stream)
    }

    /// Get the tool manager
    pub fn tool_manager(&self) -> &ToolCallManager {
        &self.tool_manager
    }
    
    /// Get the configuration
    pub fn config(&self) -> &Conf {
        &self.config
    }

    /// Get the HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &C {
        &self.converter
    }
}
