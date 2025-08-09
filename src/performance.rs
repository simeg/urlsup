use std::time::{Duration, Instant};
use sysinfo::System;

/// The number of bytes in a megabyte for memory calculations
const BYTES_PER_MB: f64 = 1_048_576.0;

/// Performance thresholds for generating recommendations
mod thresholds {
    use std::time::Duration;

    pub const HIGH_MEMORY_MB: f64 = 1000.0;
    pub const VERY_HIGH_MEMORY_MB: f64 = 2000.0;
    pub const LOW_CPU_USAGE_PERCENT: f32 = 50.0;
    pub const SLOW_VALIDATION_ITEMS_PER_SEC: f64 = 10.0;
    pub const SLOW_FILE_PROCESSING_ITEMS_PER_SEC: f64 = 100.0;
    pub const LONG_PROCESSING_TIME: Duration = Duration::from_secs(60);
}

/// Result of a single operation benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub operation: String,
    pub duration: Duration,
    pub items_processed: usize,
    pub memory_used: u64,
    pub cpu_usage: f32,
}

impl BenchmarkResult {
    /// Calculate throughput in items per second
    pub fn throughput(&self) -> f64 {
        if self.duration.as_millis() > 0 {
            self.items_processed as f64 / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Check if this operation is a validation operation
    pub fn is_validation(&self) -> bool {
        self.operation.contains("validation")
    }

    /// Check if this operation is a file processing operation
    pub fn is_file_processing(&self) -> bool {
        self.operation.contains("file_processing")
    }
}

/// Complete performance analysis report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub total_duration: Duration,
    pub operations: Vec<BenchmarkResult>,
    pub peak_memory_mb: f64,
    pub avg_cpu_usage: f32,
    pub recommendations: Vec<String>,
}

impl PerformanceReport {
    /// Create a new performance report from collected data
    fn new(
        total_duration: Duration,
        operations: Vec<BenchmarkResult>,
        memory_samples: &[u64],
        cpu_samples: &[f32],
    ) -> Self {
        let peak_memory_mb =
            memory_samples.iter().max().copied().unwrap_or_default() as f64 / BYTES_PER_MB;

        let avg_cpu_usage = if cpu_samples.is_empty() {
            0.0
        } else {
            cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32
        };

        let recommendations = Self::generate_recommendations(
            total_duration,
            &operations,
            peak_memory_mb,
            avg_cpu_usage,
        );

        Self {
            total_duration,
            operations,
            peak_memory_mb,
            avg_cpu_usage,
            recommendations,
        }
    }

