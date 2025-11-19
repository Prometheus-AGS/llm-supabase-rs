// src/features/shell_execution/commands.rs
//
// Specific command implementations for patch operations

use crate::features::diff_patch::models::PatchError;
use crate::features::diff_patch::{parse_unified_diff, apply_patch_to_file, PatchConfig};
use crate::features::shell_execution::{ShellExecutor, CommandResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

/// Trait for shell commands that can be executed
#[async_trait::async_trait]
pub trait ShellCommand {
    /// Execute the command using the provided executor
    async fn execute(&self, executor: &ShellExecutor) -> Result<CommandResult, PatchError>;
    
    /// Get the command name
    fn name(&self) -> &str;
    
    /// Get command arguments
    fn args(&self) -> Vec<String>;
    
    /// Validate the command before execution
    fn validate(&self) -> Result<(), PatchError> {
        Ok(())
    }
}

/// Apply patch command implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchCommand {
    /// The patch content to apply
    pub patch_content: String,
    /// Target file to patch
    pub target_file: String,
    /// Configuration for patch application
    pub config: PatchConfig,
}

impl ApplyPatchCommand {
    /// Create a new apply patch command
    pub fn new(patch_content: String, target_file: String) -> Self {
        Self {
            patch_content,
            target_file,
            config: PatchConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(patch_content: String, target_file: String, config: PatchConfig) -> Self {
        Self {
            patch_content,
            target_file,
            config,
        }
    }

    /// Apply the patch directly without shell execution
    pub async fn apply_direct(&self) -> Result<CommandResult, PatchError> {
        let start_time = std::time::Instant::now();
        
        // Validate target file exists
        if !Path::new(&self.target_file).exists() {
            return Ok(CommandResult::failure(
                1,
                String::new(),
                format!("Target file does not exist: {}", self.target_file),
                start_time.elapsed(),
                Some("File not found".to_string()),
            ));
        }

        // Parse the unified diff
        let diffs = match parse_unified_diff(&self.patch_content) {
            Ok(diffs) => diffs,
            Err(e) => {
                return Ok(CommandResult::failure(
                    1,
                    String::new(),
                    format!("Failed to parse patch: {}", e),
                    start_time.elapsed(),
                    Some(e.to_string()),
                ));
            }
        };

        if diffs.is_empty() {
            return Ok(CommandResult::success(
                "No changes to apply".to_string(),
                String::new(),
                start_time.elapsed(),
            ));
        }

        // Apply the patch
        for diff in diffs {
            if diff.metadata.new_path == self.target_file || diff.metadata.old_path == self.target_file {
                match apply_patch_to_file(&self.target_file, &diff, &self.config) {
                    Ok(()) => {
                        return Ok(CommandResult::success(
                            format!("Successfully applied patch to {}", self.target_file),
                            String::new(),
                            start_time.elapsed(),
                        ));
                    }
                    Err(e) => {
                        return Ok(CommandResult::failure(
                            1,
                            String::new(),
                            format!("Failed to apply patch: {}", e),
                            start_time.elapsed(),
                            Some(e.to_string()),
                        ));
                    }
                }
            }
        }

        Ok(CommandResult::failure(
            1,
            String::new(),
            format!("No matching diff found for file: {}", self.target_file),
            start_time.elapsed(),
            Some("No matching diff".to_string()),
        ))
    }
}

#[async_trait::async_trait]
impl ShellCommand for ApplyPatchCommand {
    async fn execute(&self, executor: &ShellExecutor) -> Result<CommandResult, PatchError> {
        // For apply_patch, we implement it directly rather than using system patch command
        // This gives us better control and cross-platform compatibility
        self.apply_direct().await
    }

    fn name(&self) -> &str {
        "apply_patch"
    }

    fn args(&self) -> Vec<String> {
        vec![self.target_file.clone()]
    }

    fn validate(&self) -> Result<(), PatchError> {
        // Validate target file path
        crate::features::shell_execution::security::sanitize_path(&self.target_file)?;
        
        // Check if patch content is not empty
        if self.patch_content.trim().is_empty() {
            return Err(PatchError::InvalidPatchFormat {
                message: "Patch content cannot be empty".to_string(),
            });
        }

        // Basic validation of patch format
        if !self.patch_content.contains("---") || !self.patch_content.contains("+++") {
            return Err(PatchError::InvalidPatchFormat {
                message: "Invalid unified diff format".to_string(),
            });
        }

        Ok(())
    }
}

/// Generic shell command for executing arbitrary commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericShellCommand {
    /// Command name
    pub command: String,
    /// Command arguments
    pub arguments: Vec<String>,
    /// Working directory (optional)
    pub working_directory: Option<String>,
}

impl GenericShellCommand {
    /// Create a new generic shell command
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            command,
            arguments,
            working_directory: None,
        }
    }

    /// Set working directory
    pub fn with_working_directory(mut self, directory: String) -> Self {
        self.working_directory = Some(directory);
        self
    }
}

