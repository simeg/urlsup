use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks::UTF8;
use linkify::{LinkFinder, LinkKind};
use memchr::memchr_iter;
use once_cell::sync::Lazy;
use rayon::prelude::*;

use crate::{
    UrlLocation,
    core::constants::files,
    core::error::{Result, UrlsUpError},
    core::types::UrlLocationError,
};

use std::{io, path::Path};

const MARKDOWN_URL_PATTERN: &str =
    r#"(http://|https://)[a-z0-9]+([-.]{1}[a-z0-9]+)*(.[a-z]{2,5})?(:[0-9]{1,5})?(/.*)?"#;

static REGEX_MATCHER: Lazy<RegexMatcher> = Lazy::new(|| {
    RegexMatcher::new(MARKDOWN_URL_PATTERN).expect("Failed to compile URL regex pattern")
});

// Reuse LinkFinder instance for better performance
static LINK_FINDER: Lazy<LinkFinder> = Lazy::new(|| {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    finder
});

pub trait UrlFinder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>>;
}

#[derive(Default, Debug)]
pub struct Finder {}

impl UrlFinder for Finder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>> {
        // Use parallel processing for file reading and URL extraction
        let results: std::result::Result<Vec<Vec<UrlLocation>>, io::Error> = paths
            .par_iter()
            .map(|path| -> io::Result<Vec<UrlLocation>> {
                let url_matches = Self::parse_lines_with_urls(path)?;
                let estimated_capacity = Self::estimate_url_capacity(path, url_matches.len());
                let mut file_urls = Vec::with_capacity(estimated_capacity);

                for url_match in url_matches {
                    // Handle parse_urls error by converting to IO error
                    let url_locations = Self::parse_urls(url_match)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    file_urls.extend(url_locations);
                }

                Ok(file_urls)
            })
            .collect();

        // Flatten the results into a single vector
        let file_results = results?;
        let total_capacity: usize = file_results.iter().map(|v| v.len()).sum();
        let mut result = Vec::with_capacity(total_capacity);

        for file_urls in file_results {
            result.extend(file_urls);
        }

        Ok(result)
    }
}

type UrlMatch = (String, String, u64);

impl Finder {
    /// Fast vectorized search for URL-like patterns in file content
    /// Uses SIMD-optimized memchr for rapid pattern scanning
    #[allow(dead_code)]
    fn fast_url_scan(content: &[u8]) -> Vec<usize> {
        let mut potential_urls = Vec::new();

        // SIMD-optimized search for 'h' characters (start of http/https)
        for pos in memchr_iter(b'h', content) {
            // Quick check if this could be the start of http:// or https://
            if pos + 7 <= content.len() {
                let slice = &content[pos..pos + 7];
                if slice.starts_with(b"http://")
                    || (pos + 8 <= content.len() && content[pos..pos + 8].starts_with(b"https://"))
                {
                    potential_urls.push(pos);
                }
            }
        }

        potential_urls
    }

