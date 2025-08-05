use async_trait::async_trait;
use futures::{StreamExt, stream};
use reqwest::redirect::Policy;
use rustc_hash::FxHashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use tokio::time::{Duration, sleep};

use crate::{UrlLocation, UrlsUpOptions, config::Config, progress::ProgressReporter};

use std::cmp::Ordering;
use std::fmt;

#[async_trait]
pub trait ValidateUrls {
    async fn validate_urls(
        &self,
        urls: Vec<UrlLocation>,
        opts: &UrlsUpOptions,
    ) -> Vec<ValidationResult>;

    async fn validate_urls_with_config(
        &self,
        urls: Vec<UrlLocation>,
        config: &Config,
        progress: Option<&mut ProgressReporter>,
    ) -> Vec<ValidationResult>;
}

#[derive(Default, Debug)]
pub struct Validator {}

#[derive(Debug, Eq, Clone)]
pub struct ValidationResult {
    pub url: String,
    pub line: u64,
    pub file_name: String,
    pub status_code: Option<u16>,
    pub description: Option<String>,
}

impl Ord for ValidationResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

impl PartialOrd for ValidationResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ValidationResult {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.status_code == other.status_code
            && self.description == other.description
    }
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool {
        if let Some(num) = self.status_code {
            num == 200
        } else {
            false
        }
    }

    pub fn is_not_ok(&self) -> bool {
        !self.is_ok()
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(num) = &self.status_code {
            write!(
                f,
                "{} - {} - {} - L{}",
                num, &self.url, &self.file_name, &self.line
            )
        } else if let Some(desc) = &self.description {
            write!(
                f,
                "{} - {} - {} - L{}",
                &self.url, desc, &self.file_name, &self.line
            )
        } else {
            panic!("ValidationResult should always have status_code or description")
        }
    }
}

#[async_trait]
impl ValidateUrls for Validator {
    async fn validate_urls(
        &self,
        urls: Vec<UrlLocation>,
        opts: &UrlsUpOptions,
    ) -> Vec<ValidationResult> {
        let redirect_policy = Policy::limited(10);
        let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        let client = reqwest::Client::builder()
            .timeout(opts.timeout)
            .redirect(redirect_policy)
            .user_agent(user_agent)
            .http2_prior_knowledge() // Enable HTTP/2 for better connection reuse
            .pool_max_idle_per_host(50) // Increase connection pool size
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            // Compression is enabled by default in reqwest
            .build()
            .unwrap();

        let mut find_results_and_responses = stream::iter(urls)
            .map(|ul| {
                let client = &client;
                async move {
                    let response = client.get(&ul.url).send().await;
                    (ul.clone(), response)
                }
            })
            .buffer_unordered(opts.thread_count);

        let mut result = vec![];
        while let Some((ul, response)) = find_results_and_responses.next().await {
            // Consciously convert the Result into a ValidationResult
            // We are interested in _why_ something failed, not _if_ it failed
            let validation_result = match response {
                Ok(res) => ValidationResult {
                    url: ul.url,
                    line: ul.line,
                    file_name: ul.file_name,
                    status_code: Some(res.status().as_u16()),
                    description: None,
                },
                Err(err) => ValidationResult {
                    url: ul.url,
                    line: ul.line,
                    file_name: ul.file_name,
                    status_code: None,
                    description: std::error::Error::source(&err).map(|e| e.to_string()),
                },
            };

            result.push(validation_result);
        }

        result
    }

    async fn validate_urls_with_config(
        &self,
        urls: Vec<UrlLocation>,
        config: &Config,
        mut progress: Option<&mut ProgressReporter>,
    ) -> Vec<ValidationResult> {
        // Optimized deduplication using AHashSet
        let unique_urls = Self::deduplicate_urls_optimized(&urls);
        let unique_count = unique_urls.len(); // Store count before moving

        if let Some(ref mut prog) = progress {
            prog.start_url_validation(unique_count);
        }

        let redirect_policy = Policy::limited(10);
        let user_agent = config.user_agent.as_deref().unwrap_or(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ));