#[async_trait::async_trait]
impl ShellCommand for GenericShellCommand {
    async fn execute(&self, executor: &ShellExecutor) -> Result<CommandResult, PatchError> {
        executor.execute(&self.command, &self.arguments).await
    }

    fn name(&self) -> &str {
        &self.command
    }

    fn args(&self) -> Vec<String> {
        self.arguments.clone()
    }

    fn validate(&self) -> Result<(), PatchError> {
        if self.command.is_empty() {
            return Err(PatchError::InvalidPatchFormat {
                message: "Command cannot be empty".to_string(),
            });
        }

        // Validate working directory if specified
        if let Some(ref dir) = self.working_directory {
            crate::features::shell_execution::security::sanitize_path(dir)?;
        }

        Ok(())
    }
}

/// File operation commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationCommand {
    /// Type of operation
    pub operation: FileOperation,
    /// Source path
    pub source: String,
    /// Destination path (for copy/move operations)
    pub destination: Option<String>,
    /// Additional options
    pub options: FileOperationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    /// Read file content
    Read,
    /// Write content to file
    Write { content: String },
    /// Copy file
    Copy,
    /// Move file
    Move,
    /// Create directory
    CreateDirectory,
    /// Check if file exists
    Exists,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileOperationOptions {
    /// Create parent directories if they don't exist
    pub create_parents: bool,
    /// Overwrite existing files
    pub overwrite: bool,
    /// Create backup before overwriting
    pub backup: bool,
}

impl FileOperationCommand {
    /// Create a read file command
    pub fn read(source: String) -> Self {
        Self {
            operation: FileOperation::Read,
            source,
            destination: None,
            options: FileOperationOptions::default(),
        }
    }

    /// Create a write file command
    pub fn write(destination: String, content: String) -> Self {
        Self {
            operation: FileOperation::Write { content },
            source: destination.clone(),
            destination: Some(destination),
            options: FileOperationOptions::default(),
        }
    }

    /// Create a copy file command
    pub fn copy(source: String, destination: String) -> Self {
        Self {
            operation: FileOperation::Copy,
            source,
            destination: Some(destination),
            options: FileOperationOptions::default(),
        }
    }

