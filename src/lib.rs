use futures::{stream, StreamExt};
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use linkify::{LinkFinder, LinkKind};
use reqwest::redirect::Policy;

use std::io::Error;
use std::path::Path;
use std::time::Duration;

pub struct HttpStatusCode {
    num: u16,
    is_unknown: bool,
}

impl HttpStatusCode {
    pub fn is_ok(&self) -> bool {
        self.num == 200
    }

    pub fn is_not_ok(&self) -> bool {
        !self.is_ok()
    }

    pub fn as_u16(&self) -> u16 {
        self.num
    }

    pub fn is_unknown(&self) -> bool {
        self.is_unknown
    }
}

const MARKDOWN_URL_PATTERN: &str =
    r#"(http://|https://)[a-z0-9]+([-.]{1}[a-z0-9]+)*.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?"#;

const THREAD_COUNT: usize = 50;

pub struct Auditor {}
pub struct AuditorOptions {}

impl Auditor {
    pub async fn check(&self, paths: Vec<&Path>, _opts: AuditorOptions) {
        println!("> Checking for URLs in {:?}", &paths);

        // Find urls from files
        let urls = self.find_urls(paths);

        // Save url count to avoid having to clone url list
        let url_count = urls.len();

        // Deduplicate urls to avoid duplicate work
        let dedup_urls = self.dedup(urls);

        println!(
            "Found {} unique URLs, {} in total",
            &dedup_urls.len(),
            url_count
        );

        let mut count = 1;
        for url in &dedup_urls {
            println!("{:4}. {}", count, url.to_string());
            count += 1;
        }

        println!("Checking URLs...");

        // Query them to see if they are up
        let validation_results = self.validate_urls(dedup_urls).await;

        let non_ok_urls: Vec<(String, HttpStatusCode)> = validation_results
            .into_iter()
            .filter(|(_url, status)| status.is_not_ok())
            .collect();

        if non_ok_urls.is_empty() {
            println!("No issues!");
            std::process::exit(0)
        }

        println!("\n> Issues");
        let mut count = 1;
        for (url, status_code) in non_ok_urls {
            println!("{:4}. {} {}", count, status_code.as_u16(), url);
            count += 1;
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

    async fn validate_urls(&self, urls: Vec<String>) -> Vec<(String, HttpStatusCode)> {
        let timeout = Duration::from_secs(10);
        let redirect_policy = Policy::limited(10);
        let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        let client = reqwest::Client::builder()
            .redirect(redirect_policy)
            .user_agent(user_agent)
            .timeout(timeout)
            .build()
            .unwrap();

        let mut urls_and_responses = stream::iter(urls)
            .map(|url| {
                let client = &client;
                async move { (url.clone(), client.head(&url).send().await) }
            })
            .buffer_unordered(THREAD_COUNT);

        let mut result = vec![];
        while let Some((url, response)) = urls_and_responses.next().await {
            let url_w_status_code: (String, HttpStatusCode) = match response {
                Ok(res) => (
                    url,
                    HttpStatusCode {
                        is_unknown: false,
                        num: res.status().as_u16(),
                    },
                ),
                Err(e) => {
                    if e.status().is_none() {
                        (
                            url,
                            HttpStatusCode {
                                is_unknown: true,
                                num: 999,
                            },
                        )
                    } else {
                        (
                            url,
                            HttpStatusCode {
                                is_unknown: false,
                                num: e.status().unwrap().as_u16(),
                            },
                        )
                    }
                }
            };

            result.push(url_w_status_code);
        }

        result
    }

    fn dedup(&self, mut list: Vec<String>) -> Vec<String> {
        list.sort();
        list.dedup();
        list
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
}
