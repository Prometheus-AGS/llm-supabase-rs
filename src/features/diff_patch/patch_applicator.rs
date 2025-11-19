// src/features/diff_patch/patch_applicator.rs
//
// Unified diff patch application implementation

use crate::features::diff_patch::models::{
    DiffChunk, PatchConfig, PatchError, PatchOperation, UnifiedDiff,
};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Apply a unified diff to a string content
pub fn apply_patch(
    original_content: &str,
    diff: &UnifiedDiff,
) -> Result<String, PatchError> {
    apply_patch_with_config(original_content, diff, &PatchConfig::default())
}

/// Apply a unified diff with custom configuration
pub fn apply_patch_with_config(
    original_content: &str,
    diff: &UnifiedDiff,
    config: &PatchConfig,
) -> Result<String, PatchError> {
    if diff.metadata.is_binary {
        return Err(PatchError::BinaryFileNotSupported {
            path: diff.metadata.old_path.clone(),
        });
    }

    if diff.chunks.is_empty() {
        return Ok(original_content.to_string());
    }

    // Validate the diff first
    diff.validate()?;

    // Split content into lines for processing
    let mut lines: Vec<String> = original_content.lines()
        .map(|line| line.to_string())
        .collect();

    // Apply chunks in reverse order to maintain line numbers
    let mut chunks = diff.chunks.clone();
    chunks.sort_by(|a, b| b.old_start.cmp(&a.old_start));

    for chunk in &chunks {
        apply_chunk_to_lines(&mut lines, chunk, config)?;
    }

    // Reconstruct the content
    let mut result = lines.join("\n");
    if original_content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

/// Apply a single diff chunk to a vector of lines
fn apply_chunk_to_lines(
    lines: &mut Vec<String>,
    chunk: &DiffChunk,
    config: &PatchConfig,
) -> Result<(), PatchError> {
    let mut line_offset = 0isize;
    let mut current_old_line = chunk.old_start;
    let mut operation_index = 0;

    // Find the actual starting position (with fuzzy matching if enabled)
    let actual_start = if config.fuzzy_matching {
        find_fuzzy_match(lines, chunk, config.max_fuzz)?
    } else {
        chunk.old_start
    };

    let adjustment = actual_start as isize - chunk.old_start as isize;

    while operation_index < chunk.operations.len() {
        let operation = &chunk.operations[operation_index];
        let target_line_index = ((current_old_line as isize + adjustment - 1 + line_offset) as usize)
            .min(lines.len().saturating_sub(1));

        match operation {
            PatchOperation::Context { content } => {
                // Verify context matches (with fuzzy matching if enabled)
                if target_line_index < lines.len() {
                    if config.fuzzy_matching {
                        if !fuzzy_line_match(&lines[target_line_index], content, 0.8) {
                            return Err(PatchError::ContextMismatch {
                                message: format!(
                                    "Context mismatch at line {}: expected '{}', found '{}'",
                                    current_old_line, content, 
                                    lines.get(target_line_index).unwrap_or(&String::new())
                                ),
                            });
                        }
                    } else if lines[target_line_index] != *content {
                        return Err(PatchError::LineMismatch {
                            line_number: current_old_line,
                            expected: content.clone(),
                            actual: lines[target_line_index].clone(),
                        });
                    }
                }
                current_old_line += 1;
            }
            PatchOperation::Remove { content } => {
                // Verify the line to be removed matches
                if target_line_index >= lines.len() {
                    return Err(PatchError::LineMismatch {
                        line_number: current_old_line,
                        expected: content.clone(),
                        actual: "<EOF>".to_string(),
                    });
                }

                if config.fuzzy_matching {
                    if !fuzzy_line_match(&lines[target_line_index], content, 0.8) {
                        return Err(PatchError::LineMismatch {
                            line_number: current_old_line,
                            expected: content.clone(),
                            actual: lines[target_line_index].clone(),
                        });
                    }
                } else if lines[target_line_index] != *content {
                    return Err(PatchError::LineMismatch {
                        line_number: current_old_line,
                        expected: content.clone(),
                        actual: lines[target_line_index].clone(),
                    });
                }

                // Remove the line
                lines.remove(target_line_index);
                line_offset -= 1;
                current_old_line += 1;
            }
            PatchOperation::Add { content } => {
                // Insert the new line
                let insert_index = if target_line_index >= lines.len() {
                    lines.len()
                } else {
                    target_line_index
                };
                
                lines.insert(insert_index, content.clone());
                line_offset += 1;
            }
        }

        operation_index += 1;
    }

    Ok(())
}

/// Find the best match for a chunk with fuzzy matching
fn find_fuzzy_match(
    lines: &[String],
    chunk: &DiffChunk,
    max_fuzz: usize,
) -> Result<usize, PatchError> {
    let original_start = chunk.old_start;
    let search_start = original_start.saturating_sub(max_fuzz);
    let search_end = (original_start + max_fuzz).min(lines.len());

    let mut best_match = None;
    let mut best_score = 0.0;

    for start_pos in search_start..=search_end {
        let score = calculate_match_score(lines, chunk, start_pos);
        if score > best_score {
            best_score = score;
            best_match = Some(start_pos);
        }
    }

    if let Some(pos) = best_match {
        if best_score > 0.6 { // Minimum match threshold
            Ok(pos)
        } else {
            Err(PatchError::ContextMismatch {
                message: format!(
                    "No suitable match found for chunk starting at line {} (best score: {:.2})",
                    original_start, best_score
                ),
            })
        }
    } else {
        Ok(original_start)
    }
}

/// Calculate match score for a chunk at a given position
fn calculate_match_score(lines: &[String], chunk: &DiffChunk, start_pos: usize) -> f64 {
    let mut matches = 0;
    let mut total_context = 0;
    let mut line_index = start_pos;

    for operation in &chunk.operations {
        match operation {
            PatchOperation::Context { content } | PatchOperation::Remove { content } => {
                total_context += 1;
                if line_index < lines.len() && fuzzy_line_match(&lines[line_index], content, 0.8) {
                    matches += 1;
                }
                if matches!(operation, PatchOperation::Context { .. }) || 
                   matches!(operation, PatchOperation::Remove { .. }) {
                    line_index += 1;
                }
            }
            PatchOperation::Add { .. } => {
                // Add operations don't affect matching
            }
        }
    }

    if total_context == 0 {
        1.0
    } else {
        matches as f64 / total_context as f64
    }
}

/// Fuzzy line matching with similarity threshold
pub fn fuzzy_line_match(line1: &str, line2: &str, threshold: f64) -> bool {
    if line1 == line2 {
        return true;
    }

    let similarity = calculate_similarity(line1, line2);
    similarity >= threshold
}

/// Calculate similarity between two strings using Levenshtein distance
fn calculate_similarity(s1: &str, s2: &str) -> f64 {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    let distance = levenshtein_distance(s1, s2);
    let max_len = std::cmp::max(len1, len2);
    
    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1,      // deletion
                    matrix[i][j - 1] + 1,      // insertion
                ),
                matrix[i - 1][j - 1] + cost,   // substitution
            );
        }
    }

    matrix[len1][len2]
}

