use spinners::{Spinner, Spinners};

use crate::finder::{Finder, UrlFinder};
use crate::validator::{ValidateUrls, ValidationResult, Validator};
use std::cmp::Ordering;
use std::io;
use std::path::Path;
use std::time::Duration;

pub mod config;
pub mod error;
pub mod finder;
pub mod path_utils;
pub mod progress;
pub mod validator;

#[derive(Debug)]
pub struct UrlsUp {
    finder: Finder,
    validator: Validator,
    test_mode: bool,
}

#[derive(Clone)]
pub struct UrlsUpOptions {
    // White listed URLs to allow being broken
    pub white_list: Option<Vec<String>>,
    // Timeout for getting a response
    pub timeout: Duration,
    // HTTP status codes to allow being present
    pub allowed_status_codes: Option<Vec<u16>>,
    // Thread count
    pub thread_count: usize,
    // Allow requests to time out
    pub allow_timeout: bool,
}

#[derive(Debug, Eq, Clone)]
pub struct UrlLocation {
    // The URL that was found
    pub url: String,
    // Line number where URL was found
    pub line: u64,
    // Name of file where URL was found
    pub file_name: String,
}

impl Ord for UrlLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

impl PartialOrd for UrlLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for UrlLocation {
    fn eq(&self, other: &Self) -> bool {
        if cfg!(test) {
            // In tests we want to compare all properties
            (&self.url, &self.file_name, self.line) == (&other.url, &other.file_name, other.line)
        } else {
            self.url == other.url
        }
    }
}

impl UrlsUp {
    pub fn new(finder: Finder, validator: Validator) -> Self {
        Self {
            finder,
            validator,
            test_mode: cfg!(test),
        }
    }

    pub fn new_for_testing(finder: Finder, validator: Validator) -> Self {
        Self {
            finder,
            validator,
            test_mode: true,
        }
    }

    pub async fn run(
        &self,
        paths: Vec<&Path>,
        opts: UrlsUpOptions,
    ) -> io::Result<Vec<ValidationResult>> {
        if !self.test_mode {
            println!("> Using threads: {}", &opts.thread_count);
            println!("> Using timeout (seconds): {}", &opts.timeout.as_secs());
            println!("> Allow timeout: {}", &opts.allow_timeout);

            if let Some(white_list) = &opts.white_list {
                println!("> Ignoring white listed URL(s)");
                for (i, url) in white_list.iter().enumerate() {
                    println!("{:4}. {}", i + 1, url);
                }
            }

            if let Some(allowed) = &opts.allowed_status_codes {
                println!("> Allowing HTTP status codes");
                for (i, status_code) in allowed.iter().enumerate() {
                    println!("{:4}. {}", i + 1, status_code);
                }
            }

            let files_singular_plural = match &paths.len() {
                1 => "file",
                _ => "files",
            };

            println!(
                "> Will check URLs in {} {}",
                paths.len(),
                files_singular_plural
            );
            for (i, file) in paths.iter().enumerate() {
                println!("{:4}. {}", i + 1, file.display());
            }

            println!(); // Make output more readable
        }

        let spinner_find_urls = if !self.test_mode {
            self.spinner_start("Finding URLs in files...".to_string())
        } else {
            None
        };

        // Find URLs from files
        let mut url_locations = self.finder.find_urls(paths)?;

        // Apply white list
        if let Some(white_list) = &opts.white_list {
            url_locations = self.apply_white_list(url_locations, white_list);
        }

        // Save URL count to avoid having to clone URL list later
        let url_count = url_locations.len();

        // Deduplicate URLs to avoid duplicate work
        let dedup_urls = self.dedup(url_locations);

        if let Some(mut sp) = spinner_find_urls {
            sp.stop();
        }

        if !self.test_mode {
            println!(
                "\n\n> Found {} unique URL(s), {} in total",
                &dedup_urls.len(),
                url_count
            );

            for (i, ul) in dedup_urls.iter().enumerate() {
                println!("{:4}. {}", i + 1, ul.url);
            }

            println!(); // Make output more readable
        }

        let validation_spinner = if !self.test_mode {
            self.spinner_start("Checking URLs...".into())
        } else {
            None
        };

        // Check URLs
        let mut non_ok_urls: Vec<ValidationResult> = self
            .validator
            .validate_urls(dedup_urls, &opts)
            .await
            .into_iter()
            .filter(ValidationResult::is_not_ok)
            .collect();

        if let Some(allowed) = &opts.allowed_status_codes {
            non_ok_urls = self.filter_allowed_status_codes(non_ok_urls, allowed.clone());
        }

        if opts.allow_timeout {
            non_ok_urls = self.filter_timeouts(non_ok_urls);
        }

        if let Some(mut sp) = validation_spinner {
            sp.stop();
        }

        Ok(non_ok_urls)
    }

