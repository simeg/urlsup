use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

pub struct ProgressReporter {
    multi_progress: Arc<MultiProgress>,
    file_progress: Option<ProgressBar>,
    url_progress: Option<ProgressBar>,
    enabled: bool,
}

impl ProgressReporter {
    pub fn new(enabled: bool) -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
            file_progress: None,
            url_progress: None,
            enabled,
        }
    }

    pub fn start_file_processing(&mut self, total_files: usize) {
        if !self.enabled {
            return;
        }

        let pb = self
            .multi_progress
            .add(ProgressBar::new(total_files as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files processed ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Finding URLs in files");
        self.file_progress = Some(pb);
    }

    pub fn update_file_progress(&self, current: usize) {
        if let Some(ref pb) = self.file_progress {
            pb.set_position(current as u64);
        }
    }

    pub fn finish_file_processing(&self) {
        if let Some(ref pb) = self.file_progress {
            pb.finish_with_message("✓ File processing complete");
        }
    }

    pub fn start_url_validation(&mut self, total_urls: usize) {
        if !self.enabled {
            return;
        }

        let pb = self.multi_progress.add(ProgressBar::new(total_urls as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.yellow/red}] {pos}/{len} URLs validated ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Validating URLs");
        pb.enable_steady_tick(Duration::from_millis(120));
        self.url_progress = Some(pb);
    }

    pub fn update_url_progress(&self, current: usize) {
        if let Some(ref pb) = self.url_progress {
            pb.set_position(current as u64);
        }
    }

    pub fn finish_url_validation(&self, success_count: usize, total_count: usize) {
        if let Some(ref pb) = self.url_progress {
            let message = if success_count == total_count {
                "✓ All URLs validated successfully".to_string()
            } else {
                format!("✓ Validation complete ({success_count}/{total_count} successful)")
            };
            pb.finish_with_message(message);
        }
    }

    pub fn log_info(&self, message: &str) {
        if self.enabled {
            self.multi_progress
                .println(format!("ℹ {message}"))
                .unwrap_or(());
        }
    }

    pub fn log_warning(&self, message: &str) {
        if self.enabled {
            self.multi_progress
                .println(format!("⚠ {message}"))
                .unwrap_or(());
        }
    }

    pub fn log_error(&self, message: &str) {
        if self.enabled {
            self.multi_progress
                .println(format!("✗ {message}"))
                .unwrap_or(());
        }
    }

    /// Create a simple spinner for indeterminate progress
    pub fn create_spinner(&self, message: &str) -> Option<ProgressBar> {
        if !self.enabled {
            return None;
        }

        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    }

    pub fn get_multi_progress(&self) -> Arc<MultiProgress> {
        self.multi_progress.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_reporter_creation() {
        let reporter = ProgressReporter::new(true);
        assert!(reporter.enabled);
        assert!(reporter.file_progress.is_none());
        assert!(reporter.url_progress.is_none());
    }

    #[test]
    fn test_progress_reporter_disabled() {
        let reporter = ProgressReporter::new(false);
        assert!(!reporter.enabled);
    }

    #[test]
    fn test_progress_methods_dont_panic() {
        let mut reporter = ProgressReporter::new(false);

        // These should not panic even when disabled
        reporter.start_file_processing(10);
        reporter.update_file_progress(5);
        reporter.finish_file_processing();

        reporter.start_url_validation(20);
        reporter.update_url_progress(10);
        reporter.finish_url_validation(18, 20);

        reporter.log_info("test");
        reporter.log_warning("test");
        reporter.log_error("test");
    }
}