/// Apply a patch to a file
pub fn apply_patch_to_file(
    file_path: &str,
    diff: &UnifiedDiff,
    config: &PatchConfig,
) -> Result<(), PatchError> {
    // Validate file path for security
    if config.validate_paths {
        validate_file_path(file_path, &config.allowed_directories)?;
    }

    let path = Path::new(file_path);
    
    // Check if file exists
    if !path.exists() {
        return Err(PatchError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    // Read original content
    let original_content = fs::read_to_string(path)
        .map_err(|e| PatchError::IoError { 
            message: format!("Failed to read {}: {}", file_path, e) 
        })?;

    // Create backup if requested
    if config.create_backup {
        create_backup_file(path, &config.backup_extension)?;
    }

    // Apply the patch
    let patched_content = apply_patch_with_config(&original_content, diff, config)?;

    // Write the result
    fs::write(path, patched_content)
        .map_err(|e| PatchError::IoError { 
            message: format!("Failed to write {}: {}", file_path, e) 
        })?;

    Ok(())
}

/// Create a backup file
fn create_backup_file(original_path: &Path, backup_extension: &str) -> Result<(), PatchError> {
    let backup_path = format!("{}{}", 
        original_path.to_string_lossy(), 
        backup_extension
    );
    
    fs::copy(original_path, &backup_path)
        .map_err(|e| PatchError::BackupFailed { 
            message: format!("Failed to create backup {}: {}", backup_path, e) 
        })?;

    Ok(())
}

/// Validate file path for security
fn validate_file_path(
    file_path: &str, 
    allowed_directories: &[String]
) -> Result<(), PatchError> {
    let path = Path::new(file_path);
    
    // Check for directory traversal
    if file_path.contains("..") {
        return Err(PatchError::SecurityViolation {
            path: file_path.to_string(),
        });
    }

    // Convert to canonical path
    let canonical_path = path.canonicalize()
        .map_err(|_| PatchError::FileNotFound {
            path: file_path.to_string(),
        })?;

    // Check against allowed directories
    if !allowed_directories.is_empty() {
        let mut allowed = false;
        for allowed_dir in allowed_directories {
            let allowed_canonical = Path::new(allowed_dir).canonicalize()
                .map_err(|_| PatchError::SecurityViolation {
                    path: format!("Invalid allowed directory: {}", allowed_dir),
                })?;
            
            if canonical_path.starts_with(&allowed_canonical) {
                allowed = true;
                break;
            }
        }
        
        if !allowed {
            return Err(PatchError::SecurityViolation {
                path: file_path.to_string(),
            });
        }
    }

    Ok(())
}

/// Parse a unified diff from text
pub fn parse_unified_diff(diff_text: &str) -> Result<Vec<UnifiedDiff>, PatchError> {
    let mut diffs = Vec::new();
    let lines: Vec<&str> = diff_text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for file headers
        if lines[i].starts_with("---") && i + 1 < lines.len() && lines[i + 1].starts_with("+++") {
            let (diff, next_index) = parse_single_diff(&lines, i)?;
            diffs.push(diff);
            i = next_index;
        } else {
            i += 1;
        }
    }

    Ok(diffs)
}

/// Parse a single unified diff from lines
fn parse_single_diff(lines: &[&str], start_index: usize) -> Result<(UnifiedDiff, usize), PatchError> {
    if start_index + 1 >= lines.len() {
        return Err(PatchError::InvalidPatchFormat {
            message: "Incomplete diff header".to_string(),
        });
    }

    // Parse file headers
    let old_header = lines[start_index];
    let new_header = lines[start_index + 1];

    if !old_header.starts_with("---") || !new_header.starts_with("+++") {
        return Err(PatchError::InvalidPatchFormat {
            message: "Invalid diff headers".to_string(),
        });
    }

    let old_path = extract_file_path(old_header)?;
    let new_path = extract_file_path(new_header)?;
    
    let metadata = crate::features::diff_patch::models::DiffMetadata::new(old_path, new_path);

    // Parse chunks
    let mut chunks = Vec::new();
    let mut i = start_index + 2;

    while i < lines.len() {
        if lines[i].starts_with("@@") {
            let (chunk, next_index) = parse_chunk(&lines, i)?;
            chunks.push(chunk);
            i = next_index;
        } else if lines[i].starts_with("---") {
            // Start of next diff
            break;
        } else {
            i += 1;
        }
    }

    let diff = UnifiedDiff::new(metadata, chunks);
    Ok((diff, i))
}

/// Extract file path from header line
fn extract_file_path(header_line: &str) -> Result<String, PatchError> {
    let parts: Vec<&str> = header_line.splitn(2, '\t').collect();
    let path_part = if parts.len() > 1 { parts[0] } else { header_line };
    
    let path = if path_part.starts_with("---") {
        path_part[3..].trim()
    } else if path_part.starts_with("+++") {
        path_part[3..].trim()
    } else {
        return Err(PatchError::InvalidPatchFormat {
            message: format!("Invalid header line: {}", header_line),
        });
    };

    Ok(path.to_string())
}

/// Parse a single diff chunk
fn parse_chunk(lines: &[&str], start_index: usize) -> Result<(DiffChunk, usize), PatchError> {
    let hunk_header = lines[start_index];
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(hunk_header)?;

    let mut operations = Vec::new();
    let mut i = start_index + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("@@") {
            // Start of next chunk
            break;
        } else if line.starts_with("---") {
            // Start of next diff
            break;
        } else if line.len() > 0 {
            let op_char = line.chars().next().unwrap();
            let content = if line.len() > 1 { &line[1..] } else { "" };

            let operation = match op_char {
                ' ' => PatchOperation::context(content.to_string()),
                '+' => PatchOperation::add(content.to_string()),
                '-' => PatchOperation::remove(content.to_string()),
                _ => {
                    return Err(PatchError::InvalidPatchFormat {
                        message: format!("Invalid operation character: {}", op_char),
                    });
                }
            };
            
            operations.push(operation);
        }
        
        i += 1;
    }

    let chunk = DiffChunk::new(old_start, old_count, new_start, new_count, operations);
    chunk.validate()?;
    
    Ok((chunk, i))
}

