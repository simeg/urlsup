use std::cmp::Ordering;
use std::fmt;

/// Represents a URL location found in a file.
///
/// This type tracks where a URL was discovered within the source files,
/// including the exact line number and file path for reporting purposes.
#[derive(Debug, Eq, Clone)]
pub struct UrlLocation {
    /// The URL that was found
    pub url: String,
    /// Line number where URL was found (1-indexed)
    pub line: u64,
    /// Name of file where URL was found
    pub file_name: String,
}

/// Builder for creating `UrlLocation` instances with validation.
#[derive(Debug, Default)]
pub struct UrlLocationBuilder {
    url: Option<String>,
    line: Option<u64>,
    file_name: Option<String>,
}

/// Errors that can occur when building a `UrlLocation`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlLocationError {
    /// URL is missing or empty
    MissingUrl,
    /// Line number is missing
    MissingLine,
    /// File name is missing or empty
    MissingFileName,
    /// Line number is invalid (zero)
    InvalidLineNumber,
}

impl fmt::Display for UrlLocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUrl => write!(f, "URL is required and cannot be empty"),
            Self::MissingLine => write!(f, "Line number is required"),
            Self::MissingFileName => write!(f, "File name is required and cannot be empty"),
            Self::InvalidLineNumber => write!(f, "Line number must be greater than 0"),
        }
    }
}

impl std::error::Error for UrlLocationError {}

impl Ord for UrlLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.url.cmp(&other.url)
    }
}

impl PartialOrd for UrlLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for UrlLocation {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl UrlLocation {
    /// Create a new UrlLocation with validation.
    ///
    /// # Arguments
    /// * `url` - The URL string (must not be empty)
    /// * `line` - Line number where URL was found (must be > 0)
    /// * `file_name` - Name of the file (must not be empty)
    ///
    /// # Examples
    /// ```
    /// use urlsup::types::UrlLocation;
    ///
    /// let location = UrlLocation::new(
    ///     "https://example.com".to_string(),
    ///     42,
    ///     "README.md".to_string()
    /// ).unwrap();
    /// assert_eq!(location.url, "https://example.com");
    /// assert_eq!(location.line, 42);
    /// ```
    pub fn new(url: String, line: u64, file_name: String) -> Result<Self, UrlLocationError> {
        if url.trim().is_empty() {
            return Err(UrlLocationError::MissingUrl);
        }
        if line == 0 {
            return Err(UrlLocationError::InvalidLineNumber);
        }
        if file_name.trim().is_empty() {
            return Err(UrlLocationError::MissingFileName);
        }

        Ok(Self {
            url: url.trim().to_string(),
            line,
            file_name: file_name.trim().to_string(),
        })
    }

    /// Create a new UrlLocation without validation (for compatibility).
    ///
    /// This method is provided for backward compatibility and internal use
    /// where validation has already been performed.
    #[allow(dead_code)] // Used in tests but not in main code
    pub(crate) fn new_unchecked(url: String, line: u64, file_name: String) -> Self {
        Self {
            url,
            line,
            file_name,
        }
    }

    /// Create a builder for constructing UrlLocation instances.
    pub fn builder() -> UrlLocationBuilder {
        UrlLocationBuilder::default()
    }

    /// Get the URL as a string slice.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the line number.
    pub fn line(&self) -> u64 {
        self.line
    }

