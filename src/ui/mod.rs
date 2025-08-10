//! User interface and interaction
//!
//! This module contains all components related to user interaction,
//! including CLI parsing, output formatting, progress reporting,
//! and shell completion generation.

pub mod cli;
pub mod color;
pub mod completion;
pub mod output;
pub mod progress;
pub mod rich;
pub mod theme;
pub mod wizard;

// Re-export commonly used items
pub use cli::{Cli, Commands, cli_to_config};
pub use completion::{install_completion, print_completions};
pub use output::DisplayMetadata;
pub use progress::ProgressReporter;