    /// Vectorized line processing for better performance on large files
    #[allow(dead_code)]
    fn process_lines_vectorized(content: &str) -> Vec<(String, u64)> {
        let lines: Vec<&str> = content.lines().collect();
        let mut results = Vec::with_capacity(lines.len());

        // Process lines in chunks for better cache locality
        const CHUNK_SIZE: usize = 64;

        lines
            .chunks(CHUNK_SIZE)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                for (line_idx, line) in chunk.iter().enumerate() {
                    let line_number = (chunk_idx * CHUNK_SIZE + line_idx + 1) as u64;

                    // Quick SIMD check for potential URLs before expensive regex
                    if memchr::memchr(b'h', line.as_bytes()).is_some() ||
                       memchr::memchr(b'w', line.as_bytes()).is_some() || // www
                       memchr::memchr(b'f', line.as_bytes()).is_some()
                    {
                        // ftp
                        results.push((line.to_string(), line_number));
                    }
                }
            });

        results
    }

    /// Estimate URL capacity based on file extension and initial match count
    fn estimate_url_capacity(path: &Path, match_count: usize) -> usize {
        if match_count == 0 {
            return 0;
        }

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let multiplier = match extension {
            "md" | "markdown" => 2, // Markdown files often have multiple URLs per line
            "html" | "htm" => 3,    // HTML files may have many URLs per match
            "txt" | "rst" => 1,     // Plain text files usually have fewer URLs
            "json" | "xml" => 2,    // Config files may have API endpoints
            _ => files::ESTIMATED_URLS_PER_MATCH,
        };

        // Use a minimum capacity and scale with match count
        (match_count.saturating_mul(multiplier)).max(4)
    }

    /// Parse lines from a file that contain URLs based on regex pattern matching.
    ///
    /// This is the first stage of URL finding - it quickly identifies lines
    /// that are likely to contain URLs using regex.
    fn parse_lines_with_urls(path: &Path) -> io::Result<Vec<UrlMatch>> {
        let mut matches = Vec::with_capacity(files::DEFAULT_URL_CAPACITY_PER_FILE); // Pre-allocate for estimated URLs per file

        Searcher::new().search_path(
            &*REGEX_MATCHER,
            path,
            UTF8(|line_number, line| {
                let file_name = path.display().to_string();
                let url_match: UrlMatch = (line.to_string(), file_name, line_number);
                matches.push(url_match);
                Ok(true)
            }),
        )?;

        Ok(matches)
    }

    /// Parse URLs from a line of text and return valid UrlLocation instances.
    ///
    /// This method extracts actual URLs from matched lines using the linkify crate
    /// and creates properly validated UrlLocation instances.
    fn parse_urls(url_match: UrlMatch) -> Result<Vec<UrlLocation>> {
        let (line_content, file_name, line) = url_match;

        // Use the static LinkFinder for better performance
        let mut url_locations = Vec::new();

        for link in LINK_FINDER.links(&line_content) {
            match UrlLocation::new(link.as_str().to_string(), line, file_name.clone()) {
                Ok(location) => url_locations.push(location),
                Err(UrlLocationError::MissingUrl) => {
                    // Skip empty URLs - this shouldn't happen with linkify, but be defensive
                    continue;
                }
                Err(UrlLocationError::InvalidLineNumber) => {
                    // This indicates a bug in our code since we control the line number
                    return Err(UrlsUpError::Validation(format!(
                        "Invalid line number {line} for URL in file {file_name}"
                    )));
                }
                Err(UrlLocationError::MissingFileName) => {
                    // This indicates a bug in our code since we control the file name
                    return Err(UrlsUpError::Validation(format!(
                        "Invalid file name for URL {} at line {}",
                        link.as_str(),
                        line
                    )));
                }
                Err(UrlLocationError::MissingLine) => {
                    // This shouldn't happen since we always provide a line number
                    return Err(UrlsUpError::Validation(
                        "Missing line number - this is a bug".to_string(),
                    ));
                }
            }
        }

        Ok(url_locations)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use std::io::Write;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_parse_urls() {
        let md_link =
            "arbitrary [something](http://foo.bar) arbitrary http://foo2.bar arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![
            UrlLocation::new_unchecked(
                "http://foo.bar".to_string(),
                99,
                "this-file-name".to_string(),
            ),
            UrlLocation::new_unchecked(
                "http://foo2.bar".to_string(),
                99,
                "this-file-name".to_string(),
            ),
        ];
        let actual = Finder::parse_urls(url_match).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_urls__img_url() {
        let md_link = "arbitrary ![image](http://foo.bar) arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![UrlLocation::new_unchecked(
            "http://foo.bar".to_string(),
            99,
            "this-file-name".to_string(),
        )];
        let actual = Finder::parse_urls(url_match).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_urls__badge_url() {
        let md_link = "arbitrary [something]: http://foo.bar arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![UrlLocation::new_unchecked(
            "http://foo.bar".to_string(),
            99,
            "this-file-name".to_string(),
        )];
        let actual = Finder::parse_urls(url_match).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_lines_with_urls__from_file() -> TestResult {
        let mut file = tempfile::NamedTempFile::new()?;
        let file_name = file.path().display().to_string();
        file.write_all(
            "arbitrary [something](http://specific-link.one) arbitrary\n\
             arbitrary [something](http://specific-link.two) arbitrary\n\
             arbitrary [badge-something]: http://specific-link.three arbitrary\n\
             arbitrary http://specific-link.four arbitrary"
                .as_bytes(),
        )?;

        let actual = Finder::parse_lines_with_urls(file.path())?;

        let actual_match1 = actual.first().unwrap().to_owned();
        let actual_match2 = actual.get(1).unwrap().to_owned();
        let actual_match3 = actual.get(2).unwrap().to_owned();
        let actual_match4 = actual.get(3).unwrap().to_owned();

        assert_eq!(
            actual_match1,
            (
                "arbitrary [something](http://specific-link.one) arbitrary\n".to_string(),
                file_name.to_string(),
                1
            )
        );
        assert_eq!(
            actual_match2,
            (
                "arbitrary [something](http://specific-link.two) arbitrary\n".to_string(),
                file_name.to_string(),
                2
            )
        );
        assert_eq!(
            actual_match3,
            (
                "arbitrary [badge-something]: http://specific-link.three arbitrary\n".to_string(),
                file_name.to_string(),
                3
            )
        );
        assert_eq!(
            actual_match4,
            (
                "arbitrary http://specific-link.four arbitrary".to_string(),
                file_name.to_string(),
                4
            )
        );

        Ok(())
    }

    #[test]
    fn test_parse_lines_with_urls__from_file__when_non_existing_file() {
        let non_existing_file = "non_existing_file.txt";
        let is_err = Finder::parse_lines_with_urls(non_existing_file.as_ref()).is_err();

        assert!(is_err);
    }

    #[test]
    fn test_find_urls__multiple_files() -> TestResult {
        let mut file1 = tempfile::NamedTempFile::new()?;
        let mut file2 = tempfile::NamedTempFile::new()?;

        file1.write_all("First file with https://example.com".as_bytes())?;
        file2.write_all("Second file with https://test.org and https://demo.net".as_bytes())?;

        let finder = Finder::default();
        let paths = vec![file1.path(), file2.path()];
        let result = finder.find_urls(paths)?;

        assert_eq!(result.len(), 3);

        let urls: Vec<&str> = result.iter().map(|ul| ul.url()).collect();
        assert!(urls.contains(&"https://example.com"));
        assert!(urls.contains(&"https://test.org"));
        assert!(urls.contains(&"https://demo.net"));

        Ok(())
    }

    #[test]
    fn test_find_urls__with_recursive_structure() -> TestResult {
        use crate::discovery::path_utils::expand_paths;
        use std::collections::HashSet;
        use std::fs;

        // Create test directory structure
        let temp_dir = tempfile::tempdir()?;
        let base = temp_dir.path();

        fs::create_dir_all(base.join("docs"))?;
        fs::create_dir_all(base.join("src"))?;

        // Create files with URLs
        fs::write(
            base.join("README.md"),
            "# Project\nWebsite: https://project.com\nRepo: https://github.com/user/repo",
        )?;
        fs::write(
            base.join("docs/guide.md"),
            "Documentation at https://docs.example.com",
        )?;
        fs::write(base.join("src/main.rs"), "// See: https://rust-lang.org")?;

        // Use expand_paths to get all markdown files recursively
        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let expanded_paths = expand_paths(vec![base], true, Some(&extensions))?;
        let paths: Vec<&std::path::Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

        let finder = Finder::default();
        let result = finder.find_urls(paths)?;

        // Should find URLs from markdown files only
        assert_eq!(result.len(), 3); // project.com, github.com, docs.example.com

        let urls: Vec<&str> = result.iter().map(|ul| ul.url()).collect();
        assert!(urls.contains(&"https://project.com"));
        assert!(urls.contains(&"https://github.com/user/repo"));
        assert!(urls.contains(&"https://docs.example.com"));

        // Should not find rust-lang.org since .rs files were filtered out
        assert!(!urls.contains(&"https://rust-lang.org"));

        Ok(())
    }

    #[test]
    fn test_find_urls__empty_directory() -> TestResult {
        use crate::discovery::path_utils::expand_paths;
        use std::collections::HashSet;

        let temp_dir = tempfile::tempdir()?;

        // Create directory with no matching files
        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let expanded_paths = expand_paths(vec![temp_dir.path()], true, Some(&extensions))?;
        let paths: Vec<&std::path::Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

        let finder = Finder::default();
        let result = finder.find_urls(paths)?;

        assert_eq!(result.len(), 0);
        Ok(())
    }

    #[test]
    fn test_parse_urls__multiple_urls_same_line() {
        let line = "Visit https://example.com and also http://test.org for more info".to_string();
        let url_match = (line, "test.md".to_string(), 5);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].url(), "https://example.com");
        assert_eq!(result[0].line(), 5);
        assert_eq!(result[0].file_name(), "test.md");

        assert_eq!(result[1].url(), "http://test.org");
        assert_eq!(result[1].line(), 5);
        assert_eq!(result[1].file_name(), "test.md");
    }

    #[test]
    fn test_parse_urls__url_with_query_params() {
        let line = "API endpoint: https://api.example.com/v1/users?id=123&format=json".to_string();
        let url_match = (line, "api.md".to_string(), 10);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].url(),
            "https://api.example.com/v1/users?id=123&format=json"
        );
    }

    #[test]
    fn test_parse_urls__url_with_fragments() {
        let line = "Docs: https://example.com/docs#installation".to_string();
        let url_match = (line, "readme.md".to_string(), 2);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url(), "https://example.com/docs#installation");
    }

    #[test]
    fn test_parse_urls__url_with_port() {
        let line = "Local server: http://localhost:8080/api".to_string();
        let url_match = (line, "config.md".to_string(), 1);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url(), "http://localhost:8080/api");
    }

    #[test]
    fn test_parse_urls__no_urls_in_line() {
        let line = "This line has no URLs, just regular text.".to_string();
        let url_match = (line, "empty.md".to_string(), 3);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_urls__urls_in_code_blocks() {
        let line = "`curl https://api.example.com/data`".to_string();
        let url_match = (line, "code.md".to_string(), 7);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url(), "https://api.example.com/data");
    }

    #[test]
    fn test_parse_urls__malformed_urls() {
        let line = "Broken: ht://bad-url and htp://another-bad".to_string();
        let url_match = (line, "broken.md".to_string(), 1);

        let result = Finder::parse_urls(url_match).unwrap();
        // linkify actually finds some URLs from malformed input
        // "ht://bad-url" and "htp://another-bad" are detected as URLs
        assert_eq!(result.len(), 2);

        // Verify the URLs that were found
        assert_eq!(result[0].url(), "ht://bad-url");
        assert_eq!(result[1].url(), "htp://another-bad");
    }

    #[test]
    fn test_parse_urls__urls_with_special_chars() {
        let line = "Search: https://example.com/search?q=rust%20programming&sort=date".to_string();
        let url_match = (line, "search.md".to_string(), 4);

        let result = Finder::parse_urls(url_match).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].url(),
            "https://example.com/search?q=rust%20programming&sort=date"
        );
    }

    #[test]
    fn test_estimate_url_capacity() {
        // Test capacity estimation for different file types
        let temp_dir = tempfile::tempdir().unwrap();

        // Create test files
        let md_file = temp_dir.path().join("test.md");
        let txt_file = temp_dir.path().join("test.txt");
        let html_file = temp_dir.path().join("test.html");
        let other_file = temp_dir.path().join("test.unknown");

        std::fs::write(&md_file, "content").unwrap();
        std::fs::write(&txt_file, "content").unwrap();
        std::fs::write(&html_file, "content").unwrap();
        std::fs::write(&other_file, "content").unwrap();

        // Markdown files should get 2x multiplier
        let md_capacity = Finder::estimate_url_capacity(&md_file, 10);
        assert_eq!(md_capacity, 20);

        // HTML files should get 3x multiplier
        let html_capacity = Finder::estimate_url_capacity(&html_file, 10);
        assert_eq!(html_capacity, 30);

        // TXT files should get 1x multiplier
        let txt_capacity = Finder::estimate_url_capacity(&txt_file, 10);
        assert_eq!(txt_capacity, 10);

        // Other files should get 2x multiplier (ESTIMATED_URLS_PER_MATCH = 2)
        let other_capacity = Finder::estimate_url_capacity(&other_file, 10);
        assert_eq!(other_capacity, 20);
    }

    #[test]
    fn test_parse_lines_with_urls_empty_file() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let empty_file = temp_dir.path().join("empty.md");
        std::fs::write(&empty_file, "")?;

        let result = Finder::parse_lines_with_urls(&empty_file)?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[test]
    fn test_parse_lines_with_urls_no_matches() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let file = temp_dir.path().join("no_urls.md");
        std::fs::write(
            &file,
            "This file has no URLs\nJust regular text\nNo links at all",
        )?;

        let result = Finder::parse_lines_with_urls(&file)?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[test]
    fn test_find_urls_nonexistent_file() {
        let finder = Finder::default();
        let nonexistent = std::path::Path::new("/definitely/does/not/exist.md");

        let result = finder.find_urls(vec![nonexistent]);
        // Should return an error or empty result, not panic
        let _ = result;
    }

    #[test]
    fn test_find_urls_with_different_extensions() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let base = temp_dir.path();

        // Create files with different extensions
        std::fs::write(base.join("readme.md"), "Markdown: https://md.example.com")?;
        std::fs::write(base.join("notes.txt"), "Text: https://txt.example.com")?;
        std::fs::write(base.join("page.html"), "HTML: https://html.example.com")?;
        std::fs::write(
            base.join("config.json"),
            r#"{"url": "https://json.example.com"}"#,
        )?;

        let readme_path = base.join("readme.md");
        let notes_path = base.join("notes.txt");
        let html_path = base.join("page.html");
        let json_path = base.join("config.json");

        let files = vec![
            readme_path.as_path(),
            notes_path.as_path(),
            html_path.as_path(),
            json_path.as_path(),
        ];

        let finder = Finder::default();
        let result = finder.find_urls(files)?;

        assert_eq!(result.len(), 4);

        let urls: Vec<&str> = result.iter().map(|ul| ul.url()).collect();
        assert!(urls.contains(&"https://md.example.com"));
        assert!(urls.contains(&"https://txt.example.com"));
        assert!(urls.contains(&"https://html.example.com"));
        assert!(urls.contains(&"https://json.example.com"));

        Ok(())
    }

    #[test]
    fn test_finder_default() {
        let finder = Finder::default();
        // Should create without panicking
        assert!(std::ptr::eq(&finder, &finder)); // Basic identity check
    }

    #[test]
    fn test_url_parsing_edge_cases() {
        let test_cases = vec![
            // URL at start of line
            ("https://start.example.com rest of line", 1),
            // URL at end of line
            ("beginning of line https://end.example.com", 1),
            // Multiple URLs separated by text
            (
                "Visit https://first.com then go to https://second.com for more",
                2,
            ),
            // URLs in parentheses
            ("See (https://example.com) for details", 1),
            // URLs in brackets
            ("Check [https://example.com] link", 1),
            // URLs with paths and queries
            (
                "API: https://api.example.com/v1/users?active=true&limit=50",
                1,
            ),
        ];

        for (line, expected_count) in test_cases {
            let url_match = (line.to_string(), "test.md".to_string(), 1);
            let result = Finder::parse_urls(url_match).unwrap();
            assert_eq!(result.len(), expected_count, "Failed for line: {}", line);
        }
    }
}
