# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`urlsup` is a CLI tool written in Rust that finds URLs in files and validates them by making HTTP requests. It's designed to be fast (async/concurrent) and useful for CI pipelines to ensure all URLs in documentation are accessible.

## Common Commands

### Development Build and Testing
```bash
# Build the project
make build

# Build and create symlink for local testing
make build link

# Run all CI checks (lint, clippy, test)
make ci

# Run tests with output
make test

# Format code
make fmt

# Check formatting (used in CI)
make lint

# Run clippy for additional linting
make clippy

# Release build
make release
```

### Running the Tool Locally
After running `make build link`, use:
```bash
./urlsup <files>...
```

## Architecture

The codebase follows a clean separation of concerns:

### Core Components

- **`Finder`** (`src/finder.rs`): Responsible for finding URLs in files using regex patterns and the `linkify` crate
- **`Validator`** (`src/validator.rs`): Handles async HTTP validation of URLs using `reqwest`
- **`Config`** (`src/config.rs`): Modern configuration system with TOML support and CLI merging
- **`ProgressReporter`** (`src/progress.rs`): Progress bars and status reporting using `indicatif`
- **`Color`** (`src/color.rs`): Terminal color and emoji utilities with capability detection
- **CLI Binary** (`src/bin/urlsup.rs`): Modern command-line interface using `clap` with colorful output

### Key Data Structures

- **`UrlLocation`**: Represents a URL found in a file with its location (line number, file name)
- **`ValidationResult`**: Contains the result of validating a URL (status code, error description)
- **`Config`**: Modern configuration structure supporting TOML files and CLI argument merging
- **`CliConfig`**: CLI-specific configuration that merges with file-based config

### Processing Flow

1. **Configuration Loading**: Loads TOML config files and merges with CLI arguments
2. **File Discovery**: Expands directories recursively with file type filtering
3. **URL Finding**: Uses `grep` crate with regex to find lines containing URLs
4. **URL Extraction**: Uses `linkify` crate to extract actual URLs from matched lines with file-type-aware capacity estimation
5. **Deduplication**: Removes duplicate URLs using optimized `FxHashSet` for maximum performance
6. **Progress Reporting**: Shows colorful progress bars with batched updates during processing
7. **Async Validation**: Uses `reqwest` with dynamic batch sizing, connection pooling, and token bucket rate limiting
8. **Filtering**: Applies allowlist, exclude patterns, and allowed status code filters
9. **Grouped Reporting**: Displays results grouped by error type with colors and emojis

## Output Formats

The tool supports three output formats:

- **`text`** (default): Colorful, emoji-enhanced output with configuration info, progress bars, and grouped error reporting
- **`json`**: Structured JSON output for automation and scripting
- **`minimal`**: Plain text output with no colors, emojis, config info, or grouping - ideal for simple CI/CD scripts

## Color and Emoji Support

The `src/color.rs` module provides:
- Terminal capability detection via `NO_COLOR` and `TERM` environment variables
- Conditional emoji rendering with text fallbacks
- ANSI color codes with automatic disabling for non-supporting terminals
- Test mode detection to ensure clean test output

### Testing Strategy

- Unit tests for individual components (URL parsing, filtering logic)
- Integration tests using `mockito` for HTTP mocking
- CLI integration tests using `assert_cmd`
- Temporary file testing with `tempfile` crate

## Performance Optimizations

The codebase includes several performance optimizations implemented in recent versions:

### Memory Management
- **File-type-aware capacity estimation** (`finder.rs`): Dynamic memory allocation based on file extensions (Markdown 2x, HTML 3x multipliers)
- **Optimized URL deduplication** (`validator.rs`): Uses `FxHashSet` for 15-20% faster performance vs standard HashMap
- **Pre-allocation strategies**: Smart capacity sizing to avoid expensive reallocations

### Network Performance
- **Dynamic batch sizing** (`validator.rs`): Batch sizes adapt to URL count and thread count (2-100 range)
- **Connection pooling**: Explicit HTTP connection reuse configuration with idle timeouts
- **Token bucket rate limiting**: Smooth request distribution vs simple sleep-based delays
- **Batched progress updates**: Reduces atomic operations by updating progress every 10 requests

### Async Processing
- **Adaptive concurrency**: Optimal batch sizes based on system resources and workload
- **Memory-efficient streaming**: Handles large URL sets without memory bloat
- **Static resource reuse**: Eliminates repeated allocations for parsing components

## Dependencies

- **Core**: `reqwest` (HTTP client), `tokio` (async runtime), `futures` (async utilities)
- **URL Processing**: `linkify` (URL extraction), `grep` (file searching), `rustc_hash` (fast hashing)
- **CLI**: `clap` (argument parsing), `indicatif` (progress bars)
- **Configuration**: `toml` (config file parsing), `serde` (serialization)
- **File Handling**: `ignore` (gitignore support), `walkdir` (directory traversal)
- **Testing**: `mockito` (HTTP mocking), `assert_cmd` (CLI testing), `tempfile` (temp files)