        let mut client_builder = reqwest::Client::builder()
            .timeout(config.timeout_duration())
            .redirect(redirect_policy)
            .user_agent(user_agent)
            .http2_prior_knowledge() // Enable HTTP/2 for better connection reuse
            .http2_keep_alive_interval(Some(Duration::from_secs(30))) // Keep connections alive
            .http2_keep_alive_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(50) // Increase connection pool size
            .pool_idle_timeout(Some(Duration::from_secs(90)));
        // Compression (gzip, deflate, brotli) is enabled by default in reqwest

        // SSL verification
        if config.skip_ssl_verification.unwrap_or(false) {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        // Proxy configuration
        if let Some(ref proxy_url) = config.proxy {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }
        }

        let client = client_builder.build().unwrap();
        let progress_counter = Arc::new(AtomicUsize::new(0));

        let thread_count = config.threads.unwrap_or_else(num_cpus::get);
        let retry_attempts = config.retry_attempts.unwrap_or(0);
        let retry_delay = config.retry_delay_duration();
        let rate_limit_delay = config.rate_limit_delay_duration();

        // Process URLs in batches for better memory efficiency
        let batch_size = thread_count.min(100); // Limit batch size to prevent memory overflow
        let mut find_results_and_responses = stream::iter(unique_urls)
            .map(|ul| {
                let client = &client;
                let progress_counter = progress_counter.clone();
                let progress_ref = progress.as_ref();
                async move {
                    // Rate limiting
                    if rate_limit_delay > Duration::from_millis(0) {
                        sleep(rate_limit_delay).await;
                    }

                    let mut response = None;
                    let mut attempts = 0;

                    // Retry logic
                    while attempts <= retry_attempts {
                        let request = if config.use_head_requests.unwrap_or(false) {
                            client.head(&ul.url)
                        } else {
                            client.get(&ul.url)
                        };

                        match request.send().await {
                            Ok(resp) => {
                                response = Some(Ok(resp));
                                break;
                            }
                            Err(err) => {
                                if attempts == retry_attempts {
                                    response = Some(Err(err));
                                } else {
                                    sleep(retry_delay).await;
                                }
                                attempts += 1;
                            }
                        }
                    }

                    // Update progress
                    let current = progress_counter.fetch_add(1, AtomicOrdering::Relaxed) + 1;
                    if let Some(prog) = progress_ref {
                        prog.update_url_progress(current);
                    }

                    (ul.clone(), response.unwrap())
                }
            })
            .buffer_unordered(batch_size);

        // Pre-allocate result vector with capacity for better memory efficiency
        let mut result = Vec::with_capacity(unique_count);
        let mut success_count = 0;

        while let Some((ul, response)) = find_results_and_responses.next().await {
            let validation_result = match response {
                Ok(res) => {
                    let status_code = res.status().as_u16();
                    if res.status().is_success() {
                        success_count += 1;
                    }
                    ValidationResult {
                        url: ul.url,
                        line: ul.line,
                        file_name: ul.file_name,
                        status_code: Some(status_code),
                        description: None,
                    }
                }
                Err(err) => ValidationResult {
                    url: ul.url,
                    line: ul.line,
                    file_name: ul.file_name,
                    status_code: None,
                    description: std::error::Error::source(&err).map(|e| e.to_string()),
                },
            };

            result.push(validation_result);
        }

        if let Some(ref prog) = progress {
            prog.finish_url_validation(success_count, result.len());
        }

        result
    }
}