    /// Generate performance recommendations based on metrics
    fn generate_recommendations(
        total_duration: Duration,
        operations: &[BenchmarkResult],
        peak_memory_mb: f64,
        avg_cpu_usage: f32,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Memory recommendations
        match peak_memory_mb {
            mb if mb > thresholds::VERY_HIGH_MEMORY_MB => {
                recommendations.push(
                    "Very high memory usage. Consider processing files in smaller batches"
                        .to_string(),
                );
            }
            mb if mb > thresholds::HIGH_MEMORY_MB => {
                recommendations.push(
                    "High memory usage detected. Consider using --concurrency flag to reduce parallel processing".to_string()
                );
            }
            _ => {}
        }

        // CPU recommendations
        if avg_cpu_usage < thresholds::LOW_CPU_USAGE_PERCENT {
            recommendations.push(
                "Low CPU utilization. Consider increasing --concurrency for better performance"
                    .to_string(),
            );
        }

        // Duration recommendations
        if total_duration > thresholds::LONG_PROCESSING_TIME {
            recommendations.extend([
                "Long processing time. Consider using --include flag to filter file types"
                    .to_string(),
                "Consider using --exclude-pattern to skip non-essential URLs".to_string(),
            ]);
        }

        // Operation-specific recommendations
        for benchmark in operations {
            let throughput = benchmark.throughput();

            if benchmark.is_validation() && throughput < thresholds::SLOW_VALIDATION_ITEMS_PER_SEC {
                recommendations.push(
                    "Slow URL validation. Consider increasing --timeout or using --head requests"
                        .to_string(),
                );
            }

            if benchmark.is_file_processing()
                && throughput < thresholds::SLOW_FILE_PROCESSING_ITEMS_PER_SEC
            {
                recommendations.push(
                    "Slow file processing. Consider using SSD storage or reducing file sizes"
                        .to_string(),
                );
            }
        }

        recommendations
    }
}

/// System metrics data
#[derive(Debug, Clone)]
struct SystemMetrics {
    memory_used: u64,
    cpu_usage: f32,
}

/// System resource and performance profiler
pub struct PerformanceProfiler {
    system: System,
    start_time: Instant,
    benchmarks: Vec<BenchmarkResult>,
    memory_samples: Vec<u64>,
    cpu_samples: Vec<f32>,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system,
            start_time: Instant::now(),
            benchmarks: Vec::new(),
            memory_samples: Vec::new(),
            cpu_samples: Vec::new(),
        }
    }

    /// Start timing a new operation
    pub fn start_operation(&mut self, operation: &str) -> OperationTimer {
        self.refresh_system();
        OperationTimer::new(operation)
    }

    /// Finish timing an operation and record the results
    pub fn finish_operation(&mut self, timer: OperationTimer, items_processed: usize) {
        let timer_result = timer.finish(items_processed);
        let system_metrics = self.get_system_metrics();

        self.memory_samples.push(system_metrics.memory_used);
        self.cpu_samples.push(system_metrics.cpu_usage);

        let benchmark = BenchmarkResult {
            operation: timer_result.operation,
            duration: timer_result.duration,
            items_processed,
            memory_used: system_metrics.memory_used,
            cpu_usage: system_metrics.cpu_usage,
        };

        self.benchmarks.push(benchmark);
    }

    /// Generate a complete performance report
    pub fn generate_report(&self) -> PerformanceReport {
        PerformanceReport::new(
            self.start_time.elapsed(),
            self.benchmarks.clone(),
            &self.memory_samples,
            &self.cpu_samples,
        )
    }

    /// Display a colorful performance summary to the user
    pub fn display_performance_summary(&self) {
        let report = self.generate_report();

        println!("\n📊 \x1b[96m\x1b[1mPerformance Summary\x1b[0m");
        println!(
            "   \x1b[2mTotal Duration\x1b[0m: \x1b[97m{:?}\x1b[0m",
            report.total_duration
        );
        println!(
            "   \x1b[2mPeak Memory\x1b[0m: \x1b[97m{:.2} MB\x1b[0m",
            report.peak_memory_mb
        );
        println!(
            "   \x1b[2mAvg CPU Usage\x1b[0m: \x1b[97m{:.1}%\x1b[0m",
            report.avg_cpu_usage
        );

        if !report.operations.is_empty() {
            println!("\n   \x1b[2mOperation Breakdown\x1b[0m:");
            for benchmark in &report.operations {
                let throughput = benchmark.throughput() as u64;

                println!(
                    "   \x1b[2m•\x1b[0m \x1b[36m{}\x1b[0m: \x1b[97m{:?}\x1b[0m (\x1b[2m{} items, {} items/sec\x1b[0m)",
                    benchmark.operation, benchmark.duration, benchmark.items_processed, throughput
                );
            }
        }

        if !report.recommendations.is_empty() {
            println!("\n💡 \x1b[93m\x1b[1mPerformance Recommendations\x1b[0m:");
            for rec in &report.recommendations {
                println!("   \x1b[2m•\x1b[0m {}", rec);
            }
        }
    }

    /// Refresh system information
    fn refresh_system(&mut self) {
        self.system.refresh_memory();
        self.system.refresh_cpu_all();
    }

    /// Get current system metrics
    fn get_system_metrics(&self) -> SystemMetrics {
        let memory_used = self.get_memory_usage();
        let cpu_usage = self.get_cpu_usage();

        SystemMetrics {
            memory_used,
            cpu_usage,
        }
    }

    /// Get current process memory usage
    fn get_memory_usage(&self) -> u64 {
        sysinfo::get_current_pid()
            .ok()
            .and_then(|pid| self.system.process(pid))
            .map(|process| process.memory())
            .unwrap_or(0)
    }

    /// Get current process CPU usage
    fn get_cpu_usage(&self) -> f32 {
        sysinfo::get_current_pid()
            .ok()
            .and_then(|pid| self.system.process(pid))
            .map(|process| process.cpu_usage())
            .unwrap_or(0.0)
    }
}

/// Timer for measuring operation duration
pub struct OperationTimer {
    operation: String,
    start_time: Instant,
}

impl OperationTimer {
    /// Create a new operation timer
    fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            start_time: Instant::now(),
        }
    }

    /// Finish timing and return the result
    fn finish(self, _items_processed: usize) -> TimerResult {
        TimerResult {
            operation: self.operation,
            duration: self.start_time.elapsed(),
        }
    }
}

/// Result of a completed timer operation
struct TimerResult {
    operation: String,
    duration: Duration,
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_profiler_basic() {
        let mut profiler = PerformanceProfiler::new();

        // Simulate some work
        let timer = profiler.start_operation("test_operation");
        thread::sleep(Duration::from_millis(10));
        profiler.finish_operation(timer, 100);

        let report = profiler.generate_report();
        assert_eq!(report.operations.len(), 1);
        assert_eq!(report.operations[0].operation, "test_operation");
        assert_eq!(report.operations[0].items_processed, 100);
    }

    #[test]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult {
            operation: "test".to_string(),
            duration: Duration::from_secs(1),
            items_processed: 100,
            memory_used: 1024,
            cpu_usage: 50.0,
        };

        assert_eq!(result.operation, "test");
        assert_eq!(result.duration, Duration::from_secs(1));
        assert_eq!(result.items_processed, 100);
    }

    #[test]
    fn test_recommendations_generation() {
        let mut profiler = PerformanceProfiler::new();

        // Add a slow operation to trigger recommendations
        profiler.benchmarks.push(BenchmarkResult {
            operation: "validation".to_string(),
            duration: Duration::from_secs(10),
            items_processed: 50, // Low throughput: 5 items/sec
            memory_used: 1024,
            cpu_usage: 30.0,
        });

        let report = profiler.generate_report();
        assert!(!report.recommendations.is_empty());
    }
}