/// Parse hunk header (e.g., "@@ -1,5 +1,6 @@")
fn parse_hunk_header(header: &str) -> Result<(usize, usize, usize, usize), PatchError> {
    let re = Regex::new(r"@@\s*-(\d+),(\d+)\s*\+(\d+),(\d+)\s*@@")
        .map_err(|e| PatchError::InvalidPatchFormat {
            message: format!("Regex error: {}", e),
        })?;

    if let Some(captures) = re.captures(header) {
        let old_start: usize = captures[1].parse()
            .map_err(|_| PatchError::InvalidPatchFormat {
                message: "Invalid old start line number".to_string(),
            })?;
        let old_count: usize = captures[2].parse()
            .map_err(|_| PatchError::InvalidPatchFormat {
                message: "Invalid old count".to_string(),
            })?;
        let new_start: usize = captures[3].parse()
            .map_err(|_| PatchError::InvalidPatchFormat {
                message: "Invalid new start line number".to_string(),
            })?;
        let new_count: usize = captures[4].parse()
            .map_err(|_| PatchError::InvalidPatchFormat {
                message: "Invalid new count".to_string(),
            })?;

        Ok((old_start, old_count, new_start, new_count))
    } else {
        Err(PatchError::InvalidPatchFormat {
            message: format!("Invalid hunk header: {}", header),
        })
    }
}

