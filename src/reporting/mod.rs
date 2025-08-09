//! Analysis and reporting
//!
//! This module handles performance analysis, HTML dashboard generation,
//! and structured logging for the application.

pub mod dashboard;
pub mod logging;
pub mod performance;

// Re-export commonly used items
pub use dashboard::{DashboardData, HtmlDashboard};
pub use performance::PerformanceProfiler;
