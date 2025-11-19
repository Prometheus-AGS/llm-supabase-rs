// tests/integration/test_diff_patch_integration.rs
//
// Integration tests for the unified diff and patch system

use llm_supabase_rs::features::diff_patch::{
    generate_diff, apply_patch, generate_codex_diff, DiffPatchManager, utils::quick_diff,
};
use llm_supabase_rs::features::shell_execution::{ShellExecutionManager, ShellConfig, CommandFactory};
use llm_supabase_rs::infrastructure::common::tools::{EnhancedToolCallManager, OpenAIToolConverter};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_basic_diff_generation() {
    let old_content = "line 1\nline 2\nline 3\n";
    let new_content = "line 1\nmodified line 2\nline 3\nline 4\n";
    
    let diff = generate_diff(old_content, new_content, "test.txt").unwrap();
    
    assert!(diff.has_changes());
    assert_eq!(diff.additions(), 2); // modified line + new line  
    assert_eq!(diff.deletions(), 1); // original line 2
    assert!(!diff.metadata.is_binary);
}

#[tokio::test]
async fn test_basic_patch_application() {
    let original = "line 1\nline 2\nline 3\n";
    let new_content = "line 1\nmodified line 2\nline 3\nline 4\n";
    
    // Generate diff
    let diff = generate_diff(original, new_content, "test.txt").unwrap();
    
    // Apply patch
    let result = apply_patch(original, &diff).unwrap();
    
    assert_eq!(result, new_content);
}

#[tokio::test]
async fn test_codex_cli_format_generation() {
    let old_content = "fn main() {\n    println!(\"Hello\");\n}\n";
    let new_content = "fn main() {\n    println!(\"Hello, World!\");\n    println!(\"Rust is awesome!\");\n}\n";
    
    let codex_diff = generate_codex_diff(old_content, new_content, "src/main.rs").unwrap();
    
    assert!(codex_diff.contains("*** Begin Patch"));
    assert!(codex_diff.contains("*** Update File: src/main.rs"));
    assert!(codex_diff.contains("*** End Patch"));
    assert!(codex_diff.contains("-    println!(\"Hello\");"));
    assert!(codex_diff.contains("+    println!(\"Hello, World!\");"));
    assert!(codex_diff.contains("+    println!(\"Rust is awesome!\");"));
}

#[tokio::test]
async fn test_file_operations() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    
    // Create initial file
    let initial_content = "line 1\nline 2\nline 3\n";
    fs::write(&file_path, initial_content).unwrap();
    
    // Generate diff
    let new_content = "line 1\nmodified line 2\nline 3\nline 4\n";
    let diff = generate_diff(initial_content, new_content, file_path.to_str().unwrap()).unwrap();
    
    // Apply patch to file
    let manager = DiffPatchManager::default();
    manager.apply_patch_to_file(file_path.to_str().unwrap(), &diff).unwrap();
    
    // Verify result
    let final_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(final_content, new_content);
}

#[tokio::test]
async fn test_shell_command_execution() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("shell_test.txt");
    
    // Create initial file
    let initial_content = "original content\n";
    fs::write(&file_path, initial_content).unwrap();
    
    // Create a shell execution manager
    let shell_manager = ShellExecutionManager::default();
    
    // Test file reading
    let read_command = CommandFactory::file_operation(
        llm_supabase_rs::features::shell_execution::commands::FileOperation::Read,
        file_path.to_str().unwrap().to_string(),
        None,
    );
    
    let dummy_executor = llm_supabase_rs::features::shell_execution::ShellExecutor::default();
    let result = read_command.execute(&dummy_executor).await.unwrap();
    
    assert!(result.success);
    assert_eq!(result.stdout.trim(), "original content");
}

#[tokio::test]
async fn test_tool_call_integration() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("tool_test.txt");
    
    // Create initial file
    let initial_content = "function test() {\n    console.log('old');\n}\n";
    fs::write(&file_path, initial_content).unwrap();
    
    // Create codex-style patch
    let new_content = "function test() {\n    console.log('new and improved');\n    console.log('additional line');\n}\n";
    let codex_patch = generate_codex_diff(initial_content, new_content, file_path.to_str().unwrap()).unwrap();
    
    // Create tool call in expected format
    let tool_call_json = json!({
        "cmd": ["apply_patch", codex_patch]
    });
    
    // Create shell execution manager and execute
    let shell_manager = ShellExecutionManager::default();
    let result = shell_manager.execute_from_tool_call(&tool_call_json).await.unwrap();
    
    assert!(result.success, "Tool call failed: {}", result.stderr);
    
    // Verify file was updated
    let updated_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated_content, new_content);
}

#[tokio::test]
async fn test_error_handling() {
    // Test file not found error
    let diff = generate_diff("old", "new", "nonexistent.txt").unwrap();
    let result = llm_supabase_rs::features::diff_patch::apply_patch_to_file(
        "nonexistent.txt", 
        &diff, 
        &llm_supabase_rs::features::diff_patch::PatchConfig::default()
    );
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, llm_supabase_rs::features::diff_patch::PatchError::FileNotFound { .. }));
}

#[tokio::test]
async fn test_security_validation() {
    // Test directory traversal protection
    let shell_manager = ShellExecutionManager::default();
    
    let malicious_tool_call = json!({
        "cmd": ["apply_patch", "*** Begin Patch\n*** Update File: ../../../etc/passwd\n--- a/etc/passwd\n+++ b/etc/passwd\n@@ -1 +1 @@\n-root:x:0:0:root:/root:/bin/bash\n+hacker:x:0:0:hacker:/root:/bin/bash\n*** End Patch"]
    });
    
    let result = shell_manager.execute_from_tool_call(&malicious_tool_call).await;
    
    // Should fail due to security validation
    assert!(result.is_err() || !result.unwrap().success);
}

