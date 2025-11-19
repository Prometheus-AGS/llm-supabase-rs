// src/features/diff_patch/mod.rs
//
// Unified diff and patch operations module

pub mod models;
pub mod diff_generator;
pub mod patch_applicator;
pub mod error_integration;

// Re-export main types and functions for convenience
pub use models::{
    DiffChunk, DiffConfig, DiffMetadata, PatchConfig, PatchError, PatchOperation, UnifiedDiff,
};

pub use diff_generator::{
    generate_diff, generate_diff_with_config, generate_file_diff, generate_codex_diff,
    MultipleDiffGenerator,
};

pub use patch_applicator::{
    apply_patch, apply_patch_with_config, apply_patch_to_file, parse_unified_diff,
    rollback_patch,
};

use std::path::Path;

/// High-level diff and patch operations
pub struct DiffPatchManager {
    diff_config: DiffConfig,
    patch_config: PatchConfig,
}

impl DiffPatchManager {
    /// Create a new diff/patch manager with default configurations
    pub fn new() -> Self {
        Self {
            diff_config: DiffConfig::default(),
            patch_config: PatchConfig::default(),
        }
    }

    /// Create a manager with custom configurations
    pub fn with_configs(diff_config: DiffConfig, patch_config: PatchConfig) -> Self {
        Self {
            diff_config,
            patch_config,
        }
    }

    /// Generate a diff between two strings
    pub fn create_diff(
        &self,
        old_content: &str,
        new_content: &str,
        file_path: &str,
    ) -> Result<UnifiedDiff, PatchError> {
        generate_diff_with_config(old_content, new_content, file_path, &self.diff_config)
    }

    /// Apply a patch to content
    pub fn apply_patch(
        &self,
        original_content: &str,
        diff: &UnifiedDiff,
    ) -> Result<String, PatchError> {
        apply_patch_with_config(original_content, diff, &self.patch_config)
    }

    /// Generate and apply a patch in one operation
    pub fn patch_content(
        &self,
        old_content: &str,
        new_content: &str,
        file_path: &str,
    ) -> Result<String, PatchError> {
        let diff = self.create_diff(old_content, new_content, file_path)?;
        self.apply_patch(old_content, &diff)
    }

    /// Create a diff between two files
    pub fn create_file_diff(
        &self,
        old_path: &str,
        new_path: &str,
    ) -> Result<UnifiedDiff, PatchError> {
        generate_file_diff(old_path, new_path, &self.diff_config)
    }

    /// Apply a patch to a file
    pub fn apply_patch_to_file(
        &self,
        file_path: &str,
        diff: &UnifiedDiff,
    ) -> Result<(), PatchError> {
        apply_patch_to_file(file_path, diff, &self.patch_config)
    }

    /// Generate a patch in Codex CLI format
    pub fn create_codex_patch(
        &self,
        old_content: &str,
        new_content: &str,
        file_path: &str,
    ) -> Result<String, PatchError> {
        generate_codex_diff(old_content, new_content, file_path)
    }

    /// Parse a unified diff from text and apply it to a file
    pub fn apply_patch_from_text(
        &self,
        patch_text: &str,
        target_file: &str,
    ) -> Result<(), PatchError> {
        let diffs = parse_unified_diff(patch_text)?;
        
        for diff in diffs {
            if diff.metadata.new_path == target_file || diff.metadata.old_path == target_file {
                self.apply_patch_to_file(target_file, &diff)?;
                return Ok(());
            }
        }
        
        Err(PatchError::FileNotFound {
            path: target_file.to_string(),
        })
    }

    /// Validate that a patch can be applied to content
    pub fn validate_patch(
        &self,
        original_content: &str,
        diff: &UnifiedDiff,
    ) -> Result<(), PatchError> {
        // Try to apply the patch in a dry run
        self.apply_patch(original_content, diff)?;
        Ok(())
    }

    /// Get statistics about a diff
    pub fn get_diff_stats(&self, diff: &UnifiedDiff) -> DiffStats {
        DiffStats {
            additions: diff.additions(),
            deletions: diff.deletions(),
            modifications: diff.chunks.len(),
            files_changed: 1,
            is_binary: diff.metadata.is_binary,
        }
    }

    /// Set allowed directories for patch operations
    pub fn set_allowed_directories(&mut self, directories: Vec<String>) {
        self.patch_config.allowed_directories = directories;
    }

    /// Enable or disable fuzzy matching
    pub fn set_fuzzy_matching(&mut self, enabled: bool) {
        self.patch_config.fuzzy_matching = enabled;
    }

    /// Set context lines for diff generation
    pub fn set_context_lines(&mut self, lines: usize) {
        self.diff_config.context_lines = lines;
    }
}

impl Default for DiffPatchManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a diff operation
#[derive(Debug, Clone)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
    pub files_changed: usize,
    pub is_binary: bool,
}

impl DiffStats {
    /// Get the total number of changes
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.total_changes() > 0
    }
}

/// Utility functions for common diff/patch operations
pub mod utils {
    use super::*;

