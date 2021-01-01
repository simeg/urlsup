use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use linkify::{LinkFinder, LinkKind};

use crate::UrlLocation;

use std::io;
use std::path::Path;

const MARKDOWN_URL_PATTERN: &str =
    r#"(http://|https://)[a-z0-9]+([-.]{1}[a-z0-9]+)*(.[a-z]{2,5})?(:[0-9]{1,5})?(/.*)?"#;

pub trait UrlFinder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>>;
}

pub struct Finder {}

impl Default for Finder {
    fn default() -> Self {
        Self {}
    }
}

impl UrlFinder for Finder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>> {
        let result = paths
            .into_iter()
            .flat_map(|path| {
                // TODO: Don't panic here but instead let Error propagate in return Result
                Finder::parse_lines_with_urls(path).unwrap_or_else(|_| {
                    panic!(
                        "Something went wrong parsing URL in file: {}",
                        path.display()
                    )
                })
            })
            .flat_map(Finder::parse_urls)
            .collect();

        Ok(result)
    }
}

type UrlMatch = (String, String, u64);

impl Finder {
    fn parse_lines_with_urls(path: &Path) -> io::Result<Vec<UrlMatch>> {
        let matcher = RegexMatcher::new(MARKDOWN_URL_PATTERN).unwrap();

        let mut matches = vec![];
        Searcher::new().search_path(
            &matcher,
            &path,
            UTF8(|line_number, line| {
                let file_name = path.display().to_string();
                let url_match: UrlMatch = (line.to_string(), file_name, line_number);
                matches.push(url_match);
                Ok(true)
            }),
        )?;

        Ok(matches)
    }

    fn parse_urls(url_match: UrlMatch) -> Vec<UrlLocation> {
        let (url, file_name, line) = url_match;

        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);

        finder
            .links(url.as_str())
            .map(|url| UrlLocation {
                line,
                file_name: file_name.to_owned(),
                url: url.as_str().to_string(),
            })
            .collect()
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
        let md_link =
            "arbitrary [something](http://foo.bar) arbitrary http://foo2.bar arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![
            UrlLocation {
                url: "http://foo.bar".to_string(),
                line: 99,
                file_name: "this-file-name".to_string(),
            },
            UrlLocation {
                url: "http://foo2.bar".to_string(),
                line: 99,
                file_name: "this-file-name".to_string(),
            },
        ];
        let actual = Finder::parse_urls(url_match);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_urls__img_url() {
        let md_link = "arbitrary ![image](http://foo.bar) arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![UrlLocation {
            url: "http://foo.bar".to_string(),
            line: 99,
            file_name: "this-file-name".to_string(),
        }];
        let actual = Finder::parse_urls(url_match);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_urls__badge_url() {
        let md_link = "arbitrary [something]: http://foo.bar arbitrary".to_string();
        let url_match = (md_link, "this-file-name".to_string(), 99);

        let expected = vec![UrlLocation {
            url: "http://foo.bar".to_string(),
            line: 99,
            file_name: "this-file-name".to_string(),
        }];
        let actual = Finder::parse_urls(url_match);

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

        let actual_match1 = actual.get(0).unwrap().to_owned();
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
}
