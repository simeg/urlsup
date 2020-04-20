use futures::{stream, StreamExt};
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use linkify::{LinkFinder, LinkKind};
use reqwest::redirect::Policy;
use spinners::{Spinner, Spinners};

use std::cmp::Ordering;
use std::io::Error;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Eq, Clone)]
pub struct AuditResult {
    url: String,
    status_code: Option<u16>,
    description: Option<String>,
}

impl Ord for AuditResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

impl PartialOrd for AuditResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AuditResult {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.status_code == other.status_code
            && self.description == other.description
    }
}

impl AuditResult {
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

    pub fn to_string(&self) -> String {
        if let Some(num) = &self.status_code {
            format!("{} {}", num.to_string(), &self.url)
        } else if let Some(desc) = &self.description {
            format!("{} {}", &self.url, desc)
        } else {
            unreachable!("AuditResult should always have status_code or description")
        }
    }
}

const MARKDOWN_URL_PATTERN: &str =
    r#"(http://|https://)[a-z0-9]+([-.]{1}[a-z0-9]+)*(.[a-z]{2,5})?(:[0-9]{1,5})?(/.*)?"#;

const DEFAULT_TIMEOUT: u64 = 30;

pub struct Auditor {}

pub struct AuditorOptions {
    pub white_list: Option<Vec<String>>,
    pub timeout: Option<u64>,
    pub allowed_status_codes: Option<Vec<u16>>,
}

impl Auditor {
    pub async fn check(&self, paths: Vec<&Path>, opts: AuditorOptions) {
        let spinner_find_urls = self.spinner_start(format!("Finding URLs in files..."));

        // Find URLs from files
        let mut urls = self.find_urls(paths);

        // Ignore white listed URLs
        if let Some(white_list) = &opts.white_list {
            println!("\n\nIgnoring white listed URLs");
            for (i, url) in white_list.iter().enumerate() {
                println!("{:4}. {}", i + 1, url.to_string());
            }

            urls = self.apply_white_list(urls, white_list);
        }

        if let Some(allowed) = &opts.allowed_status_codes {
            println!("\n\nAllowing status codes");
            for (i, status_code) in allowed.iter().enumerate() {
                println!("{:4}. {}", i + 1, status_code.to_string());
            }
        }

        // Save URL count to avoid having to clone URL list
        let url_count = urls.len();

        // Deduplicate URLs to avoid duplicate work
        let dedup_urls = self.dedup(urls);

        spinner_find_urls.stop();

        let urls_singular_plural = if &dedup_urls.len() == &(1 as usize) {
            "URL"
        } else {
            "URLs"
        };

        println!(
            "\nFound {} unique {}, {} in total",
            &dedup_urls.len(),
            urls_singular_plural,
            url_count
        );

        for (i, url) in dedup_urls.iter().enumerate() {
            println!("{:4}. {}", i + 1, url.to_string());
        }

        println!(); // Make output more readable

        let validation_spinner = self.spinner_start("Checking URLs...".into());

        // Audit urls
        let mut non_ok_urls: Vec<AuditResult> = self
            .audit_urls(dedup_urls, &opts)
            .await
            .into_iter()
            .filter(|audit_result| audit_result.is_not_ok())
            .collect();

        if let Some(allowed) = &opts.allowed_status_codes {
            non_ok_urls = self.filter_allowed_status_codes(non_ok_urls, allowed.clone());
        }

        validation_spinner.stop();

        if non_ok_urls.is_empty() {
            println!("\n\nNo issues!");
            std::process::exit(0)
        }

        println!("\n\n> Issues");
        for (i, audit_result) in non_ok_urls.iter().enumerate() {
            println!("{:4}. {}", i + 1, audit_result.to_string());
        }
        std::process::exit(1)
    }

    fn find_urls(&self, paths: Vec<&Path>) -> Vec<String> {
        paths
            .into_iter()
            .flat_map(|path| {
                self.find_lines_with_url(path).unwrap_or_else(|_| {
                    panic!(
                        "Something went wrong parsing URL in file: {}",
                        path.display()
                    )
                })
            })
            .flat_map(|line| self.parse_urls(line))
            .collect()
    }

    fn find_lines_with_url(&self, path: &Path) -> Result<Vec<String>, Error> {
        let matcher = RegexMatcher::new(MARKDOWN_URL_PATTERN).unwrap();

        let mut matches = vec![];
        Searcher::new().search_path(
            &matcher,
            &path,
            UTF8(|_lnum, line| {
                matches.push(line.trim().to_string());
                Ok(true)
            }),
        )?;

        Ok(matches)
    }