    /// Quick diff generation with default settings
    pub fn quick_diff(old: &str, new: &str, path: &str) -> Result<String, PatchError> {
        let manager = DiffPatchManager::default();
        let diff = manager.create_diff(old, new, path)?;
        Ok(diff.to_string())
    }

    /// Quick patch application with default settings
    pub fn quick_patch(content: &str, patch_text: &str) -> Result<String, PatchError> {
        let diffs = parse_unified_diff(patch_text)?;
        if diffs.is_empty() {
            return Ok(content.to_string());
        }
        
        let manager = DiffPatchManager::default();
        manager.apply_patch(content, &diffs[0])
    }

    /// Check if two strings are similar enough to be considered a match
    pub fn are_similar(s1: &str, s2: &str, threshold: f64) -> bool {
        crate::features::diff_patch::patch_applicator::fuzzy_line_match(s1, s2, threshold)
    }

    /// Generate a simple diff summary
    pub fn diff_summary(old: &str, new: &str) -> String {
        let old_lines = old.lines().count();
        let new_lines = new.lines().count();
        
        if old_lines == new_lines {
            format!("Modified {} lines", old_lines)
        } else if old_lines < new_lines {
            format!("Added {} lines ({} -> {})", new_lines - old_lines, old_lines, new_lines)
        } else {
            format!("Removed {} lines ({} -> {})", old_lines - new_lines, old_lines, new_lines)
        }
    }

    /// Create a safe file path for patch operations
    pub fn sanitize_file_path(path: &str) -> Result<String, PatchError> {
        let path_obj = Path::new(path);
        
        // Check for directory traversal
        if path.contains("..") {
            return Err(PatchError::SecurityViolation {
                path: path.to_string(),
            });
        }
        
        // Normalize the path
        Ok(path_obj.to_string_lossy().to_string())
    }

    /// Check if a file is likely to be binary based on its extension
    pub fn is_likely_binary(file_path: &str) -> bool {
        let binary_extensions = [
            "exe", "dll", "so", "dylib", "bin", "obj", "o", "a", "lib",
            "jpg", "jpeg", "png", "gif", "bmp", "ico", "tiff", "webp",
            "mp3", "mp4", "wav", "avi", "mov", "wmv", "flv", "mkv",
            "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
            "zip", "rar", "7z", "tar", "gz", "bz2", "xz",
        ];
        
        if let Some(extension) = Path::new(file_path).extension() {
            if let Some(ext_str) = extension.to_str() {
                return binary_extensions.contains(&ext_str.to_lowercase().as_str());
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_patch_manager() {
        let manager = DiffPatchManager::default();
        
        let old_content = "line 1\nline 2\nline 3\n";
        let new_content = "line 1\nmodified line 2\nline 3\nline 4\n";
        
        // Create diff
        let diff = manager.create_diff(old_content, new_content, "test.txt").unwrap();
        assert!(diff.has_changes());
        
        // Apply patch
        let result = manager.apply_patch(old_content, &diff).unwrap();
        assert_eq!(result, new_content);
    }

    #[test]
    fn test_diff_stats() {
        let manager = DiffPatchManager::default();
        
        let old_content = "line 1\nline 2\nline 3\n";
        let new_content = "line 1\nmodified line 2\nline 3\nline 4\n";
        
        let diff = manager.create_diff(old_content, new_content, "test.txt").unwrap();
        let stats = manager.get_diff_stats(&diff);
        
        assert!(stats.has_changes());
        assert_eq!(stats.additions, 2); // modified line + new line
        assert_eq!(stats.deletions, 1); // original line 2
        assert!(stats.modifications > 0);
    }

    #[test]
    fn test_codex_format_generation() {
        let manager = DiffPatchManager::default();
        
        let old_content = "original\n";
        let new_content = "modified\n";
        
        let codex_patch = manager.create_codex_patch(old_content, new_content, "test.txt").unwrap();
        
        assert!(codex_patch.contains("*** Begin Patch"));
        assert!(codex_patch.contains("*** Update File: test.txt"));
        assert!(codex_patch.contains("*** End Patch"));
    }

    #[test]
    fn test_utils_quick_functions() {
        let old = "hello\nworld\n";
        let new = "hello\nrustacean\n";
        
        let diff_text = utils::quick_diff(old, new, "test.txt").unwrap();
        assert!(diff_text.contains("-world"));
        assert!(diff_text.contains("+rustacean"));
        
        let patched = utils::quick_patch(old, &diff_text).unwrap();
        assert_eq!(patched, new);
    }

    #[test]
    fn test_file_extension_detection() {
        assert!(utils::is_likely_binary("image.jpg"));
        assert!(utils::is_likely_binary("program.exe"));
        assert!(!utils::is_likely_binary("script.rs"));
        assert!(!utils::is_likely_binary("document.txt"));
    }

    #[test]
    fn test_path_sanitization() {
        assert!(utils::sanitize_file_path("../etc/passwd").is_err());
        assert!(utils::sanitize_file_path("normal/path.txt").is_ok());
    }

    #[test]
    fn test_diff_summary() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        
        let summary = utils::diff_summary(old, new);
        assert!(summary.contains("Added 1 lines"));
    }
}