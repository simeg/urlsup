use futures::{stream, StreamExt};
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use regex::Regex;
use reqwest::{Client, StatusCode};

use std::io::Error;
use std::path::Path;

const MARKDOWN_LINK_PATTERN: &str = r"\[[^\]]+\]\(<?([^)<>]+)>?\)";
const THREAD_COUNT: usize = 50;

pub struct Auditor {}

pub struct AuditorOptions {}

impl Auditor {
    pub async fn check(&self, paths: Vec<&Path>, _opts: AuditorOptions) {
        println!("> Checking links in {:?}", &paths);

        // Find links from files
        let links = self.find_links(paths);

        // Query them to see if they are up
        let val_results = self.validate_links(links).await;

        let invalid_links: Vec<(String, StatusCode)> = val_results
            .into_iter()
            .filter(|(_link, status)| !status.is_success())
            //            .map(|(_link, status)| (_link, StatusCode::NOT_FOUND))
            .collect();

        if invalid_links.is_empty() {
            println!("No issues!");
            std::process::exit(0)
        }

        println!("\n> Issues");
        let mut count = 1;
        for (link, status_code) in invalid_links {
            println!("{:4}. {} {}", count, status_code.as_u16(), link);
            count += 1;
        }
        std::process::exit(1)
    }

    fn find_links(&self, paths: Vec<&Path>) -> Vec<String> {
        let mut result = vec![];
        for path in paths {
            let links = self.search_for_links(path).unwrap_or_else(|_| {
                panic!(
                    "Something went wrong parsing links in file: {}",
                    path.display()
                )
            });

            result.extend(links.into_iter());
        }

        println!("Found {} links", &result.len());

        let mut count = 1;
        for link in &result {
            println!("{:4}. {}", count, link.to_string());
            count += 1;
        }
        result
    }

    fn search_for_links(&self, path: &Path) -> Result<Vec<String>, Error> {
        let matcher = RegexMatcher::new(MARKDOWN_LINK_PATTERN).unwrap();

        let mut matches = vec![];
        Searcher::new().search_path(
            &matcher,
            &path,
            UTF8(|_lnum, line| {
                matches.push(line.to_string());
                Ok(true)
            }),
        )?;

        let links: Vec<String> = matches
            .into_iter()
            .map(|mat| self.parse_link(mat))
            .map(|link| link.unwrap_or_else(|| "".to_string()))
            .filter(|link| !link.is_empty())
            .filter(|link| self.is_valid_link(link.to_string()))
            .map(|link| {
                // reqwest doesn't like links without protocol
                if !link.starts_with("http") {
                    // Use HTTP over HTTPS because not every site supports HTTPS
                    // If site supports HTTPS it might (should) redirect HTTP -> HTTPS
                    return ["http://", link.as_str()].concat();
                }

                link
            })
            .collect();

        Ok(links)
    }

    fn parse_link(&self, md_link: String) -> Option<String> {
        let md_link_pattern = r"\[[^]]+]\(<?([^)<>]+)>?\)";
        let matches = Regex::new(md_link_pattern).unwrap().captures(&md_link);

        match matches {
            None => None,
            Some(caps) => match caps.get(1) {
                None => None,
                Some(m) => Some(m.as_str().to_string()),
            },
        }
    }

    fn is_valid_link(&self, link: String) -> bool {
        // Relative links
        if link.starts_with("..") || link.starts_with('#') {
            return false;
        }

        true
    }

    async fn validate_links(&self, links: Vec<String>) -> Vec<(String, StatusCode)> {
        println!("Checking links...");

        let client = Client::new();
        let mut links_and_responses = stream::iter(links)
            .map(|link| {
                let client = &client;
                async move { (link.clone(), client.get(&link).send().await) }
            })
            .buffer_unordered(THREAD_COUNT);

        let mut result = vec![];
        while let Some((link, response)) = links_and_responses.next().await {
            let link_w_status_code = match response {
                Ok(res) => (link, res.status()),
                Err(_) => (link, StatusCode::INTERNAL_SERVER_ERROR),
            };

            result.push(link_w_status_code);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link() {
        let auditor = Auditor {};
        let md_link = "arbitrary [something](http://foo.bar) arbitrary".to_string();
        let expected = "http://foo.bar".to_string();
        let actual = auditor.parse_link(md_link).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_img_link() {
        let auditor = Auditor {};
        let md_link = "arbitrary ![image](http://foo.bar) arbitrary".to_string();
        let expected = "http://foo.bar".to_string();
        let actual = auditor.parse_link(md_link).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_bad_link() {
        let auditor = Auditor {};
        let md_link = "arbitrary [something]http://foo.bar arbitrary".to_string();
        let expected = None;
        let actual = auditor.parse_link(md_link);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_valid_link() {
        let auditor = Auditor {};
        for invalid_link in &["#arbitrary", "../arbitrary"] {
            let actual = auditor.is_valid_link(invalid_link.to_string());
            assert_eq!(actual, false);
        }

        for valid_link in &[
            "http://arbitrary",
            "https://arbitrary",
            "www.arbitrary.com",
            "arbitrary.com",
        ] {
            let actual = auditor.is_valid_link(valid_link.to_string());
            assert_eq!(actual, true);
        }
    }
}
