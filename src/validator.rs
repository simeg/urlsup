use ahash::AHashSet;
use async_trait::async_trait;
use futures::{StreamExt, stream};
use reqwest::redirect::Policy;
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

#[derive(Default)]
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

        if let Some(ref mut prog) = progress {
            prog.start_url_validation(unique_urls.len());
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
            .user_agent(user_agent);

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
                        match client.get(&ul.url).send().await {
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
            .buffer_unordered(thread_count);

        let mut result = vec![];
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
    /// Optimized URL deduplication using AHashSet for better performance
    fn deduplicate_urls_optimized(urls: &[UrlLocation]) -> Vec<UrlLocation> {
        let mut seen_urls = AHashSet::with_capacity(urls.len());
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
            timeout: Duration::from_secs(10),
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
            timeout: Duration::from_secs(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let endpoint = "https://localhost.urls_up".to_string();

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
                .contains("client error (Connect)")
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
            timeout: Duration::from_secs(10),
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint_200 = server.url() + "/200";
        let endpoint_404 = server.url() + "/404";
        let endpoint_non_existing = "https://localhost.urls_up".to_string();

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
                .contains("client error (Connect)")
        );

        Ok(())
    }
}
