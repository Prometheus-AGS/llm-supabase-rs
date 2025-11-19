// src/features/diff_patch/models.rs
//
// Data structures for unified diff generation and patch application

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Represents a single operation in a diff hunk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchOperation {
    /// Context line (unchanged)
    Context { content: String },
    /// Line to be added
    Add { content: String },
    /// Line to be removed
    Remove { content: String },
}

impl PatchOperation {
    /// Get the content of the operation
    pub fn content(&self) -> &str {
        match self {
            PatchOperation::Context { content } => content,
            PatchOperation::Add { content } => content,
            PatchOperation::Remove { content } => content,
        }
    }

    /// Get the operation type as a character prefix
    pub fn prefix_char(&self) -> char {
        match self {
            PatchOperation::Context { .. } => ' ',
            PatchOperation::Add { .. } => '+',
            PatchOperation::Remove { .. } => '-',
        }
    }

    /// Create a context operation
    pub fn context(content: String) -> Self {
        PatchOperation::Context { content }
    }

    /// Create an add operation
    pub fn add(content: String) -> Self {
        PatchOperation::Add { content }
    }

    /// Create a remove operation
    pub fn remove(content: String) -> Self {
        PatchOperation::Remove { content }
    }
}

/// Represents a single hunk in a unified diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffChunk {
    /// Starting line number in the original file (1-based)
    pub old_start: usize,
    /// Number of lines in the original file covered by this hunk
    pub old_count: usize,
    /// Starting line number in the new file (1-based)
    pub new_start: usize,
    /// Number of lines in the new file covered by this hunk
    pub new_count: usize,
    /// The operations that make up this hunk
    pub operations: Vec<PatchOperation>,
    /// Optional context line (function name, etc.)
    pub context: Option<String>,
}

impl DiffChunk {
    /// Create a new diff chunk
    pub fn new(
        old_start: usize,
        old_count: usize,
        new_start: usize,
        new_count: usize,
        operations: Vec<PatchOperation>,
    ) -> Self {
        Self {
            old_start,
            old_count,
            new_start,
            new_count,
            operations,
            context: None,
        }
    }

    /// Create a new diff chunk with context
    pub fn with_context(
        old_start: usize,
        old_count: usize,
        new_start: usize,
        new_count: usize,
        operations: Vec<PatchOperation>,
        context: String,
    ) -> Self {
        Self {
            old_start,
            old_count,
            new_start,
            new_count,
            operations,
            context: Some(context),
        }
    }

    /// Get the hunk header line (e.g., "@@ -1,5 +1,6 @@")
    pub fn header_line(&self) -> String {
        let mut header = format!("@@ -{},{} +{},{} @@", 
            self.old_start, self.old_count, 
            self.new_start, self.new_count
        );
        
        if let Some(ref context) = self.context {
            header.push(' ');
            header.push_str(context);
        }
        
        header
    }

    /// Validate that the chunk operations match the declared counts
    pub fn validate(&self) -> Result<(), PatchError> {
        let mut old_lines = 0;
        let mut new_lines = 0;

        for op in &self.operations {
            match op {
                PatchOperation::Context { .. } => {
                    old_lines += 1;
                    new_lines += 1;
                }
                PatchOperation::Remove { .. } => {
                    old_lines += 1;
                }
                PatchOperation::Add { .. } => {
                    new_lines += 1;
                }
            }
        }

        if old_lines != self.old_count {
            return Err(PatchError::InvalidChunk {
                message: format!(
                    "Old line count mismatch: expected {}, got {}", 
                    self.old_count, old_lines
                ),
            });
        }

        if new_lines != self.new_count {
            return Err(PatchError::InvalidChunk {
                message: format!(
                    "New line count mismatch: expected {}, got {}", 
                    self.new_count, new_lines
                ),
            });
        }

        Ok(())
    }
}

impl fmt::Display for DiffChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.header_line())?;
        
        for operation in &self.operations {
            writeln!(f, "{}{}", operation.prefix_char(), operation.content())?;
        }
        
        Ok(())
    }
}

/// Metadata about files in a diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffMetadata {
    /// Original file path
    pub old_path: String,
    /// New file path (may be same as old_path)
    pub new_path: String,
    /// Original file timestamp (optional)
    pub old_timestamp: Option<String>,
    /// New file timestamp (optional) 
    pub new_timestamp: Option<String>,
    /// File mode (optional, e.g., "100644")
    pub mode: Option<String>,
    /// Whether this is a binary file
    pub is_binary: bool,
}

