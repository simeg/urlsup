use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Result, UrlsUpError};

pub fn expand_paths(
    input_paths: Vec<&Path>,
    recursive: bool,
    file_types: Option<&HashSet<String>>,
) -> Result<Vec<PathBuf>> {
    let mut result_paths = Vec::new();

    for path in input_paths {
        if path.is_file() {
            // Check file extension if filtering is enabled
            if let Some(extensions) = file_types {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(ext) {
                        result_paths.push(path.to_path_buf());
                    }
                } else if extensions.contains("") {
                    // Include files without extensions if "" is in the set
                    result_paths.push(path.to_path_buf());
                }
            } else {
                result_paths.push(path.to_path_buf());
            }
        } else if path.is_dir() && recursive {
            let mut builder = ignore::WalkBuilder::new(path);
            builder.hidden(false); // Include hidden files

            for entry in builder.build() {
                let entry = entry?;
                let entry_path = entry.path();

                if entry_path.is_file() {
                    // Check file extension if filtering is enabled
                    if let Some(extensions) = file_types {
                        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                            if extensions.contains(ext) {
                                result_paths.push(entry_path.to_path_buf());
                            }
                        } else if extensions.contains("") {
                            // Include files without extensions if "" is in the set
                            result_paths.push(entry_path.to_path_buf());
                        }
                    } else {
                        result_paths.push(entry_path.to_path_buf());
                    }
                }
            }
        } else if path.is_dir() && !recursive {
            return Err(UrlsUpError::PathExpansion(format!(
                "'{}' is a directory. Use --recursive to process directories.",
                path.display()
            )));
        }
    }

    Ok(result_paths)
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn create_test_structure() -> std::result::Result<TempDir, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let base = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(base.join("subdir/nested"))?;
        fs::create_dir_all(base.join("other"))?;

        // Create files with different extensions
        fs::write(base.join("README.md"), "# Test\nhttps://example.com")?;
        fs::write(base.join("file.txt"), "Some text with https://test.com")?;
        fs::write(
            base.join("script.sh"),
            "#!/bin/bash\necho https://shell.com",
        )?;
        fs::write(base.join("config.json"), r#"{"url": "https://json.com"}"#)?;
        fs::write(base.join("no_extension"), "https://noext.com")?;

        // Create nested files
        fs::write(
            base.join("subdir/nested/deep.md"),
            "Deep file https://deep.com",
        )?;
        fs::write(
            base.join("other/another.txt"),
            "Another https://another.com",
        )?;

        // Create .gitignore
        fs::write(base.join(".gitignore"), "*.log\ntmp/\n")?;

        // Create ignored files
        fs::write(base.join("debug.log"), "Should be ignored")?;
        fs::create_dir_all(base.join("tmp"))?;
        fs::write(base.join("tmp/temp.md"), "Should be ignored")?;

        Ok(temp_dir)
    }

    #[test]
    fn test_expand_paths__single_file() -> TestResult {
        let temp_dir = create_test_structure()?;
        let readme_path = temp_dir.path().join("README.md");

        let result = expand_paths(vec![&readme_path], false, None)?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], readme_path);
        Ok(())
    }

    #[test]
    fn test_expand_paths__file_with_extension_filter() -> TestResult {
        let temp_dir = create_test_structure()?;
        let readme_path = temp_dir.path().join("README.md");
        let txt_path = temp_dir.path().join("file.txt");

        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        // Should include .md file
        let result = expand_paths(vec![&readme_path], false, Some(&extensions))?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], readme_path);

        // Should exclude .txt file
        let result = expand_paths(vec![&txt_path], false, Some(&extensions))?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[test]
    fn test_expand_paths__directory_without_recursive_fails() -> TestResult {
        let temp_dir = create_test_structure()?;

        let result = expand_paths(vec![temp_dir.path()], false, None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("is a directory. Use --recursive")
        );
        Ok(())
    }

    #[test]
    fn test_expand_paths__recursive_all_files() -> TestResult {
        let temp_dir = create_test_structure()?;

        let result = expand_paths(vec![temp_dir.path()], true, None)?;

        // Should find all files in the directory structure
        // The exact count depends on gitignore behavior, but should find our main files
        assert!(result.len() >= 7); // At least the main files

        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // Check that we find the main files we created
        assert!(file_names.contains(&"README.md".to_string()));
        assert!(file_names.contains(&"file.txt".to_string()));
        assert!(file_names.contains(&"deep.md".to_string()));
        assert!(file_names.contains(&"another.txt".to_string()));

        Ok(())
    }

    #[test]
    fn test_expand_paths__recursive_with_file_type_filter() -> TestResult {
        let temp_dir = create_test_structure()?;

        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let result = expand_paths(vec![temp_dir.path()], true, Some(&extensions))?;

        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // Should only find markdown files
        assert!(file_names.contains(&"README.md".to_string()));
        assert!(file_names.contains(&"deep.md".to_string()));
        assert!(!file_names.contains(&"file.txt".to_string()));
        assert!(!file_names.contains(&"script.sh".to_string()));

        // All found files should have .md extension
        for path in &result {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                assert_eq!(ext, "md");
            }
        }

        Ok(())
    }

    #[test]
    fn test_expand_paths__multiple_extensions() -> TestResult {
        let temp_dir = create_test_structure()?;

        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());
        extensions.insert("txt".to_string());

        let result = expand_paths(vec![temp_dir.path()], true, Some(&extensions))?;

        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // Should find .md and .txt files
        assert!(file_names.contains(&"README.md".to_string()));
        assert!(file_names.contains(&"file.txt".to_string()));
        assert!(file_names.contains(&"deep.md".to_string()));
        assert!(file_names.contains(&"another.txt".to_string()));
        assert!(!file_names.contains(&"script.sh".to_string()));
        assert!(!file_names.contains(&"config.json".to_string()));

        // All found files should have .md or .txt extension
        for path in &result {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                assert!(ext == "md" || ext == "txt");
            }
        }

        Ok(())
    }

    #[test]
    fn test_expand_paths__files_without_extension() -> TestResult {
        let temp_dir = create_test_structure()?;

        let mut extensions = HashSet::new();
        extensions.insert("".to_string()); // Empty string means files without extension

        let result = expand_paths(vec![temp_dir.path()], true, Some(&extensions))?;

        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // Should find files without extensions
        assert!(file_names.contains(&"no_extension".to_string()));

        // All found files should have no extension
        for path in &result {
            assert!(path.extension().is_none());
        }

        Ok(())
    }

    #[test]
    fn test_expand_paths__mixed_files_and_directories() -> TestResult {
        let temp_dir = create_test_structure()?;
        let readme_path = temp_dir.path().join("README.md");
        let subdir_path = temp_dir.path().join("subdir");

        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let result = expand_paths(vec![&readme_path, &subdir_path], true, Some(&extensions))?;

        // Should find README.md directly and deep.md from subdir recursively
        assert_eq!(result.len(), 2);

        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"README.md".to_string()));
        assert!(file_names.contains(&"deep.md".to_string()));

        Ok(())
    }

    #[test]
    fn test_expand_paths__nonexistent_file() -> TestResult {
        let result = expand_paths(
            vec![Path::new("/definitely/nonexistent/path/file.md")],
            false,
            None,
        )?;
        // Non-existent files are simply not included in the result
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_expand_paths__permission_denied() -> TestResult {
        // This test simulates permission issues on paths that may not be accessible
        // On most systems, this will pass but provides coverage for error handling
        let result = expand_paths(vec![Path::new("/proc/1/mem")], false, None);
        // The result may succeed or fail depending on system, but shouldn't panic
        let _ = result;
        Ok(())
    }
}
