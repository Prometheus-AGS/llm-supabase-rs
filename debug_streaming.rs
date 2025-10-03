// Debug script to test streaming directly
//
// Run with: cargo run --bin debug_streaming

use std::io::{self, Write};
use reqwest::Client;
use serde_json::{json, Value};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test streaming directly to Vertex AI (if we had credentials)
    println!("Testing streaming response...");
    
    // Test our service instead
    let client = Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-key")
        .json(&json!({
            "model": "claude-sonnet-4-5@20250929",
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": "Hello! Please say hello back."
                }
            ]
        }))
        .send()
        .await?;

    println!("Response status: {}", response.status());
    println!("Response headers: {:?}", response.headers());
    
    let mut stream = response.bytes_stream();
    let mut chunk_count = 0;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                chunk_count += 1;
                let chunk_str = String::from_utf8_lossy(&chunk);
                println!("Chunk {}: {}", chunk_count, chunk_str);
                io::stdout().flush().unwrap();
            }
            Err(e) => {
                println!("Error reading chunk: {}", e);
                break;
            }
        }
    }
    
    println!("Total chunks received: {}", chunk_count);
    Ok(())
}