    fn parse_urls(&self, line: String) -> Vec<String> {
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);

        finder
            .links(line.as_str())
            .map(|url| url.as_str().to_string())
            .collect()
    }

    async fn audit_urls(&self, urls: Vec<String>, opts: &AuditorOptions) -> Vec<AuditResult> {
        let timeout = Duration::from_secs(opts.timeout.unwrap_or(DEFAULT_TIMEOUT));
        let redirect_policy = Policy::limited(10);
        let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(redirect_policy)
            .user_agent(user_agent)
            .build()
            .unwrap();

        let mut urls_and_responses = stream::iter(urls)
            .map(|url| {
                let client = &client;
                async move {
                    let response = client.get(&url).send().await;
                    (url.clone(), response)
                }
            })
            .buffer_unordered(num_cpus::get());

        let mut result = vec![];
        while let Some((url, response)) = urls_and_responses.next().await {
            let audit_result = match response {
                Ok(res) => AuditResult {
                    url,
                    status_code: Some(res.status().as_u16()),
                    description: None,
                },
                Err(err) => AuditResult {
                    url,
                    status_code: None,
                    description: std::error::Error::source(&err)
                        .map_or(None, |e| Some(e.to_string())),
                },
            };

            result.push(audit_result);
        }

        result
    }

    fn apply_white_list(&self, urls: Vec<String>, white_list: &Vec<String>) -> Vec<String> {
        urls.into_iter()
            .filter(|url| {
                // If white list URL matches URL
                if white_list.contains(url) {
                    return false;
                }

                // If URL begins with white list URL
                for white_listed_url in white_list.iter() {
                    if url.starts_with(white_listed_url) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn filter_allowed_status_codes(
        &self,
        audit_results: Vec<AuditResult>,
        allowed_status_codes: Vec<u16>,
    ) -> Vec<AuditResult> {
        audit_results
            .into_iter()
            .filter(|ar| {
                if let Some(status_code) = ar.status_code {
                    if allowed_status_codes.contains(&status_code) {
                        return false;
                    }
                }
                return true;
            })
            .collect()
    }

    fn dedup(&self, mut list: Vec<String>) -> Vec<String> {
        list.sort();
        list.dedup();
        list
    }

    fn spinner_start(&self, msg: String) -> Spinner {
        Spinner::new(Spinners::Dots, msg)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use std::io::Write;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_parse_urls() {
        let auditor = Auditor {};
        let md_link =
            "arbitrary [something](http://foo.bar) arbitrary http://foo2.bar arbitrary".to_string();
        let expected = vec!["http://foo.bar".to_string(), "http://foo2.bar".to_string()];
        let actual = auditor.parse_urls(md_link);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_img_url() {
        let auditor = Auditor {};
        let md_link = "arbitrary ![image](http://foo.bar) arbitrary".to_string();
        let expected = vec!["http://foo.bar".to_string()];
        let actual = auditor.parse_urls(md_link);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_badge_url() {
        let auditor = Auditor {};
        let md_link = "arbitrary [something]: http://foo.bar arbitrary".to_string();
        let expected = vec!["http://foo.bar".to_string()];
        let actual = auditor.parse_urls(md_link);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_lines_with_url__from_file() -> TestResult {
        let auditor = Auditor {};
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(
            "arbitrary [something](http://specific-link.one) arbitrary\n\
             arbitrary [something](http://specific-link.two) arbitrary\n\
             arbitrary [badge-something]: http://specific-link.three arbitrary\n\
             arbitrary http://specific-link.four arbitrary"
                .as_bytes(),
        )?;

        let actual = auditor.find_lines_with_url(file.path()).unwrap();

        let actual_line1 = &actual.get(0).unwrap().as_str().to_owned();
        let actual_line2 = &actual.get(1).unwrap().as_str().to_owned();
        let actual_line3 = &actual.get(2).unwrap().as_str().to_owned();
        let actual_line4 = &actual.get(3).unwrap().as_str().to_owned();

        assert_eq!(
            actual_line1,
            "arbitrary [something](http://specific-link.one) arbitrary"
        );
        assert_eq!(
            actual_line2,
            "arbitrary [something](http://specific-link.two) arbitrary"
        );
        assert_eq!(
            actual_line3,
            "arbitrary [badge-something]: http://specific-link.three arbitrary"
        );
        assert_eq!(
            actual_line4,
            "arbitrary http://specific-link.four arbitrary"
        );

        Ok(())
    }

    #[test]
    fn test_find_lines_with_urL__from_file__when_non_existing_file() -> TestResult {
        let auditor = Auditor {};
        let non_existing_file = "non_existing_file.txt";
        let is_err = auditor
            .find_lines_with_url(non_existing_file.as_ref())
            .is_err();

        assert!(is_err);

        Ok(())
    }

    #[test]
    fn test_dedup() {
        let auditor = Auditor {};
        let duplicate: Vec<String> = vec!["duplicate", "duplicate", "unique-1", "unique-2"]
            .into_iter()
            .map(String::from)
            .collect();

        let actual = auditor.dedup(duplicate);
        let expected: Vec<String> = vec!["duplicate", "unique-1", "unique-2"]
            .into_iter()
            .map(String::from)
            .collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_apply_white_list__filters_out_white_listed_urls() {
        let auditor = Auditor {};
        let urls: Vec<String> = vec![
            "http://should-keep.com",
            "http://should-ignore.com",
            "http://should-also-ignore.com/something/something-else",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let white_list: Vec<String> =
            vec!["http://should-ignore.com", "http://should-also-ignore.com"]
                .into_iter()
                .map(String::from)
                .collect();

        let actual = auditor.apply_white_list(urls, &white_list);
        let expected: Vec<String> = vec!["http://should-keep.com"]
            .into_iter()
            .map(String::from)
            .collect();

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_filter_allowed_status_codes__removes_allowed_status_codes() {
        let auditor = Auditor {};
        let ar1 = AuditResult {
            url: "keep-this".to_string(),
            status_code: Some(200),
            description: None,
        };
        let ar2 = AuditResult {
            url: "keep-this-2".to_string(),
            status_code: None,
            description: Some("arbirary".to_string()),
        };
        let ar3 = AuditResult {
            url: "remove-this".to_string(),
            status_code: Some(404),
            description: None,
        };
        let actual = auditor
            .filter_allowed_status_codes(vec![ar1.clone(), ar2.clone(), ar3.clone()], vec![404]);
        let expected = vec![ar1, ar2];

        assert_eq!(actual, expected)
    }
}

#[cfg(test)]
mod integration_tests {
    #![allow(non_snake_case)]

    use super::*;
    use mockito::mock;
    use std::io::Write;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn test_audit_urls__handles_url_with_status_code() {
        let auditor = Auditor {};
        let opts = AuditorOptions {
            white_list: None,
            timeout: None,
            allowed_status_codes: None,
        };
        let _m = mock("GET", "/200").with_status(200).create();
        let endpoint = mockito::server_url() + "/200";

        let audit_results = auditor.audit_urls(vec![endpoint.clone()], &opts).await;

        let actual = audit_results.first().expect("No AuditResults returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, Some(200));
        assert_eq!(actual.description, None);
    }

    #[tokio::test]
    async fn test_audit_urls__handles_not_available_url() {
        let auditor = Auditor {};
        let opts = AuditorOptions {
            white_list: None,
            timeout: None,
            allowed_status_codes: None,
        };
        let endpoint = "https://localhost.auditor".to_string();

        let audit_results = auditor.audit_urls(vec![endpoint.clone()], &opts).await;

        let actual = audit_results.first().expect("No AuditResults returned");

        assert_eq!(actual.url, endpoint);
        assert_eq!(actual.status_code, None);
        assert!(actual
            .description
            .as_ref()
            .unwrap()
            .contains("error trying to connect: dns error: failed to lookup address information:"));
    }

    #[tokio::test]
    async fn test_check__works() -> TestResult {
        let auditor = Auditor {};
        let opts = AuditorOptions {
            white_list: None,
            timeout: None,
            allowed_status_codes: None,
        };
        let _m200 = mock("GET", "/200").with_status(200).create();
        let _m404 = mock("GET", "/404").with_status(404).create();
        let endpoint_200 = mockito::server_url() + "/200";
        let endpoint_404 = mockito::server_url() + "/404";
        let endpoint_non_existing = "https://localhost.auditor".to_string();

        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(
            format!(
                "arbitrary {} arbitrary [arbitrary]({}) arbitrary {}",
                endpoint_200, endpoint_404, endpoint_non_existing
            )
            .as_bytes(),
        )?;

        let mut actual: Vec<AuditResult> = auditor
            .audit_urls(
                vec![
                    endpoint_200.clone(),
                    endpoint_404.clone(),
                    endpoint_non_existing.clone(),
                ],
                &opts,
            )
            .await;

        actual.sort();

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