    /// Execute the file operation directly
    async fn execute_direct(&self) -> Result<CommandResult, PatchError> {
        let start_time = std::time::Instant::now();

        match &self.operation {
            FileOperation::Read => {
                match fs::read_to_string(&self.source) {
                    Ok(content) => Ok(CommandResult::success(
                        content,
                        String::new(),
                        start_time.elapsed(),
                    )),
                    Err(e) => Ok(CommandResult::failure(
                        1,
                        String::new(),
                        format!("Failed to read file {}: {}", self.source, e),
                        start_time.elapsed(),
                        Some(e.to_string()),
                    )),
                }
            }
            FileOperation::Write { content } => {
                let target = self.destination.as_ref().unwrap_or(&self.source);
                
                // Create parent directories if requested
                if self.options.create_parents {
                    if let Some(parent) = Path::new(target).parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            return Ok(CommandResult::failure(
                                1,
                                String::new(),
                                format!("Failed to create parent directories: {}", e),
                                start_time.elapsed(),
                                Some(e.to_string()),
                            ));
                        }
                    }
                }

                // Create backup if requested
                if self.options.backup && Path::new(target).exists() {
                    let backup_path = format!("{}.bak", target);
                    if let Err(e) = fs::copy(target, &backup_path) {
                        return Ok(CommandResult::failure(
                            1,
                            String::new(),
                            format!("Failed to create backup: {}", e),
                            start_time.elapsed(),
                            Some(e.to_string()),
                        ));
                    }
                }

                match fs::write(target, content) {
                    Ok(()) => Ok(CommandResult::success(
                        format!("Successfully wrote to {}", target),
                        String::new(),
                        start_time.elapsed(),
                    )),
                    Err(e) => Ok(CommandResult::failure(
                        1,
                        String::new(),
                        format!("Failed to write file {}: {}", target, e),
                        start_time.elapsed(),
                        Some(e.to_string()),
                    )),
                }
            }
            FileOperation::Copy => {
                let dest = self.destination.as_ref().ok_or_else(|| {
                    PatchError::InvalidPatchFormat {
                        message: "Copy operation requires destination".to_string(),
                    }
                })?;

                match fs::copy(&self.source, dest) {
                    Ok(_) => Ok(CommandResult::success(
                        format!("Successfully copied {} to {}", self.source, dest),
                        String::new(),
                        start_time.elapsed(),
                    )),
                    Err(e) => Ok(CommandResult::failure(
                        1,
                        String::new(),
                        format!("Failed to copy file: {}", e),
                        start_time.elapsed(),
                        Some(e.to_string()),
                    )),
                }
            }
            FileOperation::Move => {
                let dest = self.destination.as_ref().ok_or_else(|| {
                    PatchError::InvalidPatchFormat {
                        message: "Move operation requires destination".to_string(),
                    }
                })?;

                match fs::rename(&self.source, dest) {
                    Ok(()) => Ok(CommandResult::success(
                        format!("Successfully moved {} to {}", self.source, dest),
                        String::new(),
                        start_time.elapsed(),
                    )),
                    Err(e) => Ok(CommandResult::failure(
                        1,
                        String::new(),
                        format!("Failed to move file: {}", e),
                        start_time.elapsed(),
                        Some(e.to_string()),
                    )),
                }
            }
            FileOperation::CreateDirectory => {
                match fs::create_dir_all(&self.source) {
                    Ok(()) => Ok(CommandResult::success(
                        format!("Successfully created directory {}", self.source),
                        String::new(),
                        start_time.elapsed(),
                    )),
                    Err(e) => Ok(CommandResult::failure(
                        1,
                        String::new(),
                        format!("Failed to create directory: {}", e),
                        start_time.elapsed(),
                        Some(e.to_string()),
                    )),
                }
            }
            FileOperation::Exists => {
                let exists = Path::new(&self.source).exists();
                Ok(CommandResult::success(
                    if exists { "true" } else { "false" }.to_string(),
                    String::new(),
                    start_time.elapsed(),
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl ShellCommand for FileOperationCommand {
    async fn execute(&self, _executor: &ShellExecutor) -> Result<CommandResult, PatchError> {
        self.execute_direct().await
    }

    fn name(&self) -> &str {
        match self.operation {
            FileOperation::Read => "read_file",
            FileOperation::Write { .. } => "write_file",
            FileOperation::Copy => "copy_file",
            FileOperation::Move => "move_file",
            FileOperation::CreateDirectory => "create_directory",
            FileOperation::Exists => "file_exists",
        }
    }

    fn args(&self) -> Vec<String> {
        let mut args = vec![self.source.clone()];
        if let Some(ref dest) = self.destination {
            args.push(dest.clone());
        }
        args
    }

    fn validate(&self) -> Result<(), PatchError> {
        // Validate source path
        crate::features::shell_execution::security::sanitize_path(&self.source)?;

        // Validate destination path if present
        if let Some(ref dest) = self.destination {
            crate::features::shell_execution::security::sanitize_path(dest)?;
        }

        // Validate operation-specific requirements
        match &self.operation {
            FileOperation::Copy | FileOperation::Move => {
                if self.destination.is_none() {
                    return Err(PatchError::InvalidPatchFormat {
                        message: format!("{} operation requires destination path", self.name()),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Command factory for creating commands from different sources
pub struct CommandFactory;

impl CommandFactory {
    /// Create command from JSON tool call
    pub fn from_tool_call(tool_call: &serde_json::Value) -> Result<Box<dyn ShellCommand + Send + Sync>, PatchError> {
        if let Some(cmd_array) = tool_call.get("cmd").and_then(|c| c.as_array()) {
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

            match command {
                "apply_patch" => {
                    if args.len() != 1 {
                        return Err(PatchError::InvalidPatchFormat {
                            message: "apply_patch requires exactly one argument (patch content)".to_string(),
                        });
                    }
                    
                    // Parse Codex CLI format to extract file path and patch content
                    let patch_info = Self::parse_codex_patch_from_arg(&args[0])?;
                    
                    let command = ApplyPatchCommand::new(patch_info.patch_content, patch_info.target_file);
                    Ok(Box::new(command))
                }
                _ => {
                    let command = GenericShellCommand::new(command.to_string(), args);
                    Ok(Box::new(command))
                }
            }
        } else {
            Err(PatchError::InvalidPatchFormat {
                message: "Invalid tool call format: missing 'cmd' array".to_string(),
            })
        }
    }

    /// Parse Codex CLI patch format from argument
    fn parse_codex_patch_from_arg(arg: &str) -> Result<CodexPatchInfo, PatchError> {
        let lines: Vec<&str> = arg.lines().collect();
        
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

    /// Create file operation command
    pub fn file_operation(operation: FileOperation, source: String, destination: Option<String>) -> Box<dyn ShellCommand + Send + Sync> {
        let command = FileOperationCommand {
            operation,
            source,
            destination,
            options: FileOperationOptions::default(),
        };
        Box::new(command)
    }
}

/// Information extracted from Codex CLI patch format
#[derive(Debug, Clone)]
struct CodexPatchInfo {
    target_file: String,
    patch_content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_apply_patch_command() {
        let patch_content = r#"--- a/test.txt
+++ b/test.txt
@@ -1,2 +1,3 @@
 line 1
-line 2
+modified line 2
+new line 3"#;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\n").unwrap();

        let command = ApplyPatchCommand::new(
            patch_content.to_string(),
            file_path.to_string_lossy().to_string(),
        );

        assert!(command.validate().is_ok());
        
        let result = command.apply_direct().await.unwrap();
        assert!(result.success);

        let updated_content = fs::read_to_string(&file_path).unwrap();
        assert!(updated_content.contains("modified line 2"));
        assert!(updated_content.contains("new line 3"));
    }

    #[test]
    fn test_file_operation_command() {
        let read_cmd = FileOperationCommand::read("test.txt".to_string());
        assert_eq!(read_cmd.name(), "read_file");

        let write_cmd = FileOperationCommand::write("test.txt".to_string(), "content".to_string());
        assert_eq!(write_cmd.name(), "write_file");

        let copy_cmd = FileOperationCommand::copy("src.txt".to_string(), "dst.txt".to_string());
        assert_eq!(copy_cmd.name(), "copy_file");
        assert_eq!(copy_cmd.args().len(), 2);
    }

    #[test]
    fn test_command_factory() {
        let tool_call = serde_json::json!({
            "cmd": ["echo", "hello", "world"]
        });

        let command = CommandFactory::from_tool_call(&tool_call).unwrap();
        assert_eq!(command.name(), "echo");
        assert_eq!(command.args(), vec!["hello", "world"]);
    }

    #[test]
    fn test_codex_patch_parsing() {
        let codex_patch = r#"*** Begin Patch
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

        let tool_call = serde_json::json!({
            "cmd": ["apply_patch", codex_patch]
        });

        let command = CommandFactory::from_tool_call(&tool_call).unwrap();
        assert_eq!(command.name(), "apply_patch");
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Test write operation
        let write_cmd = FileOperationCommand::write(
            file_path.to_string_lossy().to_string(),
            "test content".to_string(),
        );
        let result = write_cmd.execute_direct().await.unwrap();
        assert!(result.success);
        
        // Test read operation
        let read_cmd = FileOperationCommand::read(file_path.to_string_lossy().to_string());
        let result = read_cmd.execute_direct().await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout, "test content");
        
        // Test exists operation
        let exists_cmd = FileOperationCommand {
            operation: FileOperation::Exists,
            source: file_path.to_string_lossy().to_string(),
            destination: None,
            options: FileOperationOptions::default(),
        };
        let result = exists_cmd.execute_direct().await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout, "true");
    }
}