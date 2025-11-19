// src/features/shell_execution/security.rs
//
// Security validation and sandboxing for shell command execution

use crate::features::diff_patch::models::PatchError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Security policy for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Commands that are explicitly allowed
    pub allowed_commands: HashSet<String>,
    /// Commands that are explicitly blocked
    pub blocked_commands: HashSet<String>,
    /// Whether to allow any command not explicitly blocked
    pub allow_unlisted: bool,
    /// Directories where file operations are allowed
    pub allowed_directories: Vec<PathBuf>,
    /// Maximum command line length
    pub max_command_length: usize,
    /// Whether to allow shell metacharacters
    pub allow_shell_metacharacters: bool,
    /// Maximum number of arguments
    pub max_arguments: usize,
    /// Patterns that are not allowed in arguments
    pub blocked_patterns: Vec<String>,
    /// Whether to validate file paths for directory traversal
    pub validate_paths: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let mut allowed_commands = HashSet::new();
        
        // Safe commands for patch operations
        allowed_commands.insert("apply_patch".to_string());
        allowed_commands.insert("patch".to_string());
        allowed_commands.insert("diff".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("head".to_string());
        allowed_commands.insert("tail".to_string());
        allowed_commands.insert("wc".to_string());
        allowed_commands.insert("grep".to_string());
        allowed_commands.insert("echo".to_string());
        
        let mut blocked_commands = HashSet::new();
        
        // Dangerous commands
        blocked_commands.insert("rm".to_string());
        blocked_commands.insert("rmdir".to_string());
        blocked_commands.insert("dd".to_string());
        blocked_commands.insert("chmod".to_string());
        blocked_commands.insert("chown".to_string());
        blocked_commands.insert("sudo".to_string());
        blocked_commands.insert("su".to_string());
        blocked_commands.insert("passwd".to_string());
        blocked_commands.insert("mount".to_string());
        blocked_commands.insert("umount".to_string());
        blocked_commands.insert("mkfs".to_string());
        blocked_commands.insert("fdisk".to_string());
        blocked_commands.insert("crontab".to_string());
        blocked_commands.insert("systemctl".to_string());
        blocked_commands.insert("service".to_string());
        blocked_commands.insert("reboot".to_string());
        blocked_commands.insert("shutdown".to_string());
        blocked_commands.insert("halt".to_string());
        blocked_commands.insert("init".to_string());
        blocked_commands.insert("kill".to_string());
        blocked_commands.insert("killall".to_string());
        blocked_commands.insert("pkill".to_string());

        let blocked_patterns = vec![
            r"\.\./".to_string(),          // Directory traversal
            r";".to_string(),              // Command chaining
            r"\|".to_string(),             // Pipes
            r"&".to_string(),              // Background execution
            r"`".to_string(),              // Command substitution
            r"\$\(".to_string(),           // Command substitution
            r">\s*/dev/".to_string(),      // Device access
            r"<\s*/dev/".to_string(),      // Device access
            r">\s*/proc/".to_string(),     // Proc filesystem
            r"<\s*/proc/".to_string(),     // Proc filesystem
            r">\s*/sys/".to_string(),      // Sys filesystem  
            r"<\s*/sys/".to_string(),      // Sys filesystem
            r"rm\s+-rf".to_string(),       // Recursive force delete
            r":\(\)\{.*\}:".to_string(),   // Fork bomb pattern
        ];

        Self {
            allowed_commands,
            blocked_commands,
            allow_unlisted: false,
            allowed_directories: Vec::new(),
            max_command_length: 2048,
            allow_shell_metacharacters: false,
            max_arguments: 50,
            blocked_patterns,
            validate_paths: true,
        }
    }
}

