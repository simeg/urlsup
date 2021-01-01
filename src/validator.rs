use async_trait::async_trait;
use futures::{stream, StreamExt};
use reqwest::redirect::Policy;

use crate::{UrlLocation, UrlsUpOptions};

use std::cmp::Ordering;
use std::fmt;

#[async_trait]
pub trait ValidateUrls {
    async fn validate_urls(
        &self,
        urls: Vec<UrlLocation>,
        opts: &UrlsUpOptions,
    ) -> Vec<ValidationResult>;
}

pub struct Validator {}

impl Default for Validator {
    fn default() -> Self {
        Self {}
    }
}

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
                num.to_string(),
                &self.url,
                &self.file_name,
                &self.line
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
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use mockito::mock;
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
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";

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
        assert!(actual
            .description
            .as_ref()
            .unwrap()
            .contains("error trying to connect: dns error: failed to lookup address information:"));
    }

    #[tokio::test]
    async fn test_validate_urls__timeout_reached() {
        let validator = Validator::default();
        let opts = UrlsUpOptions {
            white_list: None,
            timeout: Duration::from_nanos(1), // Use very small timeout
            allowed_status_codes: None,
            thread_count: 1,
            allow_timeout: false,
        };
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";

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
        let _m200 = mock("GET", "/200").with_status(200).create();
        let _m404 = mock("GET", "/404").with_status(404).create();
        let endpoint_200 = mockito::server_url() + "/200";
        let endpoint_404 = mockito::server_url() + "/404";
        let endpoint_non_existing = "https://localhost.urls_up".to_string();

        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(
            format!(
                "arbitrary {} arbitrary [arbitrary]({}) arbitrary {}",
                endpoint_200, endpoint_404, endpoint_non_existing
            )
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
        assert!(actual[2]
            .description
            .as_ref()
            .unwrap()
            .contains("error trying to connect: dns error: failed to lookup address information:"));

        Ok(())
    }
}