impl DiffMetadata {
    /// Create new diff metadata
    pub fn new(old_path: String, new_path: String) -> Self {
        Self {
            old_path,
            new_path,
            old_timestamp: None,
            new_timestamp: None,
            mode: None,
            is_binary: false,
        }
    }

    /// Create metadata for a single file (old_path == new_path)
    pub fn single_file(path: String) -> Self {
        Self::new(path.clone(), path)
    }

    /// Set timestamps
    pub fn with_timestamps(mut self, old_timestamp: String, new_timestamp: String) -> Self {
        self.old_timestamp = Some(old_timestamp);
        self.new_timestamp = Some(new_timestamp);
        self
    }

    /// Mark as binary file
    pub fn as_binary(mut self) -> Self {
        self.is_binary = true;
        self
    }

    /// Get the "---" header line
    pub fn old_header_line(&self) -> String {
        let mut line = format!("--- {}", self.old_path);
        if let Some(ref timestamp) = self.old_timestamp {
            line.push('\t');
            line.push_str(timestamp);
        }
        line
    }

    /// Get the "+++" header line
    pub fn new_header_line(&self) -> String {
        let mut line = format!("+++ {}", self.new_path);
        if let Some(ref timestamp) = self.new_timestamp {
            line.push('\t');
            line.push_str(timestamp);
        }
        line
    }
}

/// Represents a complete unified diff for a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedDiff {
    /// File metadata
    pub metadata: DiffMetadata,
    /// The diff chunks (hunks)
    pub chunks: Vec<DiffChunk>,
}

impl UnifiedDiff {
    /// Create a new unified diff
    pub fn new(metadata: DiffMetadata, chunks: Vec<DiffChunk>) -> Self {
        Self { metadata, chunks }
    }

    /// Create an empty diff (no changes)
    pub fn empty(old_path: String, new_path: String) -> Self {
        Self::new(DiffMetadata::new(old_path, new_path), Vec::new())
    }

    /// Check if this diff has any changes
    pub fn has_changes(&self) -> bool {
        !self.chunks.is_empty()
    }

    /// Validate all chunks in the diff
    pub fn validate(&self) -> Result<(), PatchError> {
        for (i, chunk) in self.chunks.iter().enumerate() {
            chunk.validate().map_err(|e| PatchError::InvalidChunk {
                message: format!("Chunk {}: {}", i + 1, e),
            })?;
        }
        Ok(())
    }

    /// Get the total number of additions
    pub fn additions(&self) -> usize {
        self.chunks.iter()
            .flat_map(|chunk| &chunk.operations)
            .filter(|op| matches!(op, PatchOperation::Add { .. }))
            .count()
    }

    /// Get the total number of deletions
    pub fn deletions(&self) -> usize {
        self.chunks.iter()
            .flat_map(|chunk| &chunk.operations)
            .filter(|op| matches!(op, PatchOperation::Remove { .. }))
            .count()
    }

    /// Convert to Codex CLI format
    pub fn to_codex_format(&self) -> String {
        if self.metadata.is_binary {
            return format!(
                "Binary files {} and {} differ\n", 
                self.metadata.old_path, 
                self.metadata.new_path
            );
        }

        let mut result = String::new();
        
        // Add file headers
        result.push_str(&self.metadata.old_header_line());
        result.push('\n');
        result.push_str(&self.metadata.new_header_line());
        result.push('\n');
        
        // Add chunks
        for chunk in &self.chunks {
            result.push_str(&chunk.to_string());
        }
        
        result
    }
}

impl fmt::Display for UnifiedDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_codex_format())
    }
}

