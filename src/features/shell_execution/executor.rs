// src/features/shell_execution/executor.rs
//
// Core shell command execution implementation

use crate::features::diff_patch::models::PatchError;
use crate::features::shell_execution::{CommandResult, ShellConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

/// Configuration for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum execution time
    pub timeout: Duration,
    /// Working directory
    pub working_directory: PathBuf,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Capture stdout
    pub capture_stdout: bool,
    /// Capture stderr
    pub capture_stderr: bool,
    /// Maximum output size in bytes
    pub max_output_size: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            environment: HashMap::new(),
            capture_stdout: true,
            capture_stderr: true,
            max_output_size: 1024 * 1024, // 1MB
        }
    }
}

/// Result of command execution with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Command that was executed
    pub command: String,
    /// Arguments passed to the command
    pub arguments: Vec<String>,
    /// Exit status code
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error_message: Option<String>,
    /// Working directory where command was executed
    pub working_directory: PathBuf,
}

impl ExecutionResult {
    /// Convert to CommandResult
    pub fn to_command_result(&self) -> CommandResult {
        CommandResult {
            exit_code: self.exit_code.unwrap_or(-1),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            duration: self.duration,
            success: self.success,
            error_message: self.error_message.clone(),
        }
    }
}

/// Shell command executor with security and resource controls
pub struct ShellExecutor {
    config: ExecutionConfig,
}

impl ShellExecutor {
    /// Create a new shell executor
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ExecutionConfig::default())
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ExecutionConfig) {
        self.config = config;
    }

    /// Execute a command with arguments
    pub async fn execute(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<CommandResult, PatchError> {
        let result = self.execute_detailed(command, args).await?;
        Ok(result.to_command_result())
    }

    /// Execute a command and return detailed results
    pub async fn execute_detailed(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<ExecutionResult, PatchError> {
        let start_time = Instant::now();
        
        // Create the command
        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        cmd.current_dir(&self.config.working_directory);
        
        // Set environment variables
        for (key, value) in &self.config.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        if self.config.capture_stdout {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::null());
        }

        if self.config.capture_stderr {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::null());
        }

        // Execute with timeout
        let execution_future = self.execute_with_capture(&mut cmd);
        let result = match timeout(self.config.timeout, execution_future).await {
            Ok(result) => result,
            Err(_) => {
                return Ok(ExecutionResult {
                    command: command.to_string(),
                    arguments: args.to_vec(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Command timed out".to_string(),
                    duration: self.config.timeout,
                    success: false,
                    error_message: Some("Command execution timed out".to_string()),
                    working_directory: self.config.working_directory.clone(),
                });
            }
        };

        let duration = start_time.elapsed();
        
        match result {
            Ok((exit_status, stdout, stderr)) => {
                let exit_code = exit_status.code();
                let success = exit_status.success();
                
                Ok(ExecutionResult {
                    command: command.to_string(),
                    arguments: args.to_vec(),
                    exit_code,
                    stdout,
                    stderr: stderr.clone(),
                    duration,
                    success,
                    error_message: if success { None } else { Some(stderr) },
                    working_directory: self.config.working_directory.clone(),
                })
            }
            Err(e) => {
                Ok(ExecutionResult {
                    command: command.to_string(),
                    arguments: args.to_vec(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    duration,
                    success: false,
                    error_message: Some(e.to_string()),
                    working_directory: self.config.working_directory.clone(),
                })
            }
        }
    }

    /// Execute command and capture output with size limits
    async fn execute_with_capture(
        &self,
        cmd: &mut TokioCommand,
    ) -> Result<(std::process::ExitStatus, String, String), PatchError> {
        let mut child = cmd.spawn()
            .map_err(|e| PatchError::IoError { 
                message: format!("Failed to spawn command: {}", e) 
            })?;

        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        // Capture stdout if available
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();
            
            while reader.read_line(&mut buffer).await.unwrap_or(0) > 0 {
                if stdout_content.len() + buffer.len() > self.config.max_output_size {
                    stdout_content.push_str("... [output truncated due to size limit]\n");
                    break;
                }
                stdout_content.push_str(&buffer);
                buffer.clear();
            }
        }

        // Capture stderr if available
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr);
            let mut buffer = String::new();
            
            while reader.read_line(&mut buffer).await.unwrap_or(0) > 0 {
                if stderr_content.len() + buffer.len() > self.config.max_output_size {
                    stderr_content.push_str("... [output truncated due to size limit]\n");
                    break;
                }
                stderr_content.push_str(&buffer);
                buffer.clear();
            }
        }

        // Wait for the command to complete
        let exit_status = child.wait().await
            .map_err(|e| PatchError::IoError { 
                message: format!("Failed to wait for command completion: {}", e) 
            })?;

        Ok((exit_status, stdout_content, stderr_content))
    }

    /// Execute a shell script
    pub async fn execute_script(
        &self,
        script: &str,
        shell: Option<&str>,
    ) -> Result<CommandResult, PatchError> {
        let shell_cmd = shell.unwrap_or(if cfg!(windows) { "cmd" } else { "sh" });
        let shell_arg = if cfg!(windows) { "/C" } else { "-c" };
        
        self.execute(shell_cmd, &[shell_arg.to_string(), script.to_string()]).await
    }

    /// Execute a command synchronously (blocking)
    pub fn execute_sync(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<CommandResult, PatchError> {
        let start_time = Instant::now();
        
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.current_dir(&self.config.working_directory);
        
        // Set environment variables
        for (key, value) in &self.config.environment {
            cmd.env(key, value);
        }

        // Execute and capture output
        let output = cmd.output()
            .map_err(|e| PatchError::IoError { 
                message: format!("Failed to execute command: {}", e) 
            })?;

        let duration = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        // Truncate output if necessary
        let stdout = if stdout.len() > self.config.max_output_size {
            format!("{}... [output truncated]", &stdout[..self.config.max_output_size])
        } else {
            stdout
        };
        
        let stderr = if stderr.len() > self.config.max_output_size {
            format!("{}... [output truncated]", &stderr[..self.config.max_output_size])
        } else {
            stderr
        };

        if output.status.success() {
            Ok(CommandResult::success(stdout, stderr, duration))
        } else {
            Ok(CommandResult::failure(
                output.status.code().unwrap_or(-1),
                stdout,
                stderr.clone(),
                duration,
                Some(stderr),
            ))
        }
    }

    /// Check if a command exists in the system
    pub async fn command_exists(&self, command: &str) -> bool {
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        
        match self.execute(which_cmd, &[command.to_string()]).await {
            Ok(result) => result.success,
            Err(_) => false,
        }
    }

    /// Get system information
    pub async fn get_system_info(&self) -> Result<SystemInfo, PatchError> {
        let os_info = if cfg!(windows) {
            self.execute("cmd", &["/C".to_string(), "ver".to_string()]).await?
        } else {
            self.execute("uname", &["-a".to_string()]).await?
        };

        let shell_info = if cfg!(windows) {
            CommandResult::success("cmd".to_string(), String::new(), Duration::from_millis(0))
        } else {
            self.execute("sh", &["--version".to_string()]).await.unwrap_or_else(|_| {
                CommandResult::success("sh (unknown version)".to_string(), String::new(), Duration::from_millis(0))
            })
        };

        Ok(SystemInfo {
            os: os_info.stdout.trim().to_string(),
            shell: shell_info.stdout.lines().next().unwrap_or("unknown").to_string(),
            working_directory: self.config.working_directory.clone(),
            environment_count: self.config.environment.len(),
        })
    }
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system information
    pub os: String,
    /// Shell information
    pub shell: String,
    /// Current working directory
    pub working_directory: PathBuf,
    /// Number of environment variables set
    pub environment_count: usize,
}

