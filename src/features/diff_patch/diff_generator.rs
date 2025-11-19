// src/features/diff_patch/diff_generator.rs
//
// Unified diff generation implementation

use crate::features::diff_patch::models::{
    DiffChunk, DiffConfig, DiffMetadata, PatchError, PatchOperation, UnifiedDiff,
};
use similar::{Algorithm, ChangeTag, TextDiff};
use std::path::Path;

/// Generate a unified diff between two strings
pub fn generate_diff(
    old_content: &str,
    new_content: &str,
    file_path: &str,
) -> Result<UnifiedDiff, PatchError> {
    generate_diff_with_config(old_content, new_content, file_path, &DiffConfig::default())
}

/// Generate a unified diff with custom configuration
pub fn generate_diff_with_config(
    old_content: &str,
    new_content: &str,
    file_path: &str,
    config: &DiffConfig,
) -> Result<UnifiedDiff, PatchError> {
    // Check file size limits
    if old_content.len() > config.max_file_size || new_content.len() > config.max_file_size {
        return Err(PatchError::FileSizeExceeded {
            path: file_path.to_string(),
            size: std::cmp::max(old_content.len(), new_content.len()),
        });
    }

    // Check for binary content if enabled
    if config.detect_binary && (is_binary_content(old_content) || is_binary_content(new_content)) {
        let metadata = DiffMetadata::single_file(file_path.to_string()).as_binary();
        return Ok(UnifiedDiff::new(metadata, Vec::new()));
    }

    // Create metadata
    let mut metadata = DiffMetadata::single_file(file_path.to_string());
    
    // Add timestamps if configured
    if config.include_timestamps {
        let timestamp = if let Some(ref format) = config.timestamp_format {
            chrono::Utc::now().format(format).to_string()
        } else {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
        };
        metadata = metadata.with_timestamps(timestamp.clone(), timestamp);
    }

    // Generate diff using similar crate
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(old_content, new_content);

    // Convert to our chunk format
    let chunks = convert_similar_diff_to_chunks(&diff, config.context_lines)?;

    Ok(UnifiedDiff::new(metadata, chunks))
}

/// Generate a diff between two files
pub fn generate_file_diff(
    old_path: &str,
    new_path: &str,
    config: &DiffConfig,
) -> Result<UnifiedDiff, PatchError> {
    let old_content = std::fs::read_to_string(old_path)
        .map_err(|_| PatchError::FileNotFound { 
            path: old_path.to_string() 
        })?;

    let new_content = std::fs::read_to_string(new_path)
        .map_err(|_| PatchError::FileNotFound { 
            path: new_path.to_string() 
        })?;

    // Create metadata with both paths
    let mut metadata = DiffMetadata::new(old_path.to_string(), new_path.to_string());

    // Add file metadata
    if let (Ok(old_meta), Ok(new_meta)) = (
        std::fs::metadata(old_path),
        std::fs::metadata(new_path)
    ) {
        if config.include_timestamps {
            use std::time::UNIX_EPOCH;
            
            let old_timestamp = old_meta.modified()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let new_timestamp = new_meta.modified()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let format_timestamp = |ts: u64| -> String {
                let datetime = chrono::DateTime::from_timestamp(ts as i64, 0)
                    .unwrap_or_default();
                if let Some(ref format) = config.timestamp_format {
                    datetime.format(format).to_string()
                } else {
                    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                }
            };

            metadata = metadata.with_timestamps(
                format_timestamp(old_timestamp),
                format_timestamp(new_timestamp)
            );
        }
    }

    // Check for binary files
    if config.detect_binary && (is_binary_content(&old_content) || is_binary_content(&new_content)) {
        metadata = metadata.as_binary();
        return Ok(UnifiedDiff::new(metadata, Vec::new()));
    }

    // Generate diff
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(&old_content, &new_content);

    let chunks = convert_similar_diff_to_chunks(&diff, config.context_lines)?;

    Ok(UnifiedDiff::new(metadata, chunks))
}

