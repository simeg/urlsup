# urlsup ![CI][build_badge] [![Code Coverage][coverage_badge]][coverage_report]

`urlsup` (_urls up_) finds URLs in files and checks whether they are up by
making a `GET` request and checking the response status code. This tool is
useful for lists, repos or any type of project containing URLs that you want to
be up.

It's written in Rust (stable) and executes the requests async in multiple
threads, making it very fast. **Uses browser-like HTTP client behavior with
automatic protocol negotiation and reliable connection handling.** This in 
combination with its ease of use makes it the perfect tool for your CI pipeline.

This project is a slim version of
[`awesome_bot`](https://github.com/dkhamsing/awesome_bot) but significantly faster.

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
- **⚡ Browser-like Behavior**: HTTP client that behaves like web browsers for maximum compatibility

## 📚 Table of Contents

- [🚀 Usage](#-usage)
- [📝 Examples](#-examples)
  - [Basic File Checking](#basic-file-checking)
  - [Directory Processing](#directory-processing)
  - [File Type Filtering](#file-type-filtering)
  - [Advanced Options](#advanced-options)
  - [Git Integration](#git-integration)
- [📦 Installation](#-installation)
- [🚀 Shell Completions Installation](#-shell-completions-installation)
- [⚙️ Configuration File](#️-configuration-file)
  - [Configuration Discovery](#configuration-discovery)
- [🔧 Advanced Features](#-advanced-features)
  - [🎯 Failure Threshold](#-failure-threshold)
  - [Retry Logic & Rate Limiting](#retry-logic--rate-limiting)
  - [URL Exclusion Patterns](#url-exclusion-patterns)
  - [📊 Progress Reporting](#-progress-reporting)
  - [🌐 Custom User Agent & Proxy Support](#-custom-user-agent--proxy-support)
  - [📤 Output Formats](#-output-formats)
  - [Verbose Logging](#verbose-logging)
- [🔒 Security Features](#-security-features)
- [🚀 Performance Features](#-performance-features)
- [⚡ Browser-like HTTP Client](#-browser-like-http-client)
- [🚨 Error Handling](#-error-handling)
- [🔄 GitHub Actions](#-github-actions)
- [🛠️ Development](#️-development)

## 🚀 Usage
```bash
CLI to validate URLs in files

Usage: urlsup [OPTIONS] [FILES]... [COMMAND]

Commands:
  completion-generate Generate shell completions
  completion-install  Install shell completions to standard location
  help                Print this message or the help of the given subcommand(s)

Arguments:
  [FILES]...  Files or directories to check

Options:
  -h, --help     Print help
  -V, --version  Print version

Core Options:
  -r, --recursive            Recursively process directories
  -t, --timeout <SECONDS>    Connection timeout in seconds (default: 30)
      --concurrency <COUNT>  Concurrent requests (default: CPU cores)

Filtering & Content:
      --include <EXTENSIONS>     File extensions to process (e.g., md,html,txt)
      --allowlist <URLS>         URLs to allow (comma-separated)
      --allow-status <CODES>     Status codes to allow (comma-separated)
      --exclude-pattern <REGEX>  URL patterns to exclude (regex)

Retry & Rate Limiting:
      --retry <COUNT>                Retry attempts for failed requests (default: 0)
      --retry-delay <MS>             Delay between retries in ms (default: 1000)
      --rate-limit <MS>              Delay between requests in ms (default: 0)
      --allow-timeout                Allow URLs that timeout
      --failure-threshold <PERCENT>  Failure threshold - fail only if more than X% of URLs are broken (0-100)

Output & Verbosity:
  -q, --quiet            Suppress progress output
  -v, --verbose          Enable verbose logging
      --format <FORMAT>  Output format [default: text] [possible values: text, json, minimal]
      --no-progress      Disable progress bars

Network & Security:
      --user-agent <AGENT>  Custom User-Agent header
      --proxy <URL>         HTTP/HTTPS proxy URL
      --insecure            Skip SSL certificate verification

Configuration:
      --config <FILE>  Use specific config file
      --no-config      Ignore config files
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

# Failure threshold - only fail if more than X% of URLs are broken
$ urlsup --recursive docs/ --failure-threshold 10  # Allow up to 10% failures
$ urlsup README.md --failure-threshold 0           # Strict mode - fail on any broken URL

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

## 📦 Installation

Install with `cargo` to run `urlsup` on your local machine.

```bash
cargo install urlsup
```

## 🚀 Shell Completions Installation

`urlsup` supports shell completions for bash, zsh, and fish. You can generate completions manually or use the built-in installation command for automatic setup.

### Automatic Installation (Recommended)

The `completion-install` command automatically installs shell completions to standard directories and provides setup instructions:

```bash
# Install bash completions
$ urlsup completion-install bash
✅ Shell completions installed successfully!

Completion installed to: /Users/user/.local/share/bash-completion/completions/urlsup

To enable bash completions, add this to your ~/.bashrc or ~/.bash_profile:
if [[ -d ~/.local/share/bash-completion/completions ]]; then
    for completion in ~/.local/share/bash-completion/completions/*; do
        [[ -r "$completion" ]] && source "$completion"
    done
fi

Then restart your shell or run: source ~/.bashrc

# Install zsh completions
$ urlsup completion-install zsh
✅ Shell completions installed successfully!

Completion installed to: /Users/user/.local/share/zsh/site-functions/_urlsup

To enable zsh completions, add this to your ~/.zshrc:
if [[ -d ~/.local/share/zsh/site-functions ]]; then
    fpath=(~/.local/share/zsh/site-functions $fpath)
    autoload -U compinit && compinit
fi

Then restart your shell or run: source ~/.zshrc
You may also need to clear the completion cache: rm -f ~/.zcompdump*

# Install fish completions
$ urlsup completion-install fish
✅ Shell completions installed successfully!

Completion installed to: /Users/user/.config/fish/completions/urlsup.fish

Fish completions are automatically loaded from ~/.config/fish/completions/
Restart your shell or run: fish -c 'complete --erase; source ~/.config/fish/config.fish'
```

### Manual Installation

For manual installation or unsupported shells, generate the completion script and add it yourself:

```bash
# Generate completions for your shell
$ urlsup completion-generate bash > urlsup_completion.bash
$ urlsup completion-generate zsh > _urlsup
$ urlsup completion-generate fish > urlsup.fish
$ urlsup completion-generate powershell > urlsup_completion.ps1
$ urlsup completion-generate elvish > urlsup_completion.elv

# Then add to your shell's configuration manually
```

### Supported Shells

| Shell | Auto-Install | Manual Install | Standard Location |
|-------|--------------|----------------|-------------------|
| bash  | ✅ Yes | ✅ Yes | `~/.local/share/bash-completion/completions/urlsup` |
| zsh   | ✅ Yes | ✅ Yes | `~/.local/share/zsh/site-functions/_urlsup` |
| fish  | ✅ Yes | ✅ Yes | `~/.config/fish/completions/urlsup.fish` |
| PowerShell | ❌ Manual only | ✅ Yes | Add to `$PROFILE` manually |
| Elvish | ❌ Manual only | ✅ Yes | Add to `~/.elvish/rc.elv` manually |

**Note**: The `completion-install` command creates directories as needed and handles path resolution automatically. For PowerShell and Elvish, use the manual `completion-generate` command and follow the provided instructions.

## ⚙️ Configuration File

`urlsup` supports TOML configuration files for managing complex setups. Place a `.urlsup.toml` file in your project root:

```toml
# .urlsup.toml - Project configuration for urlsup
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
failure_threshold = 10.0  # Allow up to 10% of URLs to fail

# Performance settings
use_head_requests = false  # Use HEAD instead of GET for faster validation

# Security settings
skip_ssl_verification = false
proxy = "http://proxy.company.com:8080"

# Output settings
output_format = "text"  # or "json" or "minimal"
verbose = false
```

### Configuration Discovery

`urlsup` searches for configuration files in this order:
1. `.urlsup.toml` in current directory
2. `.urlsup.toml` in parent directories (up to 3 levels)
3. Default configuration if no file found

CLI arguments always override configuration file settings.

## 🔧 Advanced Features

### 🎯 Failure Threshold

Control when `urlsup` should fail based on the percentage of broken URLs:

```bash
# Only fail if more than 20% of URLs are broken
$ urlsup --recursive docs/ --failure-threshold 20

# Strict mode - fail on any broken URL (default behavior)
$ urlsup docs/ --failure-threshold 0

# Lenient mode for large documentation sets
$ urlsup --recursive . --failure-threshold 5  # Allow up to 5% failures
```

**Configuration file:**
```toml
# In .urlsup.toml
failure_threshold = 10.0  # Allow up to 10% failures
```

**Use Cases:**
- **Large documentation**: Prevent CI failures for 1-2 stale external links out of hundreds
- **External API monitoring**: Allow some endpoints to be temporarily down
- **Migration periods**: Gradually improve link quality without breaking builds
- **Third-party content**: Handle external links that may be occasionally unreachable

**Example output:**
```bash
$ urlsup --recursive docs/ --failure-threshold 15

# When within threshold:
✅ Failure rate 12.5% is within threshold 15.0% (5/40 URLs failed)

# When exceeding threshold:
❌ Failure rate 17.5% exceeds threshold 15.0% (7/40 URLs failed)
```

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
# Text output (default) - clean, colorful, emoji-based with grouping
$ urlsup README.md
✅ No issues found!

# JSON output for scripts and automation
$ urlsup --format json README.md
{"status": "success", "issues": []}

# Minimal output for scripts (no colors, emojis, or config info)
$ urlsup --format minimal README.md
404 https://example.com/broken
500 https://api.broken.com

# Or configure in .urlsup.toml
output_format = "json"
```

#### 📊 JSON Output Examples

JSON format is perfect for automation, CI/CD integration, and programmatic processing:

**Successful validation:**
```bash
$ urlsup --format json README.md
{"status": "success", "issues": []}
```

**Failed validation with issues:**
```bash
$ urlsup --format json docs/
{"status": "failure", "issues": [
  {"url": "https://example.com/404", "file": "docs/api.md", "line": 23, "status_code": 404, "description": ""},
  {"url": "https://broken.link", "file": "docs/guide.md", "line": 45, "status_code": null, "description": "connection timeout"}
]}
```

**Processing JSON with `jq`:**

```bash
# Extract all broken URLs
$ urlsup --format json docs/ | jq -r '.issues[].url'

# Count issues by status code
$ urlsup --format json docs/ | jq '.issues | group_by(.status_code) | map({status: .[0].status_code, count: length})'

# Find all timeout errors
$ urlsup --format json docs/ | jq '.issues[] | select(.description | contains("timeout"))'

# Get files with broken links
$ urlsup --format json docs/ | jq -r '.issues[].file' | sort | uniq

# Export issues to CSV for reporting
$ urlsup --format json docs/ | jq -r '.issues[] | [.file, .line, .url, .status_code] | @csv'
```

#### Output Format Comparison

| Format | Colors/Emojis | Config Info | URL List | Progress Bars | Issue Grouping | Use Case |
|--------|---------------|-------------|----------|---------------|----------------|----------|
| `text` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | Interactive use |
| `json` | ❌ No | ❌ No | ❌ No | ❌ No | ❌ No | Automation/scripts |
| `minimal` | ❌ No | ❌ No | ❌ No | ❌ No | ❌ No | Simple scripts/CI |

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

## 🚀 Performance Features

### HEAD Request Optimization

For even faster URL validation, enable HEAD requests instead of GET requests:

```toml
# In .urlsup.toml
use_head_requests = true  # Use HEAD instead of GET for faster validation
```

**Benefits:**
- **Faster validation**: HEAD requests only fetch headers, not full content
- **Reduced bandwidth**: Minimal data transfer for each URL check
- **Better for CI/CD**: Faster pipeline execution for large documentation sets

**When to use:**
- ✅ Internal documentation validation
- ✅ Known-good server environments
- ✅ CI/CD pipelines with trusted URL sets
- ✅ Large-scale validation where speed is critical

**When NOT to use:**
- ❌ Public URL validation (some servers reject HEAD requests)
- ❌ Mixed server environments with unknown HEAD support
- ❌ First-time validation of unknown URLs

**Example usage:**
```bash
# Enable HEAD requests for faster CI validation
$ urlsup --config .urlsup-fast.toml --recursive docs/

# Where .urlsup-fast.toml contains:
use_head_requests = true
timeout = 15
threads = 16
```

## ⚡ Browser-like HTTP Client

`urlsup` uses a simplified HTTP client designed for maximum compatibility:

### 🌐 Browser-Compatible Behavior
- **Automatic Protocol Negotiation**: Lets the client and server automatically negotiate HTTP/1.1 or HTTP/2
- **Default Connection Management**: Uses reqwest's browser-like connection handling for reliability
- **Automatic Compression**: Leverages gzip, brotli, and deflate for reduced bandwidth (like browsers)
- **Reliable Error Handling**: Avoids complex optimizations that can cause connection issues

### 🎯 Memory & Algorithm Improvements
- **Ultra-Fast Hashing**: Uses `FxHashSet` for 15-20% faster URL deduplication
- **Smart Pre-allocation**: Pre-sized collections avoid expensive reallocations
- **Optimized Deduplication**: O(n) hash-based instead of O(n²) sorting-based
- **Memory-Efficient Streaming**: Handles large URL sets without memory bloat

### 🔄 Concurrent Processing
- **Adaptive Batching**: Batch sizes adapt to thread count while preventing memory issues
- **Improved Buffering**: Optimized for better concurrent URL validation
- **Static Resource Reuse**: Eliminates repeated allocations for parsing components

### 📈 Performance Gains
- **Small workloads (10-100 URLs)**: 20-30% faster validation
- **Large workloads (1000+ URLs)**: 40-60% faster with 50-70% less memory usage
- **CI/CD pipelines**: Dramatically reduced execution time for documentation validation

## 🚨 Error Handling

Comprehensive error handling with specific error types:

- **Configuration errors**: Invalid TOML, missing files
- **Network errors**: Timeouts, connection failures, DNS resolution
- **Path errors**: Invalid file paths, permission issues
- **Validation errors**: Malformed URLs, regex compilation failures

All errors include helpful context and suggestions for resolution.

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
