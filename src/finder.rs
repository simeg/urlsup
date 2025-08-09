use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks::UTF8;
use linkify::{LinkFinder, LinkKind};
use memchr::memchr_iter;
use once_cell::sync::Lazy;
use rayon::prelude::*;

use crate::{
    UrlLocation,
    constants::files,
    error::{Result, UrlsUpError},
    types::UrlLocationError,
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
        use crate::path_utils::expand_paths;
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
        use crate::path_utils::expand_paths;
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
}