/// Convert similar::TextDiff to our DiffChunk format
fn convert_similar_diff_to_chunks(
    diff: &TextDiff<'_, '_, '_, str>,
    context_lines: usize,
) -> Result<Vec<DiffChunk>, PatchError> {
    let mut chunks = Vec::new();
    let mut current_operations = Vec::new();
    let mut old_line = 1;
    let mut new_line = 1;
    let mut chunk_old_start = 1;
    let mut chunk_new_start = 1;
    let mut in_hunk = false;

    let changes: Vec<_> = diff.iter_all_changes().collect();
    
    for (i, change) in changes.iter().enumerate() {
        let line_content = change.value().trim_end_matches('\n').to_string();

        match change.tag() {
            ChangeTag::Equal => {
                let context_op = PatchOperation::context(line_content);
                
                if in_hunk {
                    current_operations.push(context_op);
                } else {
                    // Check if we're approaching a change that would start a new hunk
                    let upcoming_changes = should_start_hunk(&changes[i..], context_lines);
                    if upcoming_changes {
                        in_hunk = true;
                        chunk_old_start = old_line;
                        chunk_new_start = new_line;
                        current_operations.push(context_op);
                    }
                }

                // Check if we should end the current hunk
                if in_hunk && should_end_hunk(&changes[i..], context_lines) {
                    let chunk = create_chunk_from_operations(
                        chunk_old_start,
                        chunk_new_start,
                        &current_operations,
                    )?;
                    chunks.push(chunk);
                    current_operations.clear();
                    in_hunk = false;
                }

                old_line += 1;
                new_line += 1;
            }
            ChangeTag::Delete => {
                if !in_hunk {
                    in_hunk = true;
                    chunk_old_start = old_line.saturating_sub(
                        count_preceding_context(&changes[..i], context_lines)
                    );
                    chunk_new_start = new_line.saturating_sub(
                        count_preceding_context(&changes[..i], context_lines)
                    );
                    
                    // Add preceding context
                    add_preceding_context(&changes[..i], context_lines, &mut current_operations);
                }

                current_operations.push(PatchOperation::remove(line_content));
                old_line += 1;
            }
            ChangeTag::Insert => {
                if !in_hunk {
                    in_hunk = true;
                    chunk_old_start = old_line.saturating_sub(
                        count_preceding_context(&changes[..i], context_lines)
                    );
                    chunk_new_start = new_line.saturating_sub(
                        count_preceding_context(&changes[..i], context_lines)
                    );
                    
                    // Add preceding context
                    add_preceding_context(&changes[..i], context_lines, &mut current_operations);
                }

                current_operations.push(PatchOperation::add(line_content));
                new_line += 1;
            }
        }
    }

    // Finalize any remaining hunk
    if in_hunk && !current_operations.is_empty() {
        let chunk = create_chunk_from_operations(
            chunk_old_start,
            chunk_new_start,
            &current_operations,
        )?;
        chunks.push(chunk);
    }

    Ok(chunks)
}

/// Check if we should start a new hunk based on upcoming changes
fn should_start_hunk(remaining_changes: &[similar::Change<&str>], context_lines: usize) -> bool {
    let mut distance_to_change = 0;
    
    for change in remaining_changes {
        match change.tag() {
            ChangeTag::Equal => {
                distance_to_change += 1;
                if distance_to_change > context_lines {
                    return false;
                }
            }
            ChangeTag::Delete | ChangeTag::Insert => {
                return distance_to_change <= context_lines;
            }
        }
    }
    
    false
}

/// Check if we should end the current hunk
fn should_end_hunk(remaining_changes: &[similar::Change<&str>], context_lines: usize) -> bool {
    let mut equal_count = 0;
    let mut found_more_changes = false;

    for change in remaining_changes.iter().skip(1) { // Skip current change
        match change.tag() {
            ChangeTag::Equal => {
                equal_count += 1;
            }
            ChangeTag::Delete | ChangeTag::Insert => {
                found_more_changes = true;
                break;
            }
        }
    }

    // End hunk if we have enough context and no more changes nearby
    equal_count >= context_lines && !found_more_changes
}

