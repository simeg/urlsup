//! URL discovery and file processing
//!
//! This module handles finding URLs in files and managing
//! file path operations and directory traversal.

pub mod finder;
pub mod path_utils;

// Re-export commonly used items
pub use finder::{Finder, UrlFinder};
pub use path_utils::expand_paths;
