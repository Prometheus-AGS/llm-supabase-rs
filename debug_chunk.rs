// Quick debug script to test chunk parsing
// Run with: cargo run --bin debug_chunk

use llm_supabase_rs::infrastructure::vertex::VertexStreamChunk;

fn main() {
    // Test parsing a "message" type chunk that might be coming from Vertex AI
    let test_jsons = vec![
        r#"{"type": "message", "content": "Hello world"}"#,
        r#"{"type": "message", "text": "Hello world"}"#,
        r#"{"type": "message", "delta": {"text": "Hello world"}}"#,
        r#"{"type": "message", "message": {"content": "Hello world"}}"#,
    ];

    for (i, json) in test_jsons.iter().enumerate() {
        println!("Testing JSON {}: {}", i + 1, json);
        
        match serde_json::from_str::<VertexStreamChunk>(json) {
            Ok(chunk) => {
                println!("  ✅ Parsed successfully");
                println!("  Event type: {}", chunk.event_type);
                println!("  Has delta: {}", chunk.delta.is_some());
                println!("  Content: {:?}", chunk.get_content());
            }
            Err(e) => {
                println!("  ❌ Failed to parse: {}", e);
            }
        }
        println!();
    }
}
