// src/features/shell_execution/mod.rs
//
// Secure shell command execution for patch operations

pub mod executor;
pub mod security;
pub mod commands;

// Re-export main types
pub use executor::{ShellExecutor, ExecutionResult, ExecutionConfig};
pub use security::{SecurityPolicy, validate_command, sanitize_path};
pub use commands::{ApplyPatchCommand, ShellCommand};

use crate::features::diff_patch::models::PatchError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for shell execution environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Maximum execution time for commands
    pub timeout: Duration,
    /// Working directory for command execution
    pub working_directory: PathBuf,
    /// Environment variables to set
    pub environment: HashMap<String, String>,
    /// Whether to capture stdout
    pub capture_stdout: bool,
    /// Whether to capture stderr
    pub capture_stderr: bool,
    /// Maximum output size in bytes
    pub max_output_size: usize,
    /// Security policy for command validation
    pub security_policy: SecurityPolicy,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            environment: HashMap::new(),
            capture_stdout: true,
            capture_stderr: true,
            max_output_size: 1024 * 1024, // 1MB
            security_policy: SecurityPolicy::default(),
        }
    }
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Exit status code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether the command was successful
    pub success: bool,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

impl CommandResult {
    /// Create a successful result
    pub fn success(stdout: String, stderr: String, duration: Duration) -> Self {
        Self {
            exit_code: 0,
            stdout,
            stderr,
            duration,
            success: true,
            error_message: None,
        }
    }

    /// Create a failed result
    pub fn failure(
        exit_code: i32,
        stdout: String,
        stderr: String,
        duration: Duration,
        error: Option<String>,
    ) -> Self {
        Self {
            exit_code,
            stdout,
            stderr,
            duration,
            success: false,
            error_message: error,
        }
    }

    /// Create an error result
    pub fn error(message: String) -> Self {
        Self {
            exit_code: -1,
            stdout: String::new(),
            stderr: message.clone(),
            duration: Duration::from_secs(0),
            success: false,
            error_message: Some(message),
        }
    }
}

/// Shell execution manager for handling different types of commands
pub struct ShellExecutionManager {
    config: ShellConfig,
    executor: ShellExecutor,
}

impl ShellExecutionManager {
    /// Create a new shell execution manager
    pub fn new(config: ShellConfig) -> Self {
        let executor = ShellExecutor::new(ExecutionConfig::from_shell_config(&config));
        Self { config, executor }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ShellConfig::default())
    }

    /// Execute a shell command with security validation
    pub async fn execute_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<CommandResult, PatchError> {
        // Validate command against security policy
        validate_command(command, args, &self.config.security_policy)?;

        // Execute the command
        self.executor.execute(command, args).await
    }

    /// Execute an apply_patch command specifically
    pub async fn execute_apply_patch(
        &self,
        patch_content: &str,
        target_file: &str,
    ) -> Result<CommandResult, PatchError> {
        let command = ApplyPatchCommand::new(patch_content.to_string(), target_file.to_string());
        command.execute(&self.executor).await
    }

    /// Execute a command from JSON tool call format
    pub async fn execute_from_tool_call(
        &self,
        tool_call_args: &serde_json::Value,
    ) -> Result<CommandResult, PatchError> {
        // Parse the tool call arguments
        if let Some(cmd_array) = tool_call_args.get("cmd").and_then(|c| c.as_array()) {
            if cmd_array.is_empty() {
                return Err(PatchError::InvalidPatchFormat {
                    message: "Empty command array".to_string(),
                });
            }

            let command = cmd_array[0].as_str()
                .ok_or_else(|| PatchError::InvalidPatchFormat {
                    message: "Command must be a string".to_string(),
                })?;

            let args: Vec<String> = cmd_array[1..].iter()
                .map(|arg| arg.as_str().unwrap_or("").to_string())
                .collect();

            // Handle special case for apply_patch
            if command == "apply_patch" && args.len() == 1 {
                return self.parse_and_apply_patch(&args[0]).await;
            }

            // Execute regular command
            self.execute_command(command, &args).await
        } else {
            Err(PatchError::InvalidPatchFormat {
                message: "Invalid tool call format: missing 'cmd' array".to_string(),
            })
        }
    }

    /// Parse and apply a patch from Codex CLI format
    async fn parse_and_apply_patch(&self, patch_text: &str) -> Result<CommandResult, PatchError> {
        // Parse Codex CLI patch format
        let patch_info = self.parse_codex_patch(patch_text)?;
        
        // Execute the patch application
        self.execute_apply_patch(&patch_info.patch_content, &patch_info.target_file).await
    }

    /// Parse Codex CLI patch format
    fn parse_codex_patch(&self, patch_text: &str) -> Result<CodexPatchInfo, PatchError> {
        let lines: Vec<&str> = patch_text.lines().collect();
        
        // Look for Codex CLI markers
        let mut in_patch = false;
        let mut target_file = None;
        let mut patch_lines = Vec::new();

        for line in lines {
            if line == "*** Begin Patch" {
                in_patch = true;
                continue;
            }
            
            if line == "*** End Patch" {
                break;
            }
            
            if line.starts_with("*** Update File: ") {
                target_file = Some(line[17..].trim().to_string());
                continue;
            }
            
            if in_patch && target_file.is_some() {
                patch_lines.push(line);
            }
        }

        let target_file = target_file.ok_or_else(|| PatchError::InvalidPatchFormat {
            message: "No target file specified in patch".to_string(),
        })?;

        let patch_content = patch_lines.join("\n");
        
        Ok(CodexPatchInfo {
            target_file,
            patch_content,
        })
    }

    /// Set working directory
    pub fn set_working_directory(&mut self, path: PathBuf) {
        self.config.working_directory = path;
        self.executor.update_config(ExecutionConfig::from_shell_config(&self.config));
    }

    /// Add environment variable
    pub fn add_environment_variable(&mut self, key: String, value: String) {
        self.config.environment.insert(key, value);
        self.executor.update_config(ExecutionConfig::from_shell_config(&self.config));
    }

    /// Update security policy
    pub fn update_security_policy(&mut self, policy: SecurityPolicy) {
        self.config.security_policy = policy;
    }
}