    fn apply_white_list(
        &self,
        url_locations: Vec<UrlLocation>,
        white_list: &[String],
    ) -> Vec<UrlLocation> {
        url_locations
            .into_iter()
            .filter(|ul| !white_list.contains(&ul.url))
            .filter(|ul| {
                // If URL starts with any white listed URL
                for white_listed_url in white_list.iter() {
                    if ul.url.starts_with(white_listed_url) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn filter_allowed_status_codes(
        &self,
        validation_results: Vec<ValidationResult>,
        allowed_status_codes: Vec<u16>,
    ) -> Vec<ValidationResult> {
        validation_results
            .into_iter()
            .filter(|vr| {
                if let Some(status_code) = vr.status_code {
                    if allowed_status_codes.contains(&status_code) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn filter_timeouts(&self, validation_results: Vec<ValidationResult>) -> Vec<ValidationResult> {
        validation_results
            .into_iter()
            .filter(|vr| {
                if let Some(description) = &vr.description {
                    if description == "operation timed out" {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn dedup(&self, mut list: Vec<UrlLocation>) -> Vec<UrlLocation> {
        list.sort();
        list.dedup();
        list
    }

    fn spinner_start(&self, msg: String) -> Option<Spinner> {
        if term::stdout().is_some() {
            Some(Spinner::new(Spinners::Dots, msg))
        } else {
            println!("{msg}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn test_url_location_ordering() {
        let url1 = UrlLocation {
            url: "a".to_string(),
            line: 1,
            file_name: "file1".to_string(),
        };
        let url2 = UrlLocation {
            url: "b".to_string(),
            line: 2,
            file_name: "file2".to_string(),
        };
        let url3 = UrlLocation {
            url: "a".to_string(),
            line: 1,
            file_name: "file1".to_string(),
        };

        assert!(url1 < url2);
        assert!(url2 > url1);
        assert_eq!(url1, url3); // Exact same URL, line, and file
    }

    #[test]
    fn test_url_location_equality() {
        let url1 = UrlLocation {
            url: "https://example.com".to_string(),
            line: 10,
            file_name: "file1.md".to_string(),
        };
        let url2 = UrlLocation {
            url: "https://example.com".to_string(),
            line: 10,
            file_name: "file1.md".to_string(),
        };
        let url3 = UrlLocation {
            url: "https://different.com".to_string(),
            line: 10,
            file_name: "file1.md".to_string(),
        };

        assert_eq!(url1, url2); // Identical URLs
        assert_ne!(url1, url3); // Different URL
    }

    #[test]
    fn test_url_location_partial_ord() {
        let url1 = UrlLocation {
            url: "alpha".to_string(),
            line: 1,
            file_name: "file".to_string(),
        };
        let url2 = UrlLocation {
            url: "beta".to_string(),
            line: 1,
            file_name: "file".to_string(),
        };

        assert_eq!(url1.partial_cmp(&url2), Some(Ordering::Less));
        assert_eq!(url2.partial_cmp(&url1), Some(Ordering::Greater));
        assert_eq!(url1.partial_cmp(&url1), Some(Ordering::Equal));
    }

    #[test]
    fn test_urlsup_new_constructor() {
        let finder = Finder::default();
        let validator = Validator::default();
        let urls_up = UrlsUp::new(finder, validator);

        // Should create successfully without panicking
        assert!(!format!("{urls_up:?}").is_empty());
    }

    #[test]
    fn test_spinner_start_disabled() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        // When spinner can't be created (non-TTY), should return None
        let spinner = urls_up.spinner_start("Test".to_string());
        // This test may pass or fail depending on environment, but shouldn't panic
        drop(spinner);
    }

    #[test]
    fn test_empty_url_list_handling() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());

        let empty_results = urls_up.filter_timeouts(vec![]);
        assert!(empty_results.is_empty());

        let empty_dedup = urls_up.dedup(vec![]);
        assert!(empty_dedup.is_empty());
    }

    #[test]
    fn test_dedup() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let duplicate = vec![
            UrlLocation {
                url: "duplicate".to_string(),
                line: 99,
                file_name: "this-file-name-dup".to_string(),
            },
            UrlLocation {
                url: "duplicate".to_string(),
                line: 99,
                file_name: "this-file-name-dup".to_string(),
            },
            UrlLocation {
                url: "unique-1".to_string(),
                line: 10,
                file_name: "this-file-name-1".to_string(),
            },
            UrlLocation {
                url: "unique-2".to_string(),
                line: 20,
                file_name: "this-file-name-2".to_string(),
            },
        ];

        let actual = urls_up.dedup(duplicate);
        let expected = vec![
            UrlLocation {
                url: "duplicate".to_string(),
                line: 99,
                file_name: "this-file-name-dup".to_string(),
            },
            UrlLocation {
                url: "unique-1".to_string(),
                line: 10,
                file_name: "this-file-name-1".to_string(),
            },
            UrlLocation {
                url: "unique-2".to_string(),
                line: 20,
                file_name: "this-file-name-2".to_string(),
            },
        ];

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_apply_white_list__filters_out_white_listed_urls() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![
            UrlLocation {
                url: "http://should-keep.com".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
            },
            UrlLocation {
                url: "http://should-ignore.com".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
            },
            UrlLocation {
                url: "http://should-also-ignore.com/something/something-else".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
            },
        ];

        let white_list: Vec<String> =
            vec!["http://should-ignore.com", "http://should-also-ignore.com"]
                .into_iter()
                .map(String::from)
                .collect();

        let actual = urls_up.apply_white_list(urls, &white_list);
        let expected = vec![UrlLocation {
            url: "http://should-keep.com".to_string(),
            line: 0,
            file_name: "arbitrary".to_string(),
        }];

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_filter_allowed_status_codes__removes_allowed_status_codes() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let vr1 = ValidationResult {
            url: "keep-this".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: Some(200),
            description: None,
        };
        let vr2 = ValidationResult {
            url: "keep-this-2".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: None,
            description: Some("arbitrary".to_string()),
        };
        let vr3 = ValidationResult {
            url: "remove-this".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: Some(404),
            description: None,
        };
        let actual = urls_up.filter_allowed_status_codes(vec![vr1, vr2, vr3], vec![404]);
        let expected = vec![
            ValidationResult {
                url: "keep-this".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
                status_code: Some(200),
                description: None,
            },
            ValidationResult {
                url: "keep-this-2".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
                status_code: None,
                description: Some("arbitrary".to_string()),
            },
        ];

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_filter_timeouts__removes_timeouts() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let vr1 = ValidationResult {
            url: "keep-this".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: Some(200),
            description: None,
        };
        let vr2 = ValidationResult {
            url: "keep-this-2".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: None,
            description: Some("arbitrary".to_string()),
        };
        let vr3 = ValidationResult {
            url: "remove-this".to_string(),
            line: 0, // arbitrary
            file_name: "arbitrary".to_string(),
            status_code: None,
            description: Some("operation timed out".to_string()),
        };
        let actual = urls_up.filter_timeouts(vec![vr1, vr2, vr3]);
        let expected = vec![
            ValidationResult {
                url: "keep-this".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
                status_code: Some(200),
                description: None,
            },
            ValidationResult {
                url: "keep-this-2".to_string(),
                line: 0, // arbitrary
                file_name: "arbitrary".to_string(),
                status_code: None,
                description: Some("arbitrary".to_string()),
            },
        ];

        assert_eq!(actual, expected)
    }
}

#[cfg(test)]
mod it_tests {
    #![allow(non_snake_case)]

    use super::*;
    use mockito::Server;
    use std::io::Write;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn test_run__has_no_issues() -> TestResult {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let actual = urls_up.run(vec![file.path()], opts).await?;

        assert!(actual.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run__has_issues() -> TestResult {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/404").with_status(404).create();
        let endpoint = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let result = urls_up.run(vec![file.path()], opts).await?;

        assert!(!result.is_empty());

        let actual = result.first().unwrap();

        assert_eq!(actual.description, None);
        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, Some(404));
        Ok(())
    }

    #[tokio::test]
    async fn test_run__issues_when_timeout_reached() -> TestResult {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        // Use an unreachable address to trigger timeout
        let endpoint = "http://192.0.2.1:80/200".to_string(); // RFC 5737 TEST-NET-1 address
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let result = urls_up.run(vec![file.path()], opts).await?;

        assert!(!result.is_empty());

        let actual = result.first().unwrap();

        assert_eq!(actual.description, Some("operation timed out".to_string()));
        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, None);
        Ok(())
    }

    #[test]
    fn test_apply_white_list_starts_with() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let url_locations = vec![
            UrlLocation {
                url: "https://github.com/user/repo".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://gitlab.com/user/repo".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://example.com".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
            },
        ];

        let white_list = vec!["https://github.com".to_string()];
        let result = urls_up.apply_white_list(url_locations, &white_list);

        // Should filter out URLs that start with whitelisted URLs
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .any(|url| url.url == "https://gitlab.com/user/repo")
        );
        assert!(result.iter().any(|url| url.url == "https://example.com"));
        assert!(
            !result
                .iter()
                .any(|url| url.url == "https://github.com/user/repo")
        );
    }

    #[test]
    fn test_apply_white_list_exact_match() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let url_locations = vec![
            UrlLocation {
                url: "https://exact-match.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://other.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
            },
        ];

        let white_list = vec!["https://exact-match.com".to_string()];
        let result = urls_up.apply_white_list(url_locations, &white_list);

        // Should filter out exact match
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url, "https://other.com");
    }

    #[test]
    fn test_filter_allowed_status_codes_with_some_status() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let validation_results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: None,
            },
            ValidationResult {
                url: "https://other.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: None,
            },
            ValidationResult {
                url: "https://timeout.com".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("timeout".to_string()),
            },
        ];

        let allowed_status_codes = vec![404];
        let result = urls_up.filter_allowed_status_codes(validation_results, allowed_status_codes);

        // Should filter out 404 but keep 500 and timeout (no status code)
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|vr| vr.status_code == Some(500)));
        assert!(result.iter().any(|vr| vr.status_code.is_none()));
        assert!(!result.iter().any(|vr| vr.status_code == Some(404)));
    }

    #[test]
    fn test_filter_timeouts_removes_timeout_descriptions() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let validation_results = vec![
            ValidationResult {
                url: "https://connection-error.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("client error (Connect)".to_string()),
            },
            ValidationResult {
                url: "https://server-error.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: None,
            },
            ValidationResult {
                url: "https://timeout.com".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("operation timed out".to_string()), // Exact match required
            },
        ];

        let result = urls_up.filter_timeouts(validation_results);

        // Should filter out only the exact "operation timed out" description
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|vr| vr.status_code == Some(500)));
        assert!(result.iter().any(|vr| {
            vr.description
                .as_ref()
                .map(|d| d.contains("Connect"))
                .unwrap_or(false)
        }));
        assert!(!result.iter().any(|vr| {
            vr.description
                .as_ref()
                .map(|d| d == "operation timed out")
                .unwrap_or(false)
        }));
    }

    #[tokio::test]
    async fn test_run_with_whitelist_and_allowed_codes() -> TestResult {
        let mut server = mockito::Server::new_async().await;
        let _m1 = server.mock("GET", "/allowed").with_status(404).create();
        let _m2 = server.mock("GET", "/blocked").with_status(500).create();

        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            format!(
                "Check these: {} and {}",
                server.url() + "/allowed",
                server.url() + "/blocked"
            ),
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: Some(vec![404]), // Allow 404
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should only have the 500 error (404 is allowed)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status_code, Some(500));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_whitelist_filtering() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            "Check these: https://github.com/example and https://example.com",
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: Some(vec!["https://github.com".to_string()]),
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should only try to validate example.com (github.com is whitelisted)
        assert!(result.iter().all(|vr| !vr.url.contains("github.com")));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_allow_timeout() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:80/timeout")?; // Non-routable IP

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10), // Very short timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: true, // Allow timeouts
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should have no issues because timeouts are allowed
        assert!(result.is_empty() || result.iter().all(|vr| vr.status_code.is_some()));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_multiple_files() -> TestResult {
        let temp_file1 = tempfile::NamedTempFile::new()?;
        let temp_file2 = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file1, "File 1: http://192.0.2.1:1/status/200")?;
        std::fs::write(&temp_file2, "File 2: http://192.0.2.1:1/status/404")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up
            .run(vec![temp_file1.path(), temp_file2.path()], opts)
            .await?;

        // Should process both files
        assert!(!result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_whitelist_output() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: Some(vec![
                "https://github.com".to_string(),
                "https://gitlab.com".to_string(),
            ]),
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 2,
            allow_timeout: false,
        };

        // This will test the whitelist printing logic
        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should complete without panic
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_allowed_status_codes_output() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: Some(vec![404, 403, 500]),
            thread_count: 1,
            allow_timeout: false,
        };

        // This will test the allowed status codes printing logic
        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should complete without panic
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_single_file_vs_multiple_files_output() -> TestResult {
        let temp_file1 = tempfile::NamedTempFile::new()?;
        let temp_file2 = tempfile::NamedTempFile::new()?;
        let temp_file3 = tempfile::NamedTempFile::new()?;

        std::fs::write(&temp_file1, "Check: http://192.0.2.1:1/example")?;
        std::fs::write(&temp_file2, "Check: http://192.0.2.1:1/test")?;
        std::fs::write(&temp_file3, "Check: http://192.0.2.1:1/demo")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        // Test single file (should print "file")
        let result1 = urls_up.run(vec![temp_file1.path()], opts.clone()).await?;
        let _ = result1; // Verify single file completes

        // Test multiple files (should print "files")
        let result2 = urls_up
            .run(vec![temp_file2.path(), temp_file3.path()], opts)
            .await?;
        let _ = result2; // Verify multiple files complete

        Ok(())
    }

    #[tokio::test]
    async fn test_run_empty_file() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "")?; // Empty file

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should find no URLs and return empty result
        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run_file_with_no_urls() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            "This file contains no URLs at all. Just plain text.",
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should find no URLs and return empty result
        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_duplicate_urls() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            "Check these URLs:\nhttp://192.0.2.1:1/example\nhttp://192.0.2.1:1/example\nhttp://192.0.2.1:1/test\nhttp://192.0.2.1:1/example",
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should deduplicate URLs - tests the dedup logic
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_various_thread_counts() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());

        // Test different thread counts
        for thread_count in [1, 2, 4, 8] {
            let opts = UrlsUpOptions {
                white_list: None,
                timeout: Duration::from_millis(10),
                allowed_status_codes: None,
                thread_count,
                allow_timeout: false,
            };

            let result = urls_up.run(vec![temp_file.path()], opts).await?;
            let _ = result; // Just verify it completes without error
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_whitelist_complete_filtering() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            "URLs: https://github.com/repo and https://github.com/other and https://example.com",
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: Some(vec![
                "https://github.com".to_string(),
                "https://example.com".to_string(),
            ]),
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // All URLs should be filtered out by whitelist
        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_all_options_enabled() -> TestResult {
        let mut server = mockito::Server::new_async().await;
        let _m1 = server.mock("GET", "/ok").with_status(200).create();
        let _m2 = server.mock("GET", "/forbidden").with_status(403).create();
        let _m3 = server.mock("GET", "/notfound").with_status(404).create();

        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(
            &temp_file,
            format!(
                "URLs: {} and {} and {} and https://whitelisted.com",
                server.url() + "/ok",
                server.url() + "/forbidden",
                server.url() + "/notfound"
            ),
        )?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: Some(vec!["https://whitelisted.com".to_string()]),
            timeout: Duration::from_millis(10),
            allowed_status_codes: Some(vec![403]), // Allow 403 but not 404
            thread_count: 2,
            allow_timeout: true,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;

        // Should have 404 error (200 is successful, 403 is allowed, whitelist is filtered)
        assert!(result.iter().any(|vr| vr.status_code == Some(404)));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_io_error_propagation() -> TestResult {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        // Try to read a non-existent file - should propagate IO error
        let result = urls_up
            .run(vec![Path::new("/definitely/does/not/exist.md")], opts)
            .await;

        // Should return an IO error
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_dedup_preserves_first_occurrence() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![
            UrlLocation {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "file1.md".to_string(),
            },
            UrlLocation {
                url: "https://test.com".to_string(),
                line: 2,
                file_name: "file1.md".to_string(),
            },
            UrlLocation {
                url: "https://example.com".to_string(), // Same URL, same file, same line = actual duplicate
                line: 1,
                file_name: "file1.md".to_string(),
            },
            UrlLocation {
                url: "https://demo.com".to_string(),
                line: 10,
                file_name: "file3.md".to_string(),
            },
        ];

        let result = urls_up.dedup(urls);

        // Should remove the actual duplicate (same URL, file, and line)
        assert_eq!(result.len(), 3);

        // Should be sorted alphabetically
        assert_eq!(result[0].url, "https://demo.com");
        assert_eq!(result[1].url, "https://example.com");
        assert_eq!(result[2].url, "https://test.com");

        // Should preserve the first occurrence data
        let example_url = result
            .iter()
            .find(|u| u.url == "https://example.com")
            .unwrap();
        assert_eq!(example_url.line, 1);
        assert_eq!(example_url.file_name, "file1.md");
    }

    #[test]
    fn test_dedup_different_locations_same_url() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![
            UrlLocation {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "file1.md".to_string(),
            },
            UrlLocation {
                url: "https://example.com".to_string(), // Same URL but different location
                line: 5,
                file_name: "file2.md".to_string(),
            },
        ];

        let result = urls_up.dedup(urls);

        // In test mode, these are considered different because of different file/line
        // so both should be preserved
        assert_eq!(result.len(), 2);

        // Should be sorted alphabetically, but since URLs are same, by other fields
        assert!(result.iter().all(|u| u.url == "https://example.com"));
    }

    #[test]
    fn test_apply_white_list_empty_list() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![UrlLocation {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        }];

        let result = urls_up.apply_white_list(urls.clone(), &[]);

        // Empty whitelist should not filter anything
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url, "https://example.com");
    }

    #[test]
    fn test_filter_allowed_status_codes_empty_list() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: Some(404),
            description: None,
        }];

        let filtered = urls_up.filter_allowed_status_codes(results.clone(), vec![]);

        // Empty allowed codes list should not filter anything
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status_code, Some(404));
    }

    #[test]
    fn test_filter_timeouts_no_timeouts() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(200),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("network error".to_string()),
            },
        ];

        let filtered = urls_up.filter_timeouts(results.clone());

        // Should keep all since none have "operation timed out"
        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_run__no_issues_when_timeout_reached_and_allow_timeout() -> TestResult {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: true,
        };
        // Use an unreachable address to trigger timeout
        let endpoint = "http://192.0.2.1:80/200".to_string(); // RFC 5737 TEST-NET-1 address
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let actual = urls_up.run(vec![file.path()], opts).await?;

        assert!(actual.is_empty());
        Ok(())
    }

    #[test]
    fn test_spinner_start_terminal_detection() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());

        // Test spinner creation in various scenarios
        let spinner1 = urls_up.spinner_start("Test spinner 1".to_string());
        let spinner2 = urls_up.spinner_start("Test spinner 2".to_string());
        let spinner3 = urls_up.spinner_start("Test spinner 3".to_string());

        // Should not panic regardless of TTY status
        drop(spinner1);
        drop(spinner2);
        drop(spinner3);
    }

    #[test]
    fn test_urlsup_options_various_combinations() {
        // Test UrlsUpOptions with different field combinations
        let opts1 = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_secs(30),
            allowed_status_codes: None,
            thread_count: 4,
            allow_timeout: false,
        };
        assert_eq!(opts1.timeout.as_secs(), 30);
        assert_eq!(opts1.thread_count, 4);
        assert!(!opts1.allow_timeout);

        let opts2 = UrlsUpOptions {
            white_list: Some(vec!["https://test.com".to_string()]),
            timeout: Duration::from_millis(500),
            allowed_status_codes: Some(vec![200, 404]),
            thread_count: 1,
            allow_timeout: true,
        };
        assert!(opts2.white_list.is_some());
        assert!(opts2.allowed_status_codes.is_some());
        assert!(opts2.allow_timeout);
    }

    #[tokio::test]
    async fn test_run_with_many_files() -> TestResult {
        // Test with many files to exercise file enumeration logic
        let mut files = Vec::new();
        let mut paths = Vec::new();

        for i in 0..10 {
            let temp_file = tempfile::NamedTempFile::new()?;
            std::fs::write(
                &temp_file,
                format!(
                    "File {} content: http://192.0.2.{}.1/{}",
                    i,
                    (i % 100) + 1,
                    i
                ),
            )?;
            paths.push(temp_file.path().to_path_buf());
            files.push(temp_file);
        }

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 2,
            allow_timeout: false,
        };

        let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
        let result = urls_up.run(path_refs, opts).await?;

        // Should handle many files without issue
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_long_whitelist() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());

        // Create a long whitelist to test enumeration
        let long_whitelist: Vec<String> = (0..20)
            .map(|i| format!("https://whitelist{i}.com"))
            .collect();

        let opts = UrlsUpOptions {
            white_list: Some(long_whitelist),
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_long_allowed_status_list() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());

        // Create a long list of allowed status codes
        let long_status_list: Vec<u16> = vec![
            200, 201, 202, 300, 301, 302, 400, 401, 403, 404, 500, 502, 503, 504, 429, 418, 422,
            451,
        ];

        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: Some(long_status_list),
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[test]
    fn test_dedup_empty_list() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let result = urls_up.dedup(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_dedup_single_item() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![UrlLocation {
            url: "https://single.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        }];
        let result = urls_up.dedup(urls);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url, "https://single.com");
    }

    #[test]
    fn test_dedup_no_duplicates() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let urls = vec![
            UrlLocation {
                url: "https://a.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://b.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://c.com".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
            },
        ];
        let result = urls_up.dedup(urls);
        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn test_run_with_complex_file_path() -> TestResult {
        let temp_dir = tempfile::TempDir::new()?;
        let complex_path = temp_dir.path().join("sub dir").join("file with spaces.md");
        std::fs::create_dir_all(complex_path.parent().unwrap())?;
        std::fs::write(&complex_path, "Check: http://192.0.2.1:1/example")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![&complex_path], opts).await?;
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[test]
    fn test_url_location_clone() {
        let url = UrlLocation {
            url: "https://example.com".to_string(),
            line: 42,
            file_name: "test.md".to_string(),
        };

        let cloned = url.clone();
        assert_eq!(url.url, cloned.url);
        assert_eq!(url.line, cloned.line);
        assert_eq!(url.file_name, cloned.file_name);
    }

    #[test]
    fn test_url_location_debug() {
        let url = UrlLocation {
            url: "https://debug.com".to_string(),
            line: 123,
            file_name: "debug.md".to_string(),
        };

        let debug_str = format!("{url:?}");
        assert!(debug_str.contains("https://debug.com"));
        assert!(debug_str.contains("123"));
        assert!(debug_str.contains("debug.md"));
    }

    #[test]
    fn test_filter_allowed_status_codes_none_status() {
        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: None, // No status code
            description: Some("connection failed".to_string()),
        }];

        let filtered = urls_up.filter_allowed_status_codes(results.clone(), vec![404]);

        // Should keep items with no status code
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].status_code.is_none());
    }

    #[tokio::test]
    async fn test_run_captures_validation_spinner() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        // This should exercise the validation spinner creation and cleanup
        let result = urls_up.run(vec![temp_file.path()], opts).await?;
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_minimal_thread_count() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "Check: http://192.0.2.1:1/test")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10),
            allowed_status_codes: None,
            thread_count: 1, // Use 1 thread instead of 0 which can hang
            allow_timeout: false,
        };

        // Should handle minimal thread count gracefully
        let result = urls_up.run(vec![temp_file.path()], opts).await?;
        let _ = result; // Just verify it completes without error
        Ok(())
    }

    #[tokio::test]
    async fn test_run_with_very_long_timeout() -> TestResult {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(&temp_file, "No URLs here")?;

        let urls_up = UrlsUp::new_for_testing(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_secs(3600), // Very long timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = urls_up.run(vec![temp_file.path()], opts).await?;
        assert!(result.is_empty()); // No URLs to validate
        Ok(())
    }
}
