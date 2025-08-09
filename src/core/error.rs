use std::fmt;

/// Comprehensive error types for urlsup operations
#[derive(Debug)]
pub enum UrlsUpError {
    /// IO error (file operations, etc.)
    Io(std::io::Error),

    /// Configuration error
    Config(String),

    /// URL validation error
    Validation(String),

    /// HTTP client error
    Http(reqwest::Error),

    /// Path expansion error
    PathExpansion(String),

    /// Regex compilation error
    Regex(regex::Error),

    /// TOML parsing error
    TomlParsing(toml::de::Error),

    /// File not found error
    FileNotFound(String),

    /// Invalid argument error
    InvalidArgument(String),

    /// File walking/ignore error
    FileWalking(ignore::Error),
}

impl fmt::Display for UrlsUpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlsUpError::Io(err) => write!(f, "IO error: {err}"),
            UrlsUpError::Config(msg) => write!(f, "Configuration error: {msg}"),
            UrlsUpError::Validation(msg) => write!(f, "Validation error: {msg}"),
            UrlsUpError::Http(err) => write!(f, "HTTP error: {err}"),
            UrlsUpError::PathExpansion(msg) => write!(f, "Path expansion error: {msg}"),
            UrlsUpError::Regex(err) => write!(f, "Regex error: {err}"),
            UrlsUpError::TomlParsing(err) => write!(f, "TOML parsing error: {err}"),
            UrlsUpError::FileNotFound(path) => write!(f, "File not found: {path}"),
            UrlsUpError::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            UrlsUpError::FileWalking(err) => write!(f, "File walking error: {err}"),
        }
    }
}