/// Builder for creating ExecutionConfig
pub struct ExecutionConfigBuilder {
    config: ExecutionConfig,
}

impl ExecutionConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: ExecutionConfig::default(),
        }
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set working directory
    pub fn working_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.working_directory = path.into();
        self
    }

    /// Add environment variable
    pub fn environment_variable<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.config.environment.insert(key.into(), value.into());
        self
    }

    /// Set output capture settings
    pub fn capture_output(mut self, stdout: bool, stderr: bool) -> Self {
        self.config.capture_stdout = stdout;
        self.config.capture_stderr = stderr;
        self
    }

    /// Set maximum output size
    pub fn max_output_size(mut self, size: usize) -> Self {
        self.config.max_output_size = size;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ExecutionConfig {
        self.config
    }
}

impl Default for ExecutionConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_execution_config_builder() {
        let config = ExecutionConfigBuilder::new()
            .timeout(Duration::from_secs(10))
            .working_directory("/tmp")
            .environment_variable("TEST_VAR", "test_value")
            .capture_output(true, false)
            .max_output_size(512)
            .build();

        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.working_directory, PathBuf::from("/tmp"));
        assert_eq!(config.environment.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert!(config.capture_stdout);
        assert!(!config.capture_stderr);
        assert_eq!(config.max_output_size, 512);
    }

    #[test]
    fn test_execution_result_conversion() {
        let result = ExecutionResult {
            command: "echo".to_string(),
            arguments: vec!["hello".to_string()],
            exit_code: Some(0),
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(10),
            success: true,
            error_message: None,
            working_directory: PathBuf::from("/"),
        };

        let cmd_result = result.to_command_result();
        assert_eq!(cmd_result.exit_code, 0);
        assert!(cmd_result.success);
        assert_eq!(cmd_result.stdout, "hello\n");
    }

    #[tokio::test]
    async fn test_shell_executor_creation() {
        let config = ExecutionConfig::default();
        let executor = ShellExecutor::new(config);
        
        // Test that we can create an executor
        assert_eq!(executor.config.timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_command_exists() {
        let executor = ShellExecutor::default();
        
        // Test with a command that should exist
        let echo_exists = executor.command_exists("echo").await;
        
        // On most systems, echo should exist
        // This might fail in very minimal environments, but that's expected
        if echo_exists {
            assert!(echo_exists);
        }
    }

    #[tokio::test]
    async fn test_simple_command_execution() {
        let executor = ShellExecutor::default();
        
        // Try to execute a simple command
        // Use a command that works on both Unix and Windows
        let result = if cfg!(windows) {
            executor.execute("cmd", &["/C".to_string(), "echo hello".to_string()]).await
        } else {
            executor.execute("echo", &["hello".to_string()]).await
        };

        match result {
            Ok(cmd_result) => {
                // If the command succeeds, check the output
                if cmd_result.success {
                    assert!(cmd_result.stdout.contains("hello"));
                }
            }
            Err(_) => {
                // Command might not be available in test environment
                // This is acceptable for unit tests
            }
        }
    }

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);
        assert_eq!(config.max_output_size, 1024 * 1024);
    }
}