/// Count preceding context lines
fn count_preceding_context(
    preceding_changes: &[similar::Change<&str>],
    context_lines: usize,
) -> usize {
    let mut count = 0;
    
    for change in preceding_changes.iter().rev() {
        if count >= context_lines {
            break;
        }
        
        if matches!(change.tag(), ChangeTag::Equal) {
            count += 1;
        } else {
            break;
        }
    }
    
    count
}

/// Add preceding context lines to operations
fn add_preceding_context(
    preceding_changes: &[similar::Change<&str>],
    context_lines: usize,
    operations: &mut Vec<PatchOperation>,
) {
    let context_changes: Vec<_> = preceding_changes
        .iter()
        .rev()
        .take_while(|change| matches!(change.tag(), ChangeTag::Equal))
        .take(context_lines)
        .collect();
    
    for change in context_changes.iter().rev() {
        let line_content = change.value().trim_end_matches('\n').to_string();
        operations.push(PatchOperation::context(line_content));
    }
}

/// Create a diff chunk from accumulated operations
fn create_chunk_from_operations(
    old_start: usize,
    new_start: usize,
    operations: &[PatchOperation],
) -> Result<DiffChunk, PatchError> {
    let mut old_count = 0;
    let mut new_count = 0;

    for op in operations {
        match op {
            PatchOperation::Context { .. } => {
                old_count += 1;
                new_count += 1;
            }
            PatchOperation::Remove { .. } => {
                old_count += 1;
            }
            PatchOperation::Add { .. } => {
                new_count += 1;
            }
        }
    }

    let chunk = DiffChunk::new(
        old_start,
        old_count,
        new_start,
        new_count,
        operations.to_vec(),
    );

    chunk.validate()?;
    Ok(chunk)
}

/// Detect if content is binary
fn is_binary_content(content: &str) -> bool {
    // Simple binary detection: look for null bytes or high ratio of non-printable chars
    let null_count = content.chars().filter(|&c| c == '\0').count();
    if null_count > 0 {
        return true;
    }

    let total_chars = content.chars().count();
    if total_chars == 0 {
        return false;
    }

    let non_printable = content.chars()
        .filter(|&c| !c.is_ascii_graphic() && !c.is_ascii_whitespace())
        .count();

    // If more than 30% of characters are non-printable, consider it binary
    (non_printable as f64 / total_chars as f64) > 0.3
}

/// Generate a diff in Codex CLI format
pub fn generate_codex_diff(
    old_content: &str,
    new_content: &str,
    file_path: &str,
) -> Result<String, PatchError> {
    let diff = generate_diff(old_content, new_content, file_path)?;
    
    if !diff.has_changes() {
        return Ok(String::new());
    }

    let mut result = String::new();
    result.push_str("*** Begin Patch\n");
    result.push_str(&format!("*** Update File: {}\n", file_path));
    result.push_str(&diff.to_codex_format());
    result.push_str("*** End Patch");
    
    Ok(result)
}

/// Utility to generate diffs for multiple files
pub struct MultipleDiffGenerator {
    config: DiffConfig,
}

impl MultipleDiffGenerator {
    /// Create a new multiple diff generator
    pub fn new(config: DiffConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(DiffConfig::default())
    }

    /// Generate diffs for multiple file pairs
    pub fn generate_diffs(
        &self,
        file_pairs: &[(String, String)], // (old_path, new_path) pairs
    ) -> Result<Vec<UnifiedDiff>, PatchError> {
        let mut diffs = Vec::new();
        
        for (old_path, new_path) in file_pairs {
            let diff = generate_file_diff(old_path, new_path, &self.config)?;
            if diff.has_changes() {
                diffs.push(diff);
            }
        }
        
        Ok(diffs)
    }