impl SecurityPolicy {
    /// Create a permissive policy (for testing/development)
    pub fn permissive() -> Self {
        Self {
            allowed_commands: HashSet::new(),
            blocked_commands: HashSet::new(),
            allow_unlisted: true,
            allowed_directories: Vec::new(),
            max_command_length: 8192,
            allow_shell_metacharacters: true,
            max_arguments: 100,
            blocked_patterns: Vec::new(),
            validate_paths: false,
        }
    }

    /// Create a restrictive policy (production use)
    pub fn restrictive() -> Self {
        let mut policy = Self::default();
        policy.allow_unlisted = false;
        policy.allow_shell_metacharacters = false;
        policy.max_command_length = 1024;
        policy.max_arguments = 20;
        policy
    }

    /// Add an allowed command
    pub fn allow_command<S: Into<String>>(&mut self, command: S) {
        self.allowed_commands.insert(command.into());
    }

    /// Add a blocked command
    pub fn block_command<S: Into<String>>(&mut self, command: S) {
        self.blocked_commands.insert(command.into());
    }

    /// Add an allowed directory
    pub fn allow_directory<P: Into<PathBuf>>(&mut self, path: P) {
        self.allowed_directories.push(path.into());
    }

    /// Add a blocked pattern
    pub fn block_pattern<S: Into<String>>(&mut self, pattern: S) {
        self.blocked_patterns.push(pattern.into());
    }

    /// Check if a command is allowed
    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Check if explicitly blocked
        if self.blocked_commands.contains(command) {
            return false;
        }

        // Check if explicitly allowed
        if self.allowed_commands.contains(command) {
            return true;
        }

        // Check default policy
        self.allow_unlisted
    }
}

/// Validate a command and its arguments against security policy
pub fn validate_command(
    command: &str,
    args: &[String],
    policy: &SecurityPolicy,
) -> Result<(), PatchError> {
    // Check command length
    let full_command = format!("{} {}", command, args.join(" "));
    if full_command.len() > policy.max_command_length {
        return Err(PatchError::SecurityViolation {
            path: format!(
                "Command line too long: {} characters (max: {})",
                full_command.len(),
                policy.max_command_length
            ),
        });
    }

    // Check argument count
    if args.len() > policy.max_arguments {
        return Err(PatchError::SecurityViolation {
            path: format!(
                "Too many arguments: {} (max: {})",
                args.len(),
                policy.max_arguments
            ),
        });
    }

    // Check if command is allowed
    if !policy.is_command_allowed(command) {
        return Err(PatchError::SecurityViolation {
            path: format!("Command not allowed: {}", command),
        });
    }

    // Check for blocked patterns
    for pattern in &policy.blocked_patterns {
        let regex = Regex::new(pattern)
            .map_err(|e| PatchError::SecurityViolation {
                path: format!("Invalid regex pattern: {}", e),
            })?;

        if regex.is_match(&full_command) {
            return Err(PatchError::SecurityViolation {
                path: format!("Command contains blocked pattern: {}", pattern),
            });
        }
    }

    // Check for shell metacharacters if not allowed
    if !policy.allow_shell_metacharacters {
        let dangerous_chars = ['|', '&', ';', '>', '<', '`', '$', '*', '?', '[', ']', '(', ')', '{', '}'];
        for &ch in &dangerous_chars {
            if full_command.contains(ch) {
                return Err(PatchError::SecurityViolation {
                    path: format!("Shell metacharacter not allowed: {}", ch),
                });
            }
        }
    }

    // Validate file paths in arguments
    if policy.validate_paths {
        for arg in args {
            validate_file_path_in_arg(arg, &policy.allowed_directories)?;
        }
    }

    Ok(())
}

/// Validate file paths in command arguments
fn validate_file_path_in_arg(
    arg: &str,
    allowed_directories: &[PathBuf],
) -> Result<(), PatchError> {
    // Skip non-path arguments
    if !looks_like_path(arg) {
        return Ok(());
    }

    let path = Path::new(arg);

    // Check for directory traversal
    if arg.contains("..") {
        return Err(PatchError::SecurityViolation {
            path: format!("Directory traversal detected: {}", arg),
        });
    }

    // Check against allowed directories if specified
    if !allowed_directories.is_empty() {
        let canonical_path = path.canonicalize()
            .or_else(|_| {
                // If canonicalization fails, try the parent directory
                if let Some(parent) = path.parent() {
                    parent.canonicalize()
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Path not found"))
                }
            });

        match canonical_path {
            Ok(canon_path) => {
                let mut allowed = false;
                for allowed_dir in allowed_directories {
                    if let Ok(allowed_canonical) = allowed_dir.canonicalize() {
                        if canon_path.starts_with(&allowed_canonical) {
                            allowed = true;
                            break;
                        }
                    }
                }
                
                if !allowed {
                    return Err(PatchError::SecurityViolation {
                        path: format!("Path not in allowed directories: {}", arg),
                    });
                }
            }
            Err(_) => {
                // If we can't canonicalize, be more restrictive
                // Allow only relative paths within current directory
                if path.is_absolute() {
                    return Err(PatchError::SecurityViolation {
                        path: format!("Absolute path not allowed: {}", arg),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if a string looks like a file path
fn looks_like_path(s: &str) -> bool {
    // Simple heuristics to detect file paths
    s.contains('/') || 
    s.contains('\\') || 
    s.ends_with(".txt") ||
    s.ends_with(".rs") ||
    s.ends_with(".py") ||
    s.ends_with(".js") ||
    s.ends_with(".json") ||
    s.ends_with(".md") ||
    s.ends_with(".yaml") ||
    s.ends_with(".yml") ||
    s.starts_with("./") ||
    s.starts_with("../") ||
    s == "." ||
    s == ".."
}

/// Sanitize a file path for safe operations
pub fn sanitize_path(path: &str) -> Result<String, PatchError> {
    let path_obj = Path::new(path);
    
    // Check for directory traversal
    if path.contains("..") {
        return Err(PatchError::SecurityViolation {
            path: format!("Directory traversal detected: {}", path),
        });
    }
    
    // Normalize the path
    let normalized = path_obj.to_string_lossy();
    
    // Remove dangerous characters
    let safe_path = normalized
        .replace('|', "_")
        .replace('&', "_")
        .replace(';', "_")
        .replace('`', "_")
        .replace('$', "_");
    
    Ok(safe_path.to_string())
}

/// Create a sandbox environment for command execution
pub struct CommandSandbox {
    policy: SecurityPolicy,
    temp_directory: Option<PathBuf>,
}

impl CommandSandbox {
    /// Create a new command sandbox
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            temp_directory: None,
        }
    }

    /// Create with default policy
    pub fn default() -> Self {
        Self::new(SecurityPolicy::default())
    }

    /// Set up a temporary directory for operations
    pub fn setup_temp_directory(&mut self) -> Result<(), PatchError> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| PatchError::IoError {
                message: format!("Failed to create temporary directory: {}", e),
            })?;
        
        self.temp_directory = Some(temp_dir.into_path());
        Ok(())
    }

    /// Validate a command within the sandbox
    pub fn validate_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(), PatchError> {
        validate_command(command, args, &self.policy)
    }

    /// Get the temporary directory path
    pub fn temp_directory(&self) -> Option<&PathBuf> {
        self.temp_directory.as_ref()
    }

    /// Clean up the sandbox
    pub fn cleanup(&mut self) -> Result<(), PatchError> {
        if let Some(temp_dir) = &self.temp_directory {
            if temp_dir.exists() {
                std::fs::remove_dir_all(temp_dir)
                    .map_err(|e| PatchError::IoError {
                        message: format!("Failed to cleanup temporary directory: {}", e),
                    })?;
            }
            self.temp_directory = None;
        }
        Ok(())
    }
}

impl Drop for CommandSandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Escape shell arguments to prevent injection
pub fn escape_shell_arg(arg: &str) -> String {
    if cfg!(windows) {
        // Windows command line escaping
        if arg.is_empty() {
            return "\"\"".to_string();
        }
        
        if arg.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
            return arg.to_string();
        }
        
        format!("\"{}\"", arg.replace('\"', "\\\""))
    } else {
        // Unix shell escaping
        if arg.is_empty() {
            return "''".to_string();
        }
        
        if arg.chars().all(|c| c.is_ascii_alphanumeric() || "._-/".contains(c)) {
            return arg.to_string();
        }
        
        format!("'{}'", arg.replace('\'', "'\"'\"'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        
        assert!(policy.is_command_allowed("echo"));
        assert!(policy.is_command_allowed("cat"));
        assert!(!policy.is_command_allowed("rm"));
        assert!(!policy.is_command_allowed("sudo"));
        assert!(!policy.allow_unlisted);
    }

    #[test]
    fn test_security_policy_permissive() {
        let policy = SecurityPolicy::permissive();
        
        assert!(policy.allow_unlisted);
        assert!(policy.allow_shell_metacharacters);
        assert!(policy.is_command_allowed("any_command"));
    }

    #[test]
    fn test_validate_safe_command() {
        let policy = SecurityPolicy::default();
        let result = validate_command("echo", &["hello".to_string()], &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_blocked_command() {
        let policy = SecurityPolicy::default();
        let result = validate_command("rm", &["-rf".to_string(), "/".to_string()], &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_length() {
        let mut policy = SecurityPolicy::default();
        policy.max_command_length = 10;
        
        let long_args = vec!["very_long_argument_that_exceeds_limit".to_string()];
        let result = validate_command("echo", &long_args, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_blocked_patterns() {
        let policy = SecurityPolicy::default();
        
        // Test directory traversal
        let result = validate_command("cat", &["../../../etc/passwd".to_string()], &policy);
        assert!(result.is_err());
        
        // Test command chaining
        let result = validate_command("echo", &["hello;rm -rf /".to_string()], &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("/etc/passwd"));
        assert!(looks_like_path("./file.txt"));
        assert!(looks_like_path("../parent/file.rs"));
        assert!(looks_like_path("document.json"));
        assert!(!looks_like_path("hello"));
        assert!(!looks_like_path("123"));
        assert!(!looks_like_path("--help"));
    }

    #[test]
    fn test_sanitize_path() {
        assert!(sanitize_path("../etc/passwd").is_err());
        assert!(sanitize_path("normal/path.txt").is_ok());
        
        let sanitized = sanitize_path("path|with&dangerous;chars").unwrap();
        assert!(!sanitized.contains('|'));
        assert!(!sanitized.contains('&'));
        assert!(!sanitized.contains(';'));
    }

    #[test]
    fn test_escape_shell_arg() {
        assert_eq!(escape_shell_arg("simple"), "simple");
        assert_eq!(escape_shell_arg(""), if cfg!(windows) { "\"\"" } else { "''" });
        
        let escaped = escape_shell_arg("arg with spaces");
        if cfg!(windows) {
            assert!(escaped.starts_with('"') && escaped.ends_with('"'));
        } else {
            assert!(escaped.starts_with('\'') && escaped.ends_with('\''));
        }
    }

    #[test]
    fn test_command_sandbox() {
        let policy = SecurityPolicy::default();
        let sandbox = CommandSandbox::new(policy);
        
        let result = sandbox.validate_command("echo", &["test".to_string()]);
        assert!(result.is_ok());
        
        let result = sandbox.validate_command("rm", &["-rf".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_security_policy_builder() {
        let mut policy = SecurityPolicy::default();
        policy.allow_command("custom_command");
        policy.block_command("dangerous_command");
        policy.allow_directory(PathBuf::from("/safe/directory"));
        
        assert!(policy.is_command_allowed("custom_command"));
        assert!(!policy.is_command_allowed("dangerous_command"));
        assert_eq!(policy.allowed_directories.len(), 1);
    }
}