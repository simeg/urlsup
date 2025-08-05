# urlsup ![CI][build_badge] [![Code Coverage][coverage_badge]][coverage_report]

`urlsup` (_urls up_) finds URLs in files and checks whether they are up by
making a `GET` request and checking the response status code. This tool is
useful for lists, repos or any type of project containing URLs that you want to
be up.

It's written in Rust (stable) and executes the requests async in multiple
threads, making it very fast. This in combination with its ease of use makes
it the perfect tool for your CI pipeline.

This project is a slim version of
[`awesome_bot`](https://github.com/dkhamsing/awesome_bot) but a lot faster.

## 🎉 What's New in v2.0

**urlsup v2.0** introduces a modern CLI design with breaking changes for better usability:

### 🔄 Renamed Flags (Breaking Changes)
- `--white-list` → `--allowlist` (modern terminology)
- `--allow` → `--allow-status` (clearer naming)
- `--threads` → `--concurrency` (industry standard)
- `--file-types` → `--include` (shorter, clearer)

### ✨ New Features
- **📄 Configuration Files**: TOML-based config with automatic discovery
- **📤 Output Formats**: JSON support for automation (`--format json`)
- **📊 Progress Reporting**: Beautiful progress bars with real-time stats
- **🔍 Advanced Filtering**: Regex-based URL exclusion patterns
- **🔄 Retry Logic**: Configurable retry attempts with exponential backoff
- **⏱️ Rate Limiting**: Built-in request throttling
- **🔇 Quiet/Verbose Modes**: Better control over output verbosity
- **🚨 Enhanced Error Handling**: Comprehensive error types with context
- **⚡ Performance Optimizations**: Faster deduplication and connection pooling

## 🚀 Usage
```bash
CLI to validate URLs in files

Usage: urlsup [OPTIONS] <FILES>...

Arguments:
  <FILES>...  Files or directories to check

Options:
  -r, --recursive                Recursively process directories
  -t, --timeout <SECONDS>        Connection timeout in seconds (default: 30)
      --include <EXTENSIONS>     File extensions to process (e.g., md,html,txt)
      --allowlist <URLS>         URLs to allow (comma-separated)
      --allow-status <CODES>     Status codes to allow (comma-separated)
      --exclude-pattern <REGEX>  URL patterns to exclude (regex)
      --concurrency <COUNT>      Concurrent requests (default: CPU cores)
      --retry <COUNT>            Retry attempts for failed requests (default: 0)
      --retry-delay <MS>         Delay between retries in ms (default: 1000)
      --rate-limit <MS>          Delay between requests in ms (default: 0)
      --allow-timeout            Allow URLs that timeout
  -q, --quiet                    Suppress progress output
  -v, --verbose                  Enable verbose logging
      --format <FORMAT>          Output format [default: text] [possible values: text, json]
      --no-progress              Disable progress bars
      --user-agent <AGENT>       Custom User-Agent header
      --proxy <URL>              HTTP/HTTPS proxy URL
      --insecure                 Skip SSL certificate verification
      --config <FILE>            Use specific config file
      --no-config                Ignore config files
  -h, --help                     Print help
  -V, --version                  Print version
```

## 📝 Examples

### Basic File Checking
```bash
# Check a single file
$ urlsup README.md

# Check multiple files
$ urlsup README.md CHANGELOG.md

# Check files with wildcards
$ urlsup docs/*.md
```

### Directory Processing

**Important**: `urlsup` treats files and directories differently:

- **Files**: Directly processed (e.g., `urlsup README.md`)
- **Directories**: Must use `--recursive` flag (e.g., `urlsup --recursive docs/`)

```bash
# ❌ This will fail with an error
$ urlsup docs/
error: 'docs/' is a directory. Use --recursive to process directories.

# ✅ Process all files in a directory recursively
$ urlsup --recursive docs/

# ✅ Process only specific file types
$ urlsup --recursive --include md,txt docs/

# ✅ Process current directory recursively
$ urlsup --recursive .
```

### File Type Filtering
```bash
# Only check markdown and text files
$ urlsup --recursive --include md,txt .

# Only check web files
$ urlsup --recursive --include html,css,js website/

# Multiple extensions
$ urlsup --recursive --include md,rst,txt docs/
```

### Advanced Options
```bash
# Allow specific status codes
$ urlsup README.md --allow-status 403,429

# Set timeout and allow timeouts
$ urlsup README.md --allow-timeout -t 5

# Allowlist URLs (partial matches)
$ urlsup README.md --allowlist rust,crates

# Combine recursive with filtering and options
$ urlsup --recursive --include md --allow-status 403 --timeout 10 docs/

# Use quiet mode for scripts
$ urlsup --quiet --recursive docs/

# Enable verbose output for debugging
$ urlsup --verbose README.md

# Use JSON output format
$ urlsup --format json README.md

# Exclude URLs with patterns
$ urlsup --exclude-pattern ".*\.local$" --exclude-pattern "^http://localhost.*" docs/
```

### Git Integration

When using `--recursive`, `urlsup` automatically respects your `.gitignore` files:

```bash
# This will skip files/directories listed in .gitignore
$ urlsup --recursive .

# Examples of automatically ignored paths:
# - node_modules/
# - target/
# - .git/
# - *.log files
# - Any patterns in your .gitignore
```

This means you don't need to manually exclude build artifacts, dependencies, or other generated files.

## ⚙️ Configuration File

`urlsup` supports TOML configuration files for managing complex setups. Place a `.urlsup.toml` file in your project root:

```toml
# .urlsup.toml - Project configuration for urlsup v2.0
timeout = 30
threads = 8
allow_timeout = false
file_types = ["md", "html", "txt"]

# URL patterns to exclude (regex)
exclude_patterns = [
    "^https://example\\.com/private/.*",
    ".*\\.local$",
    "^http://localhost.*"
]

# URLs to allowlist
allowlist = [
    "https://api.github.com",
    "https://docs.rs"
]

# HTTP status codes to allow
allowed_status_codes = [403, 429]

# Advanced network settings
user_agent = "MyBot/1.0"
retry_attempts = 3
retry_delay = 1000  # milliseconds
rate_limit_delay = 100  # milliseconds between requests

# Security settings
skip_ssl_verification = false
proxy = "http://proxy.company.com:8080"

# Output settings
output_format = "text"  # or "json"
verbose = false
```

### Configuration Discovery

`urlsup` searches for configuration files in this order:
1. `.urlsup.toml` in current directory
2. `.urlsup.toml` in parent directories (up to 3 levels)
3. Default configuration if no file found

CLI arguments always override configuration file settings.

## 🔧 Advanced Features

### Retry Logic & Rate Limiting

Handle flaky networks and respect server limits:

```bash
# Configure via CLI (basic)
$ urlsup --timeout 60 README.md

# Configure via .urlsup.toml (advanced)
retry_attempts = 5
retry_delay = 2000
rate_limit_delay = 500
```

### URL Exclusion Patterns

Exclude URLs matching regex patterns:

```toml
# In .urlsup.toml
exclude_patterns = [
    "^https://internal\\.company\\.com/.*",  # Skip internal URLs
    ".*\\.local$",                           # Skip .local domains
    "^http://localhost.*",                   # Skip localhost
    "https://example\\.com/api/.*"           # Skip API endpoints
]
```

### 📊 Progress Reporting

Beautiful progress bars for large operations:

```bash
# Progress bars are enabled automatically for TTY terminals
$ urlsup --recursive docs/

# Output includes:
# ⠋ [00:01:23] [████████████████████████████████████████] 150/150 files processed
# ⠙ [00:00:45] [██████████████████████████              ] 245/320 URLs validated (76% successful)
```

### 🌐 Custom User Agent & Proxy Support

```toml
# In .urlsup.toml
user_agent = "MyCompany/URLChecker 2.0"
proxy = "http://proxy.company.com:8080"
skip_ssl_verification = false  # Set to true for internal/dev environments
```

### 📤 Output Formats

```bash
# Text output (default) - clean, emoji-based
$ urlsup README.md
✓ No issues found!

# JSON output for scripts and automation
$ urlsup --format json README.md
{"status": "success", "issues": []}

# Or configure in .urlsup.toml
output_format = "json"
```

### Verbose Logging

```bash
# Enable verbose output via CLI
$ urlsup --verbose README.md

# Or configure in .urlsup.toml
verbose = true

# Quiet mode for scripts (minimal output)
$ urlsup --quiet README.md
```

Verbose mode provides detailed information about:
- Files being processed
- URLs found and filtered
- Request progress and timing
- Configuration settings used

## 🔒 Security Features

### SSL Certificate Verification

```toml
# Skip SSL verification for internal/development URLs
skip_ssl_verification = true
```

**⚠️ Warning**: Only disable SSL verification for trusted internal environments.

### Proxy Support

```toml
# HTTP/HTTPS proxy configuration
proxy = "http://username:password@proxy.company.com:8080"
```

Supports both HTTP and HTTPS proxies with optional authentication.

## ⚡ Performance Optimizations

`urlsup` includes several performance optimizations:

- **Optimized Deduplication**: Uses `AHashSet` for O(1) URL deduplication instead of O(n²) sorting
- **Connection Pooling**: Reuses HTTP connections for better performance  
- **Async Processing**: Processes multiple URLs concurrently using configurable thread counts
- **Smart Caching**: Avoids redundant requests for duplicate URLs
- **Progress Tracking**: Minimal overhead progress reporting for large operations

## 🚨 Error Handling

Comprehensive error handling with specific error types:

- **Configuration errors**: Invalid TOML, missing files
- **Network errors**: Timeouts, connection failures, DNS resolution
- **Path errors**: Invalid file paths, permission issues
- **Validation errors**: Malformed URLs, regex compilation failures

All errors include helpful context and suggestions for resolution.

## 📦 Installation

Install with `cargo` to run `urlsup` on your local machine.

```bash
cargo install urlsup
```

## 🔄 GitHub Actions

See [`urlsup-action`](https://github.com/simeg/urlsup-action).

## 🛠️ Development

This repo uses a Makefile as an interface for common operations.

1) Do code changes
2) Run `make build link` to build the project and create a symlink from the built binary to the root
   of the project
3) Run `./urlsup` to execute the binary with your changes
4) Profit :star:

[build_badge]: https://github.com/simeg/urlsup/workflows/CI/badge.svg
[coverage_badge]: https://codecov.io/gh/simeg/urlsup/branch/master/graph/badge.svg?token=2bsQKkD1zg
[coverage_report]: https://codecov.io/gh/simeg/urlsup/branch/master