impl std::error::Error for UrlsUpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UrlsUpError::Io(err) => Some(err),
            UrlsUpError::Http(err) => Some(err),
            UrlsUpError::Regex(err) => Some(err),
            UrlsUpError::TomlParsing(err) => Some(err),
            UrlsUpError::FileWalking(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for UrlsUpError {
    fn from(err: std::io::Error) -> Self {
        UrlsUpError::Io(err)
    }
}

impl From<reqwest::Error> for UrlsUpError {
    fn from(err: reqwest::Error) -> Self {
        UrlsUpError::Http(err)
    }
}

impl From<regex::Error> for UrlsUpError {
    fn from(err: regex::Error) -> Self {
        UrlsUpError::Regex(err)
    }
}

impl From<toml::de::Error> for UrlsUpError {
    fn from(err: toml::de::Error) -> Self {
        UrlsUpError::TomlParsing(err)
    }
}

impl From<ignore::Error> for UrlsUpError {
    fn from(err: ignore::Error) -> Self {
        UrlsUpError::FileWalking(err)
    }
}

/// Type alias for Results using UrlsUpError
pub type Result<T> = std::result::Result<T, UrlsUpError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_error_display() {
        let config_error = UrlsUpError::Config("Invalid timeout".to_string());
        assert_eq!(
            format!("{config_error}"),
            "Configuration error: Invalid timeout"
        );

        let file_error = UrlsUpError::FileNotFound("/path/to/file".to_string());
        assert_eq!(format!("{file_error}"), "File not found: /path/to/file");
    }

    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let urlsup_error = UrlsUpError::from(io_error);

        match urlsup_error {
            UrlsUpError::Io(_) => {} // Expected
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_error_from_reqwest() {
        // Create a dummy reqwest client and try to get a URL that will fail
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_error = rt.block_on(async {
            reqwest::get("http://invalid-domain-that-does-not-exist.com")
                .await
                .unwrap_err()
        });
        let urlsup_error = UrlsUpError::from(reqwest_error);

        match urlsup_error {
            UrlsUpError::Http(_) => {} // Expected
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    #[allow(clippy::invalid_regex)]
    fn test_error_from_regex() {
        let regex_error = regex::Regex::new("[invalid").unwrap_err();
        let urlsup_error = UrlsUpError::from(regex_error);

        match urlsup_error {
            UrlsUpError::Regex(_) => {} // Expected
            _ => panic!("Expected Regex variant"),
        }
    }

    #[test]
    fn test_error_from_toml() {
        let toml_error = toml::from_str::<toml::Value>("invalid toml [").unwrap_err();
        let urlsup_error = UrlsUpError::from(toml_error);

        match urlsup_error {
            UrlsUpError::TomlParsing(_) => {} // Expected
            _ => panic!("Expected TomlParsing variant"),
        }
    }

    #[test]
    fn test_string_error_variants_display() {
        let errors = vec![
            UrlsUpError::Config("Bad config".to_string()),
            UrlsUpError::Validation("Invalid URL".to_string()),
            UrlsUpError::PathExpansion("Path error".to_string()),
            UrlsUpError::FileNotFound("/missing".to_string()),
            UrlsUpError::InvalidArgument("Bad arg".to_string()),
        ];

        for error in errors {
            let display_str = format!("{error}");
            assert!(!display_str.is_empty());
            assert!(display_str.contains(":"));
        }
    }

    #[test]
    fn test_error_source() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let urlsup_error = UrlsUpError::Io(io_error);

        assert!(urlsup_error.source().is_some());

        let config_error = UrlsUpError::Config("test".to_string());
        assert!(config_error.source().is_none());
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UrlsUpError>();
    }

    #[test]
    fn test_result_type_alias() {
        let success: Result<i32> = Ok(42);
        let error: Result<i32> = Err(UrlsUpError::Config("test".to_string()));

        assert!(success.is_ok());
        assert!(error.is_err());
        if let Ok(value) = success {
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_error_source_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let urlsup_error = UrlsUpError::Io(io_error);

        // Test that source is properly accessible
        let source = urlsup_error.source();
        assert!(source.is_some());

        let source_display = format!("{}", source.unwrap());
        assert!(source_display.contains("file not found"));
    }

    #[test]
    fn test_error_debug_format() {
        let errors = vec![
            UrlsUpError::Config("debug config".to_string()),
            UrlsUpError::Validation("debug validation".to_string()),
            UrlsUpError::PathExpansion("debug path".to_string()),
            UrlsUpError::FileNotFound("debug file".to_string()),
            UrlsUpError::InvalidArgument("debug arg".to_string()),
        ];

        for error in errors {
            let debug_str = format!("{error:?}");
            assert!(!debug_str.is_empty());
            assert!(debug_str.contains("debug"));
        }
    }

    #[test]
    fn test_error_conversion_coverage() {
        // Test all From implementations
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission");
        let converted_io = UrlsUpError::from(io_err);
        matches!(converted_io, UrlsUpError::Io(_));

        // Test regex error conversion
        #[allow(clippy::invalid_regex)]
        let regex_err = regex::Regex::new("[invalid").unwrap_err();
        let converted_regex = UrlsUpError::from(regex_err);
        matches!(converted_regex, UrlsUpError::Regex(_));

        // Test TOML error conversion
        let toml_err = toml::from_str::<toml::Value>("invalid [ toml").unwrap_err();
        let converted_toml = UrlsUpError::from(toml_err);
        matches!(converted_toml, UrlsUpError::TomlParsing(_));
    }

    #[test]
    fn test_error_no_source_variants() {
        let errors_without_source = vec![
            UrlsUpError::Config("test".to_string()),
            UrlsUpError::Validation("test".to_string()),
            UrlsUpError::PathExpansion("test".to_string()),
            UrlsUpError::FileNotFound("test".to_string()),
            UrlsUpError::InvalidArgument("test".to_string()),
        ];

        for error in errors_without_source {
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn test_all_error_variants_display_and_source_comprehensive() {
        // Test all error variants for both Display (fmt) and source() methods

        // Create actual underlying errors for variants that have source
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test io");
        #[allow(clippy::invalid_regex)]
        let regex_error = regex::Regex::new("[invalid").unwrap_err();
        let toml_error = toml::from_str::<toml::Value>("invalid [ toml").unwrap_err();

        // Create a reqwest error
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_error = rt.block_on(async {
            reqwest::get("http://invalid-domain-that-does-not-exist-12345.com")
                .await
                .unwrap_err()
        });

        // Create an ignore error by trying to walk a non-existent directory
        let ignore_error = ignore::WalkBuilder::new("/non/existent/path/12345")
            .build()
            .next()
            .unwrap()
            .unwrap_err();

        // Create all error variants
        let all_errors = vec![
            ("Io", UrlsUpError::Io(io_error), true),
            (
                "Config",
                UrlsUpError::Config("test config".to_string()),
                false,
            ),
            (
                "Validation",
                UrlsUpError::Validation("test validation".to_string()),
                false,
            ),
            ("Http", UrlsUpError::Http(reqwest_error), true),
            (
                "PathExpansion",
                UrlsUpError::PathExpansion("test path".to_string()),
                false,
            ),
            ("Regex", UrlsUpError::Regex(regex_error), true),
            ("TomlParsing", UrlsUpError::TomlParsing(toml_error), true),
            (
                "FileNotFound",
                UrlsUpError::FileNotFound("test file".to_string()),
                false,
            ),
            (
                "InvalidArgument",
                UrlsUpError::InvalidArgument("test arg".to_string()),
                false,
            ),
            ("FileWalking", UrlsUpError::FileWalking(ignore_error), true),
        ];

        // Test each variant
        for (variant_name, error, should_have_source) in all_errors {
            // Test Display implementation (fmt)
            let display_str = format!("{}", error);
            assert!(
                !display_str.is_empty(),
                "Display should not be empty for {}",
                variant_name
            );
            // Check that the display string contains a colon (all error messages have "type: message" format)
            assert!(
                display_str.contains(":"),
                "Display should contain colon for {}: {}",
                variant_name,
                display_str
            );

            // Test source() method
            let has_source = error.source().is_some();
            assert_eq!(
                has_source, should_have_source,
                "Source mismatch for {}: expected {}, got {}",
                variant_name, should_have_source, has_source
            );

            // For errors with source, verify the source is accessible
            if should_have_source {
                let source = error.source().unwrap();
                let source_str = format!("{}", source);
                assert!(
                    !source_str.is_empty(),
                    "Source should not be empty for {}",
                    variant_name
                );
            }

            // Test Debug implementation
            let debug_str = format!("{:?}", error);
            assert!(
                !debug_str.is_empty(),
                "Debug should not be empty for {}",
                variant_name
            );
            assert!(
                debug_str.contains(variant_name),
                "Debug should contain variant name '{}': {}",
                variant_name,
                debug_str
            );
        }
    }

    #[test]
    fn test_all_from_implementations_comprehensive() {
        // Test that all From implementations work correctly and produce the right variant

        // Test From<std::io::Error>
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "perm denied");
        let converted = UrlsUpError::from(io_err);
        assert!(matches!(converted, UrlsUpError::Io(_)));
        assert!(converted.source().is_some());
        assert!(format!("{}", converted).contains("IO error:"));

        // Test From<reqwest::Error>
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_err = rt.block_on(async {
            reqwest::get("http://definitely-invalid-domain-12345.com")
                .await
                .unwrap_err()
        });
        let converted = UrlsUpError::from(reqwest_err);
        assert!(matches!(converted, UrlsUpError::Http(_)));
        assert!(converted.source().is_some());
        assert!(format!("{}", converted).contains("HTTP error:"));

        // Test From<regex::Error>
        #[allow(clippy::invalid_regex)]
        let regex_err = regex::Regex::new("*invalid").unwrap_err();
        let converted = UrlsUpError::from(regex_err);
        assert!(matches!(converted, UrlsUpError::Regex(_)));
        assert!(converted.source().is_some());
        assert!(format!("{}", converted).contains("Regex error:"));

        // Test From<toml::de::Error>
        let toml_err = toml::from_str::<toml::Value>("invalid ] toml").unwrap_err();
        let converted = UrlsUpError::from(toml_err);
        assert!(matches!(converted, UrlsUpError::TomlParsing(_)));
        assert!(converted.source().is_some());
        assert!(format!("{}", converted).contains("TOML parsing error:"));

        // Test From<ignore::Error>
        let ignore_err = ignore::WalkBuilder::new("/definitely/nonexistent/path/12345")
            .build()
            .next()
            .unwrap()
            .unwrap_err();
        let converted = UrlsUpError::from(ignore_err);
        assert!(matches!(converted, UrlsUpError::FileWalking(_)));
        assert!(converted.source().is_some());
        assert!(format!("{}", converted).contains("File walking error:"));
    }
}
