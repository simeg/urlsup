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
}