    /// Generate a combined diff output for multiple files
    pub fn generate_combined_diff(
        &self,
        file_pairs: &[(String, String)],
    ) -> Result<String, PatchError> {
        let diffs = self.generate_diffs(file_pairs)?;
        
        let mut result = String::new();
        for diff in diffs {
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str(&diff.to_string());
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_diff_generation() {
        let old_content = "line 1\nline 2\nline 3\n";
        let new_content = "line 1\nmodified line 2\nline 3\n";
        
        let diff = generate_diff(old_content, new_content, "test.txt").unwrap();
        
        assert!(diff.has_changes());
        assert_eq!(diff.chunks.len(), 1);
        assert_eq!(diff.additions(), 1);
        assert_eq!(diff.deletions(), 1);
    }

    #[test]
    fn test_no_changes_diff() {
        let content = "line 1\nline 2\nline 3\n";
        
        let diff = generate_diff(content, content, "test.txt").unwrap();
        
        assert!(!diff.has_changes());
        assert_eq!(diff.chunks.len(), 0);
    }

    #[test]
    fn test_binary_detection() {
        assert!(!is_binary_content("This is text content"));
        assert!(!is_binary_content("Mixed content with symbols: @#$%"));
        assert!(is_binary_content("Binary\0content"));
        
        // Test high ratio of non-printable characters
        let binary_like = String::from_iter((0u8..=255u8).map(|b| b as char));
        assert!(is_binary_content(&binary_like));
    }

    #[test]
    fn test_addition_only_diff() {
        let old_content = "line 1\nline 2\n";
        let new_content = "line 1\nline 2\nnew line 3\n";
        
        let diff = generate_diff(old_content, new_content, "test.txt").unwrap();
        
        assert!(diff.has_changes());
        assert_eq!(diff.additions(), 1);
        assert_eq!(diff.deletions(), 0);
    }

    #[test]
    fn test_deletion_only_diff() {
        let old_content = "line 1\nline 2\nline 3\n";
        let new_content = "line 1\nline 3\n";
        
        let diff = generate_diff(old_content, new_content, "test.txt").unwrap();
        
        assert!(diff.has_changes());
        assert_eq!(diff.additions(), 0);
        assert_eq!(diff.deletions(), 1);
    }

    #[test]
    fn test_codex_format_output() {
        let old_content = "original line\n";
        let new_content = "modified line\n";
        
        let codex_diff = generate_codex_diff(old_content, new_content, "test.txt").unwrap();
        
        assert!(codex_diff.contains("*** Begin Patch"));
        assert!(codex_diff.contains("*** Update File: test.txt"));
        assert!(codex_diff.contains("*** End Patch"));
        assert!(codex_diff.contains("-original line"));
        assert!(codex_diff.contains("+modified line"));
    }

    #[test]
    fn test_diff_validation() {
        let old_content = "line1\nline2\nline3\n";
        let new_content = "line1\nmodified\nline3\nnew line\n";
        
        let diff = generate_diff(old_content, new_content, "test.txt").unwrap();
        
        // Validation should pass for properly generated diffs
        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_context_lines_configuration() {
        let old_content = "line1\nline2\nline3\nline4\nline5\nline6\n";
        let new_content = "line1\nline2\nmodified\nline4\nline5\nline6\n";
        
        let config_small = DiffConfig { context_lines: 1, ..Default::default() };
        let config_large = DiffConfig { context_lines: 5, ..Default::default() };
        
        let diff_small = generate_diff_with_config(old_content, new_content, "test.txt", &config_small).unwrap();
        let diff_large = generate_diff_with_config(old_content, new_content, "test.txt", &config_large).unwrap();
        
        // With more context lines, we should see more context operations
        let small_context_ops = diff_small.chunks[0].operations.iter()
            .filter(|op| matches!(op, PatchOperation::Context { .. }))
            .count();
        let large_context_ops = diff_large.chunks[0].operations.iter()
            .filter(|op| matches!(op, PatchOperation::Context { .. }))
            .count();
        
        assert!(large_context_ops >= small_context_ops);
    }
}