#[tokio::test]
async fn test_binary_file_detection() {
    let binary_content = vec![0u8, 1, 2, 3, 255, 254, 253];
    let text_content = "This is normal text content\n";
    
    // Test with binary content (should be detected as binary)
    let binary_string = String::from_utf8_lossy(&binary_content);
    let diff = generate_diff(&binary_string, text_content, "test.bin").unwrap();
    
    // Binary files should have no changes recorded (handled as binary)
    assert!(diff.metadata.is_binary || !diff.has_changes());
}

#[tokio::test]
async fn test_large_diff_handling() {
    // Create large content
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();
    
    for i in 0..1000 {
        old_lines.push(format!("line {}", i));
        if i == 500 {
            new_lines.push("MODIFIED LINE 500".to_string());
        } else {
            new_lines.push(format!("line {}", i));
        }
    }
    
    let old_content = old_lines.join("\n");
    let new_content = new_lines.join("\n");
    
    let diff = generate_diff(&old_content, &new_content, "large_file.txt").unwrap();
    
    assert!(diff.has_changes());
    assert_eq!(diff.additions(), 1);
    assert_eq!(diff.deletions(), 1);
    
    // Test patch application
    let result = apply_patch(&old_content, &diff).unwrap();
    assert_eq!(result, new_content);
}

#[tokio::test]
async fn test_context_lines_configuration() {
    let old_content = "line1\nline2\nline3\nline4\nline5\nline6\nline7\n";
    let new_content = "line1\nline2\nline3\nMODIFIED\nline5\nline6\nline7\n";
    
    // Test with different context line settings
    let config_small = llm_supabase_rs::features::diff_patch::DiffConfig { 
        context_lines: 1, 
        ..Default::default() 
    };
    let config_large = llm_supabase_rs::features::diff_patch::DiffConfig { 
        context_lines: 3, 
        ..Default::default() 
    };
    
    let diff_small = llm_supabase_rs::features::diff_patch::generate_diff_with_config(
        old_content, new_content, "test.txt", &config_small
    ).unwrap();
    
    let diff_large = llm_supabase_rs::features::diff_patch::generate_diff_with_config(
        old_content, new_content, "test.txt", &config_large
    ).unwrap();
    
    // Both should have changes, but different context amounts
    assert!(diff_small.has_changes());
    assert!(diff_large.has_changes());
    
    // Large context should have more context lines
    let small_context_ops = diff_small.chunks[0].operations.iter()
        .filter(|op| matches!(op, llm_supabase_rs::features::diff_patch::PatchOperation::Context { .. }))
        .count();
    let large_context_ops = diff_large.chunks[0].operations.iter()
        .filter(|op| matches!(op, llm_supabase_rs::features::diff_patch::PatchOperation::Context { .. }))
        .count();
    
    assert!(large_context_ops >= small_context_ops);
}

#[tokio::test]
async fn test_fuzzy_patch_matching() {
    let original = "line 1\nline 2 with extra spaces   \nline 3\n";
    let modified = "line 1\nline 2 with extra spaces\nline 3\nline 4\n"; // Slight whitespace difference
    
    // Create diff with exact match
    let diff = generate_diff(original.trim_end(), modified, "test.txt").unwrap();
    
    // Apply to content with slight differences (fuzzy matching should handle this)
    let config = llm_supabase_rs::features::diff_patch::PatchConfig {
        fuzzy_matching: true,
        max_fuzz: 2,
        ..Default::default()
    };
    
    let result = llm_supabase_rs::features::diff_patch::apply_patch_with_config(
        original, &diff, &config
    );
    
    // Should succeed with fuzzy matching
    assert!(result.is_ok());
}

#[tokio::test] 
async fn test_backup_and_rollback() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("backup_test.txt");
    
    let original_content = "original content\n";
    fs::write(&file_path, original_content).unwrap();
    
    // Create a diff
    let new_content = "modified content\n";
    let diff = generate_diff(original_content, new_content, file_path.to_str().unwrap()).unwrap();
    
    // Apply patch with backup enabled
    let config = llm_supabase_rs::features::diff_patch::PatchConfig {
        create_backup: true,
        backup_extension: ".backup".to_string(),
        ..Default::default()
    };
    
    llm_supabase_rs::features::diff_patch::apply_patch_to_file(
        file_path.to_str().unwrap(), &diff, &config
    ).unwrap();
    
    // Verify backup exists
    let backup_path = format!("{}.backup", file_path.to_str().unwrap());
    assert!(std::path::Path::new(&backup_path).exists());
    
    // Verify backup content
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, original_content);
    
    // Verify file was modified
    let updated_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated_content, new_content);
    
    // Test rollback
    llm_supabase_rs::features::diff_patch::rollback_patch(
        file_path.to_str().unwrap(), 
        ".backup"
    ).unwrap();
    
    // Verify rollback worked
    let rolled_back_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(rolled_back_content, original_content);
}

#[tokio::test]
async fn test_quick_utilities() {
    let old = "hello\nworld\n";
    let new = "hello\nrustacean\n";
    
    // Test quick diff utility
    let diff_text = quick_diff(old, new, "test.txt").unwrap();
    assert!(diff_text.contains("-world"));
    assert!(diff_text.contains("+rustacean"));
    
    // Test quick patch utility
    let patched = llm_supabase_rs::features::diff_patch::utils::quick_patch(old, &diff_text).unwrap();
    assert_eq!(patched, new);
}