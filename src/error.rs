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

/// Type alias for Results using UrlsUpError
pub type Result<T> = std::result::Result<T, UrlsUpError>;

#[cfg(test)]
mod tests {
    use super::*;

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
}