/// Information parsed from Codex CLI patch format
#[derive(Debug, Clone)]
struct CodexPatchInfo {
    target_file: String,
    patch_content: String,
}

impl ExecutionConfig {
    /// Create from shell config
    fn from_shell_config(shell_config: &ShellConfig) -> Self {
        Self {
            timeout: shell_config.timeout,
            working_directory: shell_config.working_directory.clone(),
            environment: shell_config.environment.clone(),
            capture_stdout: shell_config.capture_stdout,
            capture_stderr: shell_config.capture_stderr,
            max_output_size: shell_config.max_output_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_shell_config_default() {
        let config = ShellConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);
        assert_eq!(config.max_output_size, 1024 * 1024);
    }

    #[test]
    fn test_command_result_creation() {
        let success = CommandResult::success(
            "output".to_string(),
            "".to_string(),
            Duration::from_millis(100),
        );
        assert!(success.success);
        assert_eq!(success.exit_code, 0);

        let failure = CommandResult::failure(
            1,
            "".to_string(),
            "error".to_string(),
            Duration::from_millis(100),
            Some("Command failed".to_string()),
        );
        assert!(!failure.success);
        assert_eq!(failure.exit_code, 1);
    }

    #[test]
    fn test_parse_codex_patch() {
        let manager = ShellExecutionManager::default();
        let patch_text = r#"*** Begin Patch
*** Update File: src/test.rs
--- a/src/test.rs
+++ b/src/test.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World");
+    println!("New line");
 }
*** End Patch"#;

        let patch_info = manager.parse_codex_patch(patch_text).unwrap();
        assert_eq!(patch_info.target_file, "src/test.rs");
        assert!(patch_info.patch_content.contains("--- a/src/test.rs"));
        assert!(patch_info.patch_content.contains("+++ b/src/test.rs"));
    }

    #[test]
    fn test_invalid_codex_patch() {
        let manager = ShellExecutionManager::default();
        let invalid_patch = "Not a valid patch format";
        
        let result = manager.parse_codex_patch(invalid_patch);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_call_parsing() {
        let manager = ShellExecutionManager::default();
        
        // Test invalid tool call format
        let invalid_call = serde_json::json!({
            "not_cmd": ["ls", "-la"]
        });
        
        let result = manager.execute_from_tool_call(&invalid_call).await;
        assert!(result.is_err());
        
        // Test empty command array
        let empty_call = serde_json::json!({
            "cmd": []
        });
        
        let result = manager.execute_from_tool_call(&empty_call).await;
        assert!(result.is_err());
    }
}