/// Errors that can occur during diff generation or patch application
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchError {
    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Invalid patch format: {message}")]
    InvalidPatchFormat { message: String },

    #[error("Invalid chunk: {message}")]
    InvalidChunk { message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    #[error("Patch application failed: {message}")]
    PatchApplicationFailed { message: String },

    #[error("Line mismatch at line {line_number}: expected '{expected}', found '{actual}'")]
    LineMismatch {
        line_number: usize,
        expected: String,
        actual: String,
    },

    #[error("Context mismatch: {message}")]
    ContextMismatch { message: String },

    #[error("Binary file operation not supported: {path}")]
    BinaryFileNotSupported { path: String },

    #[error("Backup operation failed: {message}")]
    BackupFailed { message: String },

    #[error("Directory traversal attack detected: {path}")]
    SecurityViolation { path: String },

    #[error("File size limit exceeded: {path} ({size} bytes)")]
    FileSizeExceeded { path: String, size: usize },
}

impl From<std::io::Error> for PatchError {
    fn from(err: std::io::Error) -> Self {
        PatchError::IoError { 
            message: err.to_string() 
        }
    }
}

/// Configuration for diff generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    /// Number of context lines around changes
    pub context_lines: usize,
    /// Maximum file size to process (in bytes)
    pub max_file_size: usize,
    /// Whether to detect binary files
    pub detect_binary: bool,
    /// Whether to include timestamps in headers
    pub include_timestamps: bool,
    /// Custom timestamp format
    pub timestamp_format: Option<String>,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            context_lines: 3,
            max_file_size: 10 * 1024 * 1024, // 10MB
            detect_binary: true,
            include_timestamps: false,
            timestamp_format: None,
        }
    }
}

/// Configuration for patch application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchConfig {
    /// Whether to create backup files before applying patches
    pub create_backup: bool,
    /// Backup file extension (e.g., ".bak")
    pub backup_extension: String,
    /// Whether to allow fuzzy matching for shifted lines
    pub fuzzy_matching: bool,
    /// Maximum fuzz factor (lines that can be offset)
    pub max_fuzz: usize,
    /// Whether to validate file paths for security
    pub validate_paths: bool,
    /// Allowed base directories for patch operations
    pub allowed_directories: Vec<String>,
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            create_backup: true,
            backup_extension: ".bak".to_string(),
            fuzzy_matching: true,
            max_fuzz: 2,
            validate_paths: true,
            allowed_directories: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_operation_creation() {
        let context_op = PatchOperation::context("unchanged line".to_string());
        let add_op = PatchOperation::add("new line".to_string());
        let remove_op = PatchOperation::remove("old line".to_string());

        assert_eq!(context_op.content(), "unchanged line");
        assert_eq!(add_op.content(), "new line");
        assert_eq!(remove_op.content(), "old line");

        assert_eq!(context_op.prefix_char(), ' ');
        assert_eq!(add_op.prefix_char(), '+');
        assert_eq!(remove_op.prefix_char(), '-');
    }

    #[test]
    fn test_diff_chunk_header() {
        let chunk = DiffChunk::new(1, 5, 1, 6, vec![]);
        assert_eq!(chunk.header_line(), "@@ -1,5 +1,6 @@");

        let chunk_with_context = DiffChunk::with_context(
            10, 3, 10, 4, vec![], "function_name()".to_string()
        );
        assert_eq!(chunk_with_context.header_line(), "@@ -10,3 +10,4 @@ function_name()");
    }

    #[test]
    fn test_diff_metadata_headers() {
        let metadata = DiffMetadata::new("old.txt".to_string(), "new.txt".to_string())
            .with_timestamps("2024-01-01 10:00:00".to_string(), "2024-01-01 11:00:00".to_string());

        assert_eq!(metadata.old_header_line(), "--- old.txt\t2024-01-01 10:00:00");
        assert_eq!(metadata.new_header_line(), "+++ new.txt\t2024-01-01 11:00:00");
    }

    #[test]
    fn test_unified_diff_stats() {
        let operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::remove("old line".to_string()),
            PatchOperation::add("new line 1".to_string()),
            PatchOperation::add("new line 2".to_string()),
            PatchOperation::context("line 2".to_string()),
        ];

        let chunk = DiffChunk::new(1, 3, 1, 4, operations);
        let metadata = DiffMetadata::single_file("test.txt".to_string());
        let diff = UnifiedDiff::new(metadata, vec![chunk]);

        assert_eq!(diff.additions(), 2);
        assert_eq!(diff.deletions(), 1);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_chunk_validation() {
        // Valid chunk
        let valid_operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::remove("old line".to_string()),
            PatchOperation::add("new line".to_string()),
        ];
        let valid_chunk = DiffChunk::new(1, 2, 1, 2, valid_operations);
        assert!(valid_chunk.validate().is_ok());

        // Invalid chunk - wrong old count
        let invalid_operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::remove("old line".to_string()),
        ];
        let invalid_chunk = DiffChunk::new(1, 3, 1, 1, invalid_operations); // Says 3 old lines, but only has 2
        assert!(invalid_chunk.validate().is_err());
    }
}