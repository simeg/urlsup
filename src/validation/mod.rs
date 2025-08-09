//! URL validation logic
//!
//! This module handles HTTP validation of URLs using
//! async requests and connection management.

pub mod validator;

// Re-export commonly used items
pub use validator::{ValidateUrls, ValidationResult, Validator};
