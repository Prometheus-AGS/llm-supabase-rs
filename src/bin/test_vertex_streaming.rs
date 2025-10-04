// Test script to debug Vertex AI streaming
// Run with: cargo run --bin test_vertex_streaming

use anyhow::Result;
use reqwest::Client;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let project_id = std::env::var("GCP_PROJECT_ID")?;
    let region = std::env::var("GCP_LOCATION").unwrap_or_else(|_| "us-east5".to_string());
    let model = "claude-sonnet-4-5@20250929";
    
    // Get access token
    let output = std::process::Command::new("gcloud")
        .args(["auth", "print-access-token"])
        .output()?;
    let access_token = String::from_utf8(output.stdout)?.trim().to_string();
    
    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/anthropic/models/{}:streamRawPredict",
        region, project_id, region, model
    );
    
    println!("Streaming URL: {}", url);
    
    let request_body = serde_json::json!({
        "anthropic_version": "vertex-2023-10-16",
        "messages": [{
            "role": "user",
            "content": "Count from 1 to 5"
        }],
        "max_tokens": 100
    });
    
    println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);
    
    let client = Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    println!("Response status: {}", response.status());
    println!("Response headers: {:#?}", response.headers());
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("Error response: {}", error_text);
        return Ok(());
    }
    
    // Stream the response
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut chunk_count = 0;
    
    println!("\n--- Starting to receive chunks ---\n");
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                chunk_count += 1;
                let chunk_str = String::from_utf8_lossy(&bytes);
                println!("=== RAW CHUNK {} ({} bytes) ===", chunk_count, bytes.len());
                println!("{}", chunk_str);
                println!("=== END CHUNK {} ===\n", chunk_count);
                
                buffer.push_str(&chunk_str);
                
                // Try to parse SSE events
                while let Some(double_newline_pos) = buffer.find("\n\n") {
                    let event = buffer[..double_newline_pos].to_string();
                    buffer = buffer[double_newline_pos + 2..].to_string();
                    
                    if event.trim().is_empty() {
                        continue;
                    }
                    
                    println!(">>> PARSED SSE EVENT:");
                    for line in event.lines() {
                        println!("    {}", line);
                    }
                    
                    // Try to extract and parse JSON data
                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim() != "[DONE]" {
                                match serde_json::from_str::<serde_json::Value>(data) {
                                    Ok(json) => {
                                        println!("    Parsed JSON:");
                                        println!("    {}", serde_json::to_string_pretty(&json)?);
                                    }
                                    Err(e) => {
                                        println!("    Failed to parse JSON: {}", e);
                                        println!("    Raw data: {}", data);
                                    }
                                }
                            }
                        }
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Error receiving chunk: {}", e);
                break;
            }
        }
    }
    
    println!("\n--- Stream ended after {} chunks ---", chunk_count);
    
    if !buffer.is_empty() {
        println!("\nRemaining buffer content:");
        println!("{}", buffer);
    }
    
    Ok(())
}