    /// Get the file name as a string slice.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }
}

impl UrlLocationBuilder {
    /// Set the URL for this location.
    pub fn url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the line number for this location.
    pub fn line(mut self, line: u64) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the file name for this location.
    pub fn file_name<S: Into<String>>(mut self, file_name: S) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Build the UrlLocation, validating all required fields.
    pub fn build(self) -> Result<UrlLocation, UrlLocationError> {
        let url = self.url.ok_or(UrlLocationError::MissingUrl)?;
        let line = self.line.ok_or(UrlLocationError::MissingLine)?;
        let file_name = self.file_name.ok_or(UrlLocationError::MissingFileName)?;

        UrlLocation::new(url, line, file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_location_creation() {
        let url_location =
            UrlLocation::new("https://example.com".to_string(), 42, "test.md".to_string()).unwrap();

        assert_eq!(url_location.url(), "https://example.com");
        assert_eq!(url_location.line(), 42);
        assert_eq!(url_location.file_name(), "test.md");
    }

    #[test]
    fn test_url_location_creation_validation() {
        // Test empty URL
        let result = UrlLocation::new("".to_string(), 42, "test.md".to_string());
        assert!(matches!(result, Err(UrlLocationError::MissingUrl)));

        // Test zero line number
        let result = UrlLocation::new("https://example.com".to_string(), 0, "test.md".to_string());
        assert!(matches!(result, Err(UrlLocationError::InvalidLineNumber)));

        // Test empty file name
        let result = UrlLocation::new("https://example.com".to_string(), 42, "".to_string());
        assert!(matches!(result, Err(UrlLocationError::MissingFileName)));

        // Test whitespace trimming
        let url_location = UrlLocation::new(
            "  https://example.com  ".to_string(),
            42,
            "  test.md  ".to_string(),
        )
        .unwrap();
        assert_eq!(url_location.url(), "https://example.com");
        assert_eq!(url_location.file_name(), "test.md");
    }

    #[test]
    fn test_url_location_equality() {
        let url1 =
            UrlLocation::new("https://example.com".to_string(), 1, "file1.md".to_string()).unwrap();
        let url2 =
            UrlLocation::new("https://example.com".to_string(), 2, "file2.md".to_string()).unwrap();
        let url3 = UrlLocation::new(
            "https://different.com".to_string(),
            1,
            "file1.md".to_string(),
        )
        .unwrap();

        // Same URL should be equal even if from different lines/files
        assert_eq!(url1, url2);
        assert_ne!(url1, url3);
    }

    #[test]
    fn test_url_location_ordering() {
        let url1 = UrlLocation::new("https://a.com".to_string(), 1, "file.md".to_string()).unwrap();
        let url2 = UrlLocation::new("https://b.com".to_string(), 1, "file.md".to_string()).unwrap();

        assert!(url1 < url2);
        assert!(url2 > url1);
    }

    #[test]
    fn test_url_location_partial_ord() {
        let url1 =
            UrlLocation::new("https://example.com".to_string(), 1, "file.md".to_string()).unwrap();
        let url2 =
            UrlLocation::new("https://example.com".to_string(), 2, "file.md".to_string()).unwrap();

        assert_eq!(url1.partial_cmp(&url2), Some(Ordering::Equal));
    }

    #[test]
    fn test_url_location_clone() {
        let original =
            UrlLocation::new("https://example.com".to_string(), 1, "file.md".to_string()).unwrap();
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.url(), cloned.url());
        assert_eq!(original.line(), cloned.line());
        assert_eq!(original.file_name(), cloned.file_name());
    }

    #[test]
    fn test_url_location_debug() {
        let url_location =
            UrlLocation::new("https://example.com".to_string(), 1, "file.md".to_string()).unwrap();

        let debug_str = format!("{url_location:?}");
        assert!(debug_str.contains("https://example.com"));
        assert!(debug_str.contains("1"));
        assert!(debug_str.contains("file.md"));
    }

    #[test]
    fn test_url_location_builder() {
        let url_location = UrlLocation::builder()
            .url("https://example.com")
            .line(42)
            .file_name("test.md")
            .build()
            .unwrap();

        assert_eq!(url_location.url(), "https://example.com");
        assert_eq!(url_location.line(), 42);
        assert_eq!(url_location.file_name(), "test.md");
    }

    #[test]
    fn test_url_location_builder_missing_fields() {
        // Missing URL
        let result = UrlLocation::builder().line(42).file_name("test.md").build();
        assert!(matches!(result, Err(UrlLocationError::MissingUrl)));

        // Missing line
        let result = UrlLocation::builder()
            .url("https://example.com")
            .file_name("test.md")
            .build();
        assert!(matches!(result, Err(UrlLocationError::MissingLine)));

        // Missing file name
        let result = UrlLocation::builder()
            .url("https://example.com")
            .line(42)
            .build();
        assert!(matches!(result, Err(UrlLocationError::MissingFileName)));
    }

    #[test]
    fn test_url_location_error_display() {
        assert_eq!(
            UrlLocationError::MissingUrl.to_string(),
            "URL is required and cannot be empty"
        );
        assert_eq!(
            UrlLocationError::InvalidLineNumber.to_string(),
            "Line number must be greater than 0"
        );
        assert_eq!(
            UrlLocationError::MissingFileName.to_string(),
            "File name is required and cannot be empty"
        );
        assert_eq!(
            UrlLocationError::MissingLine.to_string(),
            "Line number is required"
        );
    }
}
