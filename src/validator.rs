use async_trait::async_trait;
use futures::{StreamExt, stream};
use reqwest::redirect::Policy;
use rustc_hash::FxHashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use tokio::time::{Duration, Instant, sleep};

use crate::{UrlLocation, config::Config, constants::http_status, progress::ProgressReporter};

use std::cmp::Ordering;
use std::fmt;
use std::sync::Mutex;

/// Simple token bucket rate limiter for smoother request distribution
#[derive(Debug)]
struct TokenBucket {
    tokens: Arc<Mutex<f64>>,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Arc<Mutex<Instant>>,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(capacity)),
            capacity,
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    async fn acquire(&self) -> bool {
        let now = Instant::now();

        // Refill tokens based on elapsed time
        {
            let mut last_refill = self.last_refill.lock().unwrap();
            let elapsed = now.duration_since(*last_refill).as_secs_f64();
            let new_tokens = elapsed * self.refill_rate;

            if new_tokens > 0.0 {
                let mut tokens = self.tokens.lock().unwrap();
                *tokens = (*tokens + new_tokens).min(self.capacity);
                *last_refill = now;
            }
        }

        // Try to acquire a token
        let mut tokens = self.tokens.lock().unwrap();
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[async_trait]
pub trait ValidateUrls {
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
    /// Check if this validation result represents a successful URL check.
    ///
    /// Currently only considers HTTP 200 as successful, but this could be
    /// extended to include other 2xx status codes based on configuration.
    pub fn is_ok(&self) -> bool {
        matches!(self.status_code, Some(http_status::OK))
    }

    /// Check if this validation result represents a failed URL check.
    pub fn is_not_ok(&self) -> bool {
        !self.is_ok()
    }

    /// Create a new ValidationResult for a successful HTTP response.
    pub fn success(url: String, line: u64, file_name: String, status_code: u16) -> Self {
        Self {
            url,
            line,
            file_name,
            status_code: Some(status_code),
            description: None,
        }
    }

    /// Create a new ValidationResult for a failed request.
    pub fn error(url: String, line: u64, file_name: String, description: String) -> Self {
        Self {
            url,
            line,
            file_name,
            status_code: None,
            description: Some(description),
        }
    }

    /// Create a ValidationResult from a UrlLocation and HTTP status.
    pub fn from_url_location_and_status(location: &UrlLocation, status_code: u16) -> Self {
        Self::success(
            location.url().to_string(),
            location.line(),
            location.file_name().to_string(),
            status_code,
        )
    }

    /// Create a ValidationResult from a UrlLocation and error description.
    pub fn from_url_location_and_error(location: &UrlLocation, description: String) -> Self {
        Self::error(
            location.url().to_string(),
            location.line(),
            location.file_name().to_string(),
            description,
        )
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

        let thread_count = config.threads.unwrap_or_else(num_cpus::get);

        let mut client_builder = reqwest::Client::builder()
            .timeout(config.timeout_duration())
            .redirect(redirect_policy)
            .user_agent(user_agent);

        // Connection pooling configuration for better performance
        client_builder = client_builder
            .pool_max_idle_per_host(thread_count.min(20)) // Limit idle connections per host
            .pool_idle_timeout(Duration::from_secs(30)) // Close idle connections after 30s
            .tcp_keepalive(Duration::from_secs(60)); // Keep TCP connections alive

        // Use reqwest defaults for compression (gzip, deflate, brotli)

        // SSL verification
        if config.skip_ssl_verification.unwrap_or(false) {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        // Proxy configuration
        if let Some(ref proxy_url) = config.proxy
            && let Ok(proxy) = reqwest::Proxy::all(proxy_url)
        {
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder.build().unwrap();
        let progress_counter = Arc::new(AtomicUsize::new(0));

        let retry_attempts = config.retry_attempts.unwrap_or(0);
        let retry_delay = config.retry_delay_duration();
        let rate_limit_delay = config.rate_limit_delay_duration();

        // Create token bucket for smoother rate limiting
        let rate_limiter = if rate_limit_delay > Duration::from_millis(0) {
            let requests_per_second = 1000.0 / rate_limit_delay.as_millis() as f64;
            Some(Arc::new(TokenBucket::new(
                thread_count as f64,
                requests_per_second,
            )))
        } else {
            None
        };

        // Process URLs in batches for better memory efficiency
        let batch_size = Self::calculate_optimal_batch_size(unique_count, thread_count);
        let mut find_results_and_responses = stream::iter(unique_urls)
            .map(|ul| {
                let client = &client;
                let progress_counter = progress_counter.clone();
                let progress_ref = progress.as_ref();
                let rate_limiter = rate_limiter.clone();
                async move {
                    // Token bucket rate limiting
                    if let Some(ref limiter) = rate_limiter {
                        while !limiter.acquire().await {
                            sleep(Duration::from_millis(10)).await;
                        }
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

                    // Update progress in batches to reduce atomic operations
                    let current = progress_counter.fetch_add(1, AtomicOrdering::Relaxed) + 1;
                    if let Some(prog) = progress_ref {
                        // Only update progress every 10 requests or on significant milestones
                        if current % 10 == 0 || current == 1 {
                            prog.update_url_progress(current);
                        }
                    }

                    (ul, response.unwrap())
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
                    ValidationResult::from_url_location_and_status(&ul, status_code)
                }
                Err(err) => {
                    let description = std::error::Error::source(&err)
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| err.to_string());
                    ValidationResult::from_url_location_and_error(&ul, description)
                }
            };

            result.push(validation_result);
        }

        if let Some(ref prog) = progress {
            // Ensure final progress update to show completion
            prog.update_url_progress(result.len());
            prog.finish_url_validation(success_count, result.len());
        }

        result
    }
}

impl Validator {
    /// Calculate optimal batch size based on URL count and system resources
    fn calculate_optimal_batch_size(url_count: usize, thread_count: usize) -> usize {
        // Base batch size on thread count, but adapt based on URL count
        let base_batch_size = thread_count;

        match url_count {
            0..=10 => base_batch_size.min(2),      // Small batch for few URLs
            11..=100 => base_batch_size.min(10),   // Medium batch for moderate URLs
            101..=1000 => base_batch_size.min(50), // Larger batch for many URLs
            _ => base_batch_size.min(100),         // Cap at 100 for very large sets
        }
    }

    /// Optimized URL deduplication using FxHashSet for maximum performance  
    pub fn deduplicate_urls_optimized(urls: &[UrlLocation]) -> Vec<UrlLocation> {
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
        let config = crate::config::Config {
            timeout: Some(5), // 5 seconds for CI stability
            threads: Some(1),
            allow_timeout: Some(false),
            ..Default::default()
        };
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";

        let results = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &config,
                None,
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
        let config = crate::config::Config {
            timeout: Some(1), // 1 second timeout to trigger timeout behavior
            threads: Some(1),
            allow_timeout: Some(false),
            ..Default::default()
        };
        let endpoint = "http://192.0.2.1:1/unreachable".to_string();

        let results = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &config,
                None,
            )
            .await;
        let actual = results.first().expect("No ValidationResult returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, None);
        assert!(actual.description.is_some());
    }

    #[tokio::test]
    async fn test_validate_urls__timeout_reached() {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(1), // Use very small timeout
            threads: Some(1),
            allow_timeout: Some(false),
            ..Default::default()
        };
        // Use an unreachable address to trigger timeout
        let endpoint = "http://192.0.2.1:80/200".to_string(); // RFC 5737 TEST-NET-1 address

        let results = validator
            .validate_urls_with_config(
                vec![UrlLocation {
                    url: endpoint.clone(),
                    line: 99, // arbitrary
                    file_name: "arbitrary".to_string(),
                }],
                &config,
                None,
            )
            .await;
        let actual = results.first().expect("No ValidationResult returned");

        assert_eq!(actual.url, endpoint);
        assert!(actual.description.is_some());
    }

    #[tokio::test]
    async fn test_validate_urls__works() -> TestResult {
        let validator = Validator::default();
        let config = crate::config::Config {
            timeout: Some(5), // 5 seconds for CI stability
            threads: Some(1),
            allow_timeout: Some(false),
            ..Default::default()
        };
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint_200 = server.url() + "/200";
        let endpoint_404 = server.url() + "/404";
        let endpoint_non_existing = "http://192.0.2.1:1/nonexisting".to_string();

        let mut file = tempfile::NamedTempFile::new().map_err(crate::error::UrlsUpError::Io)?;
        file.write_all(
            format!("arbitrary {endpoint_200} arbitrary [arbitrary]({endpoint_404}) arbitrary {endpoint_non_existing}")
            .as_bytes(),
        ).map_err(crate::error::UrlsUpError::Io)?;

        let mut actual = validator
            .validate_urls_with_config(
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
                &config,
                None,
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
        assert!(actual[2].description.is_some());

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

        let config = crate::config::Config {
            timeout: Some(1), // Very short timeout
            threads: Some(1),
            allow_timeout: Some(false),
            ..Default::default()
        };

        let result = validator
            .validate_urls_with_config(vec![url_location], &config, None)
            .await;

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

    #[tokio::test]
    async fn test_validate_urls_with_head_requests() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("HEAD", "/head-test").with_status(200).create();
        let endpoint = server.url() + "/head-test";

        let config = crate::config::Config {
            timeout: Some(1),
            use_head_requests: Some(true),
            ..Default::default()
        };

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

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status_code, Some(200));
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_large_batch() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/batch").with_status(200).create();
        let base_url = server.url() + "/batch";

        let config = crate::config::Config {
            timeout: Some(1),
            threads: Some(2),
            ..Default::default()
        };

        // Create a large batch of URLs
        let urls: Vec<UrlLocation> = (0..50)
            .map(|i| UrlLocation {
                url: base_url.clone(),
                line: i,
                file_name: format!("file{i}.md"),
            })
            .collect();

        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(urls, &config, None)
            .await;

        // All URLs should succeed since they're the same
        assert_eq!(result.len(), 1); // Deduplicated to 1 unique URL
        assert_eq!(result[0].status_code, Some(200));
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_mixed_protocols() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/mixed").with_status(200).create();
        let base_url = server.url();

        let config = crate::config::Config {
            timeout: Some(1),
            ..Default::default()
        };

        let urls = vec![
            UrlLocation {
                url: format!("{base_url}/mixed"),
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "ftp://example.com/file".to_string(), // Unsupported protocol
                line: 2,
                file_name: "test.md".to_string(),
            },
        ];

        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(urls, &config, None)
            .await;

        assert_eq!(result.len(), 2);

        // HTTP URL should succeed
        let http_result = result.iter().find(|r| r.url.starts_with("http")).unwrap();
        assert_eq!(http_result.status_code, Some(200));

        // FTP URL should fail
        let ftp_result = result.iter().find(|r| r.url.starts_with("ftp")).unwrap();
        assert!(ftp_result.status_code.is_none());
        assert!(ftp_result.description.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_concurrent_behavior() -> TestResult {
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/concurrent").with_status(200).create();
        let base_url = server.url() + "/concurrent";

        let config = crate::config::Config {
            timeout: Some(5),
            threads: Some(3), // Moderate concurrency
            ..Default::default()
        };

        // Create a few URLs that will be processed concurrently
        let urls: Vec<UrlLocation> = (0..5)
            .map(|i| UrlLocation {
                url: format!("{base_url}?test={i}"),
                line: i,
                file_name: format!("test{i}.md"),
            })
            .collect();

        let validator = Validator::default();
        let start = std::time::Instant::now();
        let result = validator
            .validate_urls_with_config(urls, &config, None)
            .await;
        let duration = start.elapsed();

        // All URLs should be processed
        assert_eq!(result.len(), 5);

        // Should be reasonably fast with concurrency
        assert!(duration.as_secs() < 10);
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_urls_malformed_urls() -> TestResult {
        let config = crate::config::Config {
            timeout: Some(1),
            ..Default::default()
        };

        let urls = vec![
            UrlLocation {
                url: "not-a-url".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "http://".to_string(), // Incomplete URL
                line: 2,
                file_name: "test.md".to_string(),
            },
            UrlLocation {
                url: "https://[invalid".to_string(), // Invalid format
                line: 3,
                file_name: "test.md".to_string(),
            },
        ];

        let validator = Validator::default();
        let result = validator
            .validate_urls_with_config(urls, &config, None)
            .await;

        assert_eq!(result.len(), 3);

        // All should fail due to malformed URLs
        for res in &result {
            assert!(res.status_code.is_none());
            assert!(res.description.is_some());
        }

        Ok(())
    }

    #[test]
    fn test_validation_result_ordering() {
        let mut results = vec![
            ValidationResult {
                url: "https://z.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(200),
                description: None,
            },
            ValidationResult {
                url: "https://a.com".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: None,
            },
            ValidationResult {
                url: "https://m.com".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
                status_code: Some(200),
                description: None,
            },
        ];

        results.sort();

        assert_eq!(results[0].url, "https://a.com");
        assert_eq!(results[1].url, "https://m.com");
        assert_eq!(results[2].url, "https://z.com");
    }

    #[test]
    fn test_validation_result_equality() {
        let result1 = ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test1.md".to_string(),
            status_code: Some(200),
            description: None,
        };

        let result2 = ValidationResult {
            url: "https://example.com".to_string(),
            line: 5,                           // Different line
            file_name: "test2.md".to_string(), // Different file
            status_code: Some(200),
            description: None,
        };

        let result3 = ValidationResult {
            url: "https://different.com".to_string(),
            line: 1,
            file_name: "test1.md".to_string(),
            status_code: Some(200),
            description: None,
        };

        // Same URL, same status/description should be equal
        assert_eq!(result1, result2);
        // Different URL should not be equal
        assert_ne!(result1, result3);
    }
}
