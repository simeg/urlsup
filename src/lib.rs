use spinners::{Spinner, Spinners};

use crate::finder::{Finder, UrlFinder};
use crate::validator::{ValidateUrls, ValidationResult, Validator};
use std::cmp::Ordering;
use std::io;
use std::path::Path;
use std::time::Duration;

pub mod finder;
pub mod validator;

pub struct UrlsUp {
    finder: Finder,
    validator: Validator,
}

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
        Self { finder, validator }
    }

    pub async fn run(
        &self,
        paths: Vec<&Path>,
        opts: UrlsUpOptions,
    ) -> io::Result<Vec<ValidationResult>> {
        println!("> Using threads: {}", &opts.thread_count);
        println!("> Using timeout (seconds): {}", &opts.timeout.as_secs());
        println!("> Allow timeout: {}", &opts.allow_timeout);

        if let Some(white_list) = &opts.white_list {
            println!("> Ignoring white listed URL(s)");
            for (i, url) in white_list.iter().enumerate() {
                println!("{:4}. {}", i + 1, url.to_string());
            }
        }

        if let Some(allowed) = &opts.allowed_status_codes {
            println!("> Allowing HTTP status codes");
            for (i, status_code) in allowed.iter().enumerate() {
                println!("{:4}. {}", i + 1, status_code.to_string());
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

        let spinner_find_urls = self.spinner_start("Finding URLs in files...".to_string());

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

        if let Some(sp) = spinner_find_urls {
            sp.stop();
        }

        println!(
            "\n\n> Found {} unique URL(s), {} in total",
            &dedup_urls.len(),
            url_count
        );

        for (i, ul) in dedup_urls.iter().enumerate() {
            println!("{:4}. {}", i + 1, ul.url.to_string());
        }

        println!(); // Make output more readable

        let validation_spinner = self.spinner_start("Checking URLs...".into());

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

        if let Some(sp) = validation_spinner {
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
            println!("{}", msg);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn test_dedup() {
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
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
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
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
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
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
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
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
    use mockito::mock;
    use std::io::Write;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn test_run__has_no_issues() -> TestResult {
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_secs(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let actual = urls_up.run(vec![file.path()], opts).await?;

        assert!(actual.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_run__has_issues() -> TestResult {
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_secs(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let _m = mock("GET", "/404").with_status(404).create();
        let endpoint = mockito::server_url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let result = urls_up.run(vec![file.path()], opts).await?;

        assert!(!result.is_empty());

        let actual = result.first().unwrap();

        assert_eq!(actual.description, None);
        assert_eq!(actual.url, "http://127.0.0.1:1234/404".to_string());
        assert_eq!(actual.status_code, Some(404));
        Ok(())
    }

    #[tokio::test]
    async fn test_run__issues_when_timeout_reached() -> TestResult {
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_nanos(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let result = urls_up.run(vec![file.path()], opts).await?;

        assert!(!result.is_empty());

        let actual = result.first().unwrap();

        assert_eq!(actual.description, Some("operation timed out".to_string()));
        assert_eq!(actual.url, "http://127.0.0.1:1234/200".to_string());
        assert_eq!(actual.status_code, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_run__no_issues_when_timeout_reached_and_allow_timeout() -> TestResult {
        let urls_up = UrlsUp::new(Finder::default(), Validator::default());
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_nanos(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: true,
        };
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let actual = urls_up.run(vec![file.path()], opts).await?;

        assert!(actual.is_empty());
        Ok(())
    }
}
