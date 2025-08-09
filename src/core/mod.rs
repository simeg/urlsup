//! Core types and foundational components
//!
//! This module contains the fundamental data types, error handling,
//! and constants used throughout the application.

pub mod constants;
pub mod error;
pub mod types;

// Re-export commonly used items for convenience
pub use error::{Result, UrlsUpError};
pub use types::{UrlLocation, UrlLocationBuilder, UrlLocationError};
