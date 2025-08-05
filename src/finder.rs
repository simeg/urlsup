use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks::UTF8;
use linkify::{LinkFinder, LinkKind};
use once_cell::sync::Lazy;

use crate::UrlLocation;

use std::io;
use std::path::Path;

const MARKDOWN_URL_PATTERN: &str =
    r#"(http://|https://)[a-z0-9]+([-.]{1}[a-z0-9]+)*(.[a-z]{2,5})?(:[0-9]{1,5})?(/.*)?"#;

static REGEX_MATCHER: Lazy<RegexMatcher> = Lazy::new(|| {
    RegexMatcher::new(MARKDOWN_URL_PATTERN).expect("Failed to compile URL regex pattern")
});

pub trait UrlFinder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>>;
}

#[derive(Default, Debug)]
pub struct Finder {}

impl UrlFinder for Finder {
    fn find_urls(&self, paths: Vec<&Path>) -> io::Result<Vec<UrlLocation>> {
        let mut result = Vec::new();

        for path in paths {
            let url_matches = Finder::parse_lines_with_urls(path)?;
            for url_match in url_matches {
                let url_locations = Finder::parse_urls(url_match);
                result.extend(url_locations);
            }
        }

        Ok(result)
    }
}

type UrlMatch = (String, String, u64);

impl Finder {
    fn parse_lines_with_urls(path: &Path) -> io::Result<Vec<UrlMatch>> {
        let mut matches = vec![];
        Searcher::new().search_path(
            &*REGEX_MATCHER,
            path,
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

        let actual_match1 = actual.first().unwrap().to_owned();
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

    #[test]
    fn test_find_urls__multiple_files() -> TestResult {
        let mut file1 = tempfile::NamedTempFile::new()?;
        let mut file2 = tempfile::NamedTempFile::new()?;

        file1.write_all("First file with https://example.com".as_bytes())?;
        file2.write_all("Second file with https://test.org and https://demo.net".as_bytes())?;

        let finder = Finder::default();
        let paths = vec![file1.path(), file2.path()];
        let result = finder.find_urls(paths)?;

        assert_eq!(result.len(), 3);

        let urls: Vec<&str> = result.iter().map(|ul| ul.url.as_str()).collect();
        assert!(urls.contains(&"https://example.com"));
        assert!(urls.contains(&"https://test.org"));
        assert!(urls.contains(&"https://demo.net"));

        Ok(())
    }

    #[test]
    fn test_find_urls__with_recursive_structure() -> TestResult {
        use crate::path_utils::expand_paths;
        use std::collections::HashSet;
        use std::fs;

        // Create test directory structure
        let temp_dir = tempfile::tempdir()?;
        let base = temp_dir.path();

        fs::create_dir_all(base.join("docs"))?;
        fs::create_dir_all(base.join("src"))?;

        // Create files with URLs
        fs::write(
            base.join("README.md"),
            "# Project\nWebsite: https://project.com\nRepo: https://github.com/user/repo",
        )?;
        fs::write(
            base.join("docs/guide.md"),
            "Documentation at https://docs.example.com",
        )?;
        fs::write(base.join("src/main.rs"), "// See: https://rust-lang.org")?;

        // Use expand_paths to get all markdown files recursively
        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let expanded_paths = expand_paths(vec![base], true, Some(&extensions))?;
        let paths: Vec<&std::path::Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

        let finder = Finder::default();
        let result = finder.find_urls(paths)?;

        // Should find URLs from markdown files only
        assert_eq!(result.len(), 3); // project.com, github.com, docs.example.com

        let urls: Vec<&str> = result.iter().map(|ul| ul.url.as_str()).collect();
        assert!(urls.contains(&"https://project.com"));
        assert!(urls.contains(&"https://github.com/user/repo"));
        assert!(urls.contains(&"https://docs.example.com"));

        // Should not find rust-lang.org since .rs files were filtered out
        assert!(!urls.contains(&"https://rust-lang.org"));

        Ok(())
    }

    #[test]
    fn test_find_urls__empty_directory() -> TestResult {
        use crate::path_utils::expand_paths;
        use std::collections::HashSet;

        let temp_dir = tempfile::tempdir()?;

        // Create directory with no matching files
        let mut extensions = HashSet::new();
        extensions.insert("md".to_string());

        let expanded_paths = expand_paths(vec![temp_dir.path()], true, Some(&extensions))?;
        let paths: Vec<&std::path::Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

        let finder = Finder::default();
        let result = finder.find_urls(paths)?;

        assert_eq!(result.len(), 0);
        Ok(())
    }
}