/// Rollback a patch application by restoring from backup
pub fn rollback_patch(file_path: &str, backup_extension: &str) -> Result<(), PatchError> {
    let backup_path = format!("{}{}", file_path, backup_extension);
    let backup_path_obj = Path::new(&backup_path);
    
    if !backup_path_obj.exists() {
        return Err(PatchError::BackupFailed {
            message: format!("Backup file not found: {}", backup_path),
        });
    }

    fs::copy(&backup_path, file_path)
        .map_err(|e| PatchError::IoError {
            message: format!("Failed to restore from backup: {}", e),
        })?;

    // Optionally remove the backup file
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::diff_patch::models::{DiffMetadata, PatchOperation};

    #[test]
    fn test_apply_simple_patch() {
        let original = "line 1\nline 2\nline 3\n";
        let operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::remove("line 2".to_string()),
            PatchOperation::add("modified line 2".to_string()),
            PatchOperation::context("line 3".to_string()),
        ];
        
        let chunk = DiffChunk::new(1, 3, 1, 3, operations);
        let metadata = DiffMetadata::single_file("test.txt".to_string());
        let diff = UnifiedDiff::new(metadata, vec![chunk]);

        let result = apply_patch(original, &diff).unwrap();
        assert_eq!(result, "line 1\nmodified line 2\nline 3\n");
    }

    #[test]
    fn test_apply_addition_patch() {
        let original = "line 1\nline 2\n";
        let operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::context("line 2".to_string()),
            PatchOperation::add("line 3".to_string()),
        ];
        
        let chunk = DiffChunk::new(1, 2, 1, 3, operations);
        let metadata = DiffMetadata::single_file("test.txt".to_string());
        let diff = UnifiedDiff::new(metadata, vec![chunk]);

        let result = apply_patch(original, &diff).unwrap();
        assert_eq!(result, "line 1\nline 2\nline 3\n");
    }

    #[test]
    fn test_apply_deletion_patch() {
        let original = "line 1\nline 2\nline 3\n";
        let operations = vec![
            PatchOperation::context("line 1".to_string()),
            PatchOperation::remove("line 2".to_string()),
            PatchOperation::context("line 3".to_string()),
        ];
        
        let chunk = DiffChunk::new(1, 3, 1, 2, operations);
        let metadata = DiffMetadata::single_file("test.txt".to_string());
        let diff = UnifiedDiff::new(metadata, vec![chunk]);

        let result = apply_patch(original, &diff).unwrap();
        assert_eq!(result, "line 1\nline 3\n");
    }

    #[test]
    fn test_fuzzy_line_matching() {
        assert!(fuzzy_line_match("hello world", "hello world", 0.8));
        assert!(fuzzy_line_match("hello world", "hello word", 0.8)); // Small typo
        assert!(!fuzzy_line_match("hello world", "goodbye world", 0.8)); // Too different
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
    }

    #[test]
    fn test_parse_hunk_header() {
        let (old_start, old_count, new_start, new_count) = 
            parse_hunk_header("@@ -1,5 +1,6 @@").unwrap();
        assert_eq!(old_start, 1);
        assert_eq!(old_count, 5);
        assert_eq!(new_start, 1);
        assert_eq!(new_count, 6);
    }

    #[test]
    fn test_validate_file_path() {
        let config = PatchConfig {
            validate_paths: true,
            allowed_directories: vec!["/tmp".to_string()],
            ..Default::default()
        };

        // Should fail - directory traversal
        assert!(validate_file_path("../etc/passwd", &config.allowed_directories).is_err());
    }

    #[test]
    fn test_calculate_similarity() {
        assert_eq!(calculate_similarity("abc", "abc"), 1.0);
        assert_eq!(calculate_similarity("", ""), 1.0);
        assert!(calculate_similarity("abc", "ab") > 0.5);
        assert!(calculate_similarity("abc", "def") < 0.5);
    }
}