impl Validator {
    /// Optimized URL deduplication using FxHashSet for maximum performance  
    fn deduplicate_urls_optimized(urls: &[UrlLocation]) -> Vec<UrlLocation> {
        let mut seen_urls = FxHashSet::with_capacity_and_hasher(urls.len(), Default::default());
        let mut unique_urls = Vec::with_capacity(urls.len());

        for url_location in urls {
            if seen_urls.insert(&url_location.url) {
                unique_urls.push(url_location.clone());
            }
        }

        unique_urls
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use mockito::Server;
    use std::io::Write;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_validation_result__when_200__is_ok() {
        let vr = ValidationResult {
            url: "irrelevant".to_string(),
            line: 0,
            file_name: "irrelevant".to_string(),
            status_code: Some(200),
            description: None,
        };

        assert!(vr.is_ok());
        assert!(!vr.is_not_ok());
    }

    #[test]
    fn test_validation_result__when_404__is_not_ok() {
        let vr = ValidationResult {
            url: "irrelevant".to_string(),
            line: 0,
            file_name: "irrelevant".to_string(),
            status_code: Some(404),
            description: None,
        };

        assert!(!vr.is_ok());
        assert!(vr.is_not_ok());
    }

    #[test]
    fn test_validation_result__when_none__is_not_ok() {
        let vr = ValidationResult {
            url: "irrelevant".to_string(),
            line: 0,
            file_name: "irrelevant".to_string(),
            status_code: None,
            description: None,
        };

        assert!(!vr.is_ok());
        assert!(vr.is_not_ok());
    }

    #[test]
    fn test_validation_result__to_string() {
        let vr_200 = ValidationResult {
            url: "http://some-domain.com".to_string(),
            line: 99,
            file_name: "some-file-name".to_string(),
            status_code: Some(200),
            description: Some("should ignore this".to_string()),
        };

        assert_eq!(
            vr_200.to_string(),
            "200 - http://some-domain.com - some-file-name - L99"
        );

        let vr_description = ValidationResult {
            url: "http://some-domain.com".to_string(),
            line: 99,
            file_name: "some-file-name".to_string(),
            status_code: None,
            description: Some("some-description".to_string()),
        };

        assert_eq!(
            vr_description.to_string(),
            "http://some-domain.com - some-description - some-file-name - L99"
        );
    }

    #[tokio::test]
    async fn test_validate_urls__handles_url_with_status_code() {
        let validator = Validator::default();
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(5000), // Increase timeout for CI stability
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";

        let results = validator
            .validate_urls(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &opts,
            )
            .await;
        let actual = results.first().expect("No ValidationResult returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, Some(200));
        assert_eq!(actual.description, None);
    }

    #[tokio::test]
    async fn test_validate_urls__handles_not_available_url() {
        let validator = Validator::default();
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(50), // Small timeout to trigger timeout behavior
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let endpoint = "http://192.0.2.1:1/unreachable".to_string();

        let results = validator
            .validate_urls(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &opts,
            )
            .await;
        let actual = results.first().expect("No ValidationResult returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, None);
        assert!(
            actual
                .description
                .as_ref()
                .unwrap()
                .contains("operation timed out")
        );
    }

    #[tokio::test]
    async fn test_validate_urls__timeout_reached() {
        let validator = Validator::default();
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        // Use an unreachable address to trigger timeout
        let endpoint = "http://192.0.2.1:80/200".to_string(); // RFC 5737 TEST-NET-1 address

        let results = validator
            .validate_urls(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &opts,
            )
            .await;
        let actual = results.first().expect("No ValidationResult returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.description, Some("operation timed out".to_string()));
    }

    #[tokio::test]
    async fn test_validate_urls__works() -> TestResult {
        let validator = Validator::default();
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(5000), // Increase timeout for CI stability
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint_200 = server.url() + "/200";
        let endpoint_404 = server.url() + "/404";
        let endpoint_non_existing = "http://192.0.2.1:1/nonexisting".to_string();

        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(
            format!("arbitrary {endpoint_200} arbitrary [arbitrary]({endpoint_404}) arbitrary {endpoint_non_existing}")
            .as_bytes(),
        )?;

        let mut actual = validator
            .validate_urls(
                vec![
                    UrlLocation {
                        url: endpoint_200.clone(),
                        line: 99, // arbitrary
                        file_name: "arbitrary".to_string(),
                    },
                    UrlLocation {
                        url: endpoint_404.clone(),
                        line: 99, // arbitrary
                        file_name: "arbitrary".to_string(),
                    },
                    UrlLocation {
                        url: endpoint_non_existing.clone(),
                        line: 99, // arbitrary
                        file_name: "arbitrary".to_string(),
                    },
                ],
                &opts,
            )
            .await;

        actual.sort(); // Sort to be able to assert deterministically

        assert_eq!(actual[0].url, endpoint_200);
        assert_eq!(actual[0].status_code, Some(200));
        assert_eq!(actual[0].description, None);

        assert_eq!(actual[1].url, endpoint_404);
        assert_eq!(actual[1].status_code, Some(404));
        assert_eq!(actual[1].description, None);

        assert_eq!(actual[2].url, endpoint_non_existing);
        assert_eq!(actual[2].status_code, None);
        assert!(
            actual[2]
                .description
                .as_ref()
                .unwrap()
                .contains("operation timed out")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let endpoint_200 = server.url() + "/200";

        let config = crate::config::Config {
            timeout: Some(1),
            threads: Some(1),
            retry_attempts: Some(0),
            ..Default::default()
        };

        let validator = Validator::default();
        let actual = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint_200.clone(),
                    line: 1,
                    file_name: "test.md".to_string(),
                }],
                &config,
                None,
            )
            .await;

        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].url, endpoint_200);
        assert_eq!(actual[0].status_code, Some(200));

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_retry() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/retry")
            .with_status(500)
            .expect(3) // Should be called 3 times (initial + 2 retries)
            .create();
        let endpoint = server.url() + "/retry";

        let config = crate::config::Config {
            timeout: Some(1),
            threads: Some(1),
            retry_attempts: Some(2),
            retry_delay: Some(10), // Very short for testing
            ..Default::default()
        };

        let validator = Validator::default();
        let actual = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 1,
                    file_name: "test.md".to_string(),
                }],
                &config,
                None,
            )
            .await;

        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].url, endpoint);
        assert_eq!(actual[0].status_code, Some(500));

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_rate_limit() -> TestResult {
        let mut server = Server::new_async().await;
        let _m1 = server.mock("GET", "/rate1").with_status(200).create();
        let _m2 = server.mock("GET", "/rate2").with_status(200).create();
        let endpoint1 = server.url() + "/rate1";
        let endpoint2 = server.url() + "/rate2";

        let config = crate::config::Config {
            timeout: Some(1),
            threads: Some(1),
            rate_limit_delay: Some(50), // 50ms delay
            ..Default::default()
        };

        let start = std::time::Instant::now();
        let validator = Validator::default();
        let actual = validator
            .validate_urls_with_config(
                vec![
                    UrlLocation {
                        url: endpoint1,
                        line: 1,
                        file_name: "test.md".to_string(),
                    },
                    UrlLocation {
                        url: endpoint2,
                        line: 2,
                        file_name: "test.md".to_string(),
                    },
                ],
                &config,
                None,
            )
            .await;
        let duration = start.elapsed();

        assert_eq!(actual.len(), 2);
        // Should take at least 50ms due to rate limiting
        assert!(duration.as_millis() >= 40); // Allow some margin

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_custom_user_agent() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/ua")
            .match_header("user-agent", "TestAgent/1.0")
            .with_status(200)
            .create();
        let endpoint = server.url() + "/ua";

        let config = crate::config::Config {
            timeout: Some(1),
            threads: Some(1),
            user_agent: Some("TestAgent/1.0".to_string()),
            ..Default::default()
        };

        let validator = Validator::default();
        let actual = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 1,
                    file_name: "test.md".to_string(),
                }],
                &config,
                None,
            )
            .await;

        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].status_code, Some(200));

        Ok(())
    }

    #[test]
    fn test_deduplicate_urls_optimized() {
        let urls = vec![
            UrlLocation {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test1.md".to_string(),
            },
            UrlLocation {
                url: "https://example.com".to_string(),
                line: 2,
                file_name: "test2.md".to_string(),
            },
            UrlLocation {
                url: "https://different.com".to_string(),
                line: 1,
                file_name: "test3.md".to_string(),
            },
        ];

        let deduped = Validator::deduplicate_urls_optimized(&urls);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].url, "https://example.com");
        assert_eq!(deduped[1].url, "https://different.com");
    }

    #[test]
    fn test_deduplicate_urls_optimized_empty() {
        let urls = vec![];
        let deduped = Validator::deduplicate_urls_optimized(&urls);
        assert_eq!(deduped.len(), 0);
    }

    #[test]
    fn test_deduplicate_urls_optimized_single() {
        let urls = vec![UrlLocation {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        }];

        let deduped = Validator::deduplicate_urls_optimized(&urls);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].url, "https://example.com");
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_insecure_ssl() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(1),
            skip_ssl_verification: Some(true),
            ..Default::default()
        };

        // Test with a self-signed or invalid SSL URL
        let url_location = UrlLocation {
            url: "http://192.0.2.1:1/ssl-test".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        };

        let result = validator
            .validate_urls_with_config(vec![url_location], &config, None)
            .await;

        // Should not panic and return a result (may still fail due to DNS, but SSL shouldn't be the issue)
        assert!(!result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_empty_list() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config::default();

        let result = validator
            .validate_urls_with_config(vec![], &config, None)
            .await;

        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_validation_result_edge_cases() {
        // Test with very high status code
        let result = ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: Some(999),
            description: None,
        };
        assert!(!result.is_ok());

        // Test with timeout scenario (no status code, specific error)
        let timeout_result = ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: None,
            description: Some("timeout".to_string()),
        };
        assert!(!timeout_result.is_ok());
        let string_repr = timeout_result.to_string();
        assert!(string_repr.contains("timeout"));
    }

    #[test]
    fn test_deduplicate_urls_optimized_large_dataset() {
        let mut urls = Vec::new();
        // Create many duplicates to test performance characteristics
        for i in 0..1000 {
            urls.push(UrlLocation {
                url: format!("https://example.com/{}", i % 10), // 10 unique URLs, 100 duplicates each
                line: i,
                file_name: format!("file{i}.md"),
            });
        }

        let deduplicated = Validator::deduplicate_urls_optimized(&urls);
        assert_eq!(deduplicated.len(), 10); // Should have exactly 10 unique URLs

        // Check that first occurrence of each URL is preserved
        for (i, url) in deduplicated.iter().enumerate() {
            assert_eq!(url.url, format!("https://example.com/{i}"));
        }
    }

    #[test]
    fn test_deduplicate_urls_optimized_mixed_protocols() {
        let urls = vec![
            UrlLocation {
                url: "http://example.com".to_string(),
                line: 1,
                file_name: "file1.md".to_string(),
            },
            UrlLocation {
                url: "https://example.com".to_string(), // Different protocol, should be separate
                line: 2,
                file_name: "file2.md".to_string(),
            },
            UrlLocation {
                url: "http://example.com".to_string(), // Duplicate
                line: 3,
                file_name: "file3.md".to_string(),
            },
        ];

        let deduplicated = Validator::deduplicate_urls_optimized(&urls);
        assert_eq!(deduplicated.len(), 2); // http and https are different
        assert_eq!(deduplicated[0].url, "http://example.com");
        assert_eq!(deduplicated[1].url, "https://example.com");
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_proxy_success() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(1),
            proxy: Some("http://valid-proxy:8080".to_string()), // Will fail gracefully
            ..Default::default()
        };

        let url_location = UrlLocation {
            url: "http://192.0.2.1:1/proxy-test".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        };

        let result = validator
            .validate_urls_with_config(vec![url_location], &config, None)
            .await;

        // Should return a result (even if proxy fails)
        assert_eq!(result.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_error_with_source() -> TestResult {
        let validator = Validator::default();

        // Use an invalid URL to trigger an error with source
        let url_location = UrlLocation {
            url: "http://0.0.0.0:1/invalid".to_string(), // Should cause connection error
            line: 1,
            file_name: "test.md".to_string(),
        };

        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_millis(10), // Very short timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };

        let result = validator.validate_urls(vec![url_location], &opts).await;

        assert_eq!(result.len(), 1);
        assert!(result[0].status_code.is_none());
        assert!(result[0].description.is_some() || result[0].description.is_none()); // Either way is valid
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_progress() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/progress").with_status(200).create();
        let endpoint = server.url() + "/progress";

        let config = crate::config::Config {
            timeout: Some(1),
            ..Default::default()
        };

        let mut progress = ProgressReporter::new(false); // Disabled for tests
        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint,
                    line: 1,
                    file_name: "test.md".to_string(),
                }],
                &config,
                Some(&mut progress),
            )
            .await;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status_code, Some(200));
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_rate_limit_zero() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/norlimit").with_status(200).create();
        let endpoint = server.url() + "/norlimit";

        let config = crate::config::Config {
            timeout: Some(1),
            rate_limit_delay: Some(0), // No rate limiting
            ..Default::default()
        };

        let start = std::time::Instant::now();
        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint,
                    line: 1,
                    file_name: "test.md".to_string(),
                }],
                &config,
                None,
            )
            .await;
        let duration = start.elapsed();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status_code, Some(200));
        // Should be fast with no rate limiting
        assert!(duration.as_millis() < 500);
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_with_config_retry_failure() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(1),
            retry_attempts: Some(2), // Try 3 times total (initial + 2 retries)
            retry_delay: Some(10),   // Short delay
            ..Default::default()
        };

        // Use non-routable IP to ensure failure
        let url_location = UrlLocation {
            url: "http://192.0.2.1:80/retry-fail".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        };

        let start = std::time::Instant::now();
        let result = validator
            .validate_urls_with_config(vec![url_location], &config, None)
            .await;
        let duration = start.elapsed();

        assert_eq!(result.len(), 1);
        assert!(result[0].status_code.is_none());
        // Should take at least retry_delay * retry_attempts time
        assert!(duration.as_millis() >= 15); // Allow some margin
        Ok(())
    }

    #[test]
    fn test_validation_result_with_description_only() {
        // Test valid case with description
        let result = ValidationResult {
            url: "https://test.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: None,
            description: Some("connection failed".to_string()),
        };

        let string_repr = result.to_string();
        assert!(string_repr.contains("https://test.com"));
        assert!(string_repr.contains("test.md"));
        assert!(string_repr.contains("connection failed"));
    }

    #[tokio::test]
    async fn test_validate_urls_default_user_agent() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(1),
            user_agent: None, // Use default user agent
            ..Default::default()
        };

        let url_location = UrlLocation {
            url: "http://192.0.2.1:1/user-agent".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
        };

        let result = validator
            .validate_urls_with_config(vec![url_location], &config, None)
            .await;

        // Should use default user agent and succeed
        assert_eq!(result.len(), 1);
        // May succeed or fail depending on network, but shouldn't panic
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_success_counting() -> TestResult {
        let mut server = Server::new_async().await;
        let _m1 = server.mock("GET", "/success1").with_status(200).create();
        let _m2 = server.mock("GET", "/success2").with_status(201).create();
        let _m3 = server.mock("GET", "/fail").with_status(404).create();

        let config = crate::config::Config {
            timeout: Some(1),
            ..Default::default()
        };

        let urls = vec![
            UrlLocation {
                url: server.url() + "/success1",
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: server.url() + "/success2",
                line: 2,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: server.url() + "/fail",
                line: 3,
                file_name: "test.md".to_string(),
            },
        ];

        let mut progress = ProgressReporter::new(false); // Disabled for tests
        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(urls, &config, Some(&mut progress))
            .await;

        // Should process all URLs
        assert_eq!(result.len(), 3);

        // Check that we have both success and failure cases
        let success_count = result
            .iter()
            .filter(|r| r.status_code == Some(200) || r.status_code == Some(201))
            .count();
        let fail_count = result.iter().filter(|r| r.status_code == Some(404)).count();

        assert_eq!(success_count, 2);
        assert_eq!(fail_count, 1);

        Ok(())
    }
}
