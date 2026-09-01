# urlsup ![CI][build_badge] [![Code Coverage][coverage_badge]][coverage_report]

`urlsup` (_urls up_) finds URLs in files and checks whether they are up by
making a `GET` request and checking the response status code. This tool is
useful for lists, repos or any type of project containing URLs that you want to
be up.

It's written in Rust (stable) and executes the requests async in multiple
threads, making it _very_ fast. This in combination with its ease of use makes
it the perfect tool for your CI pipeline.

Use the GitHub Action [`urlsup-action`](https://github.com/simeg/urlsup-action)
to quickly get your CI pipeline up and running.

Using [`awesome_bot`](https://github.com/dkhamsing/awesome_bot) today? Here's a [migration guide](docs/MIGRATION_AWESOME_BOT.md).

<img src="banner.png" alt="Dotfiles Banner" width="100%" style="display: block; margin: 0 auto;">

## 📚 Table of Contents

- [📦 Installation](#-installation)
- [🚀 Usage](#-usage)
- [📝 Examples](#-examples)
- [🚦 Exit Codes](#-exit-codes)
- [⚙️ Configuration File](#-configuration-file)
  - [Configuration Discovery](#configuration-discovery)
  - [🧙‍♂️ Configuration Wizard](#-configuration-wizard)
- [🔧 Advanced Features](#-advanced-features)
  - [🎯 Failure Threshold](#-failure-threshold)
  - [📤 Output Formats](#-output-formats)
  - [🎨 Color Output](#-color-output)
  - [📈 Performance Analysis](#-performance-analysis)
  - [📊 HTML Dashboard](#-html-dashboard)
  - [🚀 HEAD Request Optimization](#-head-request-optimization)
- [🐚 Shell Completions](#-shell-completions)
- [🔄 GitHub Actions](#-github-actions)
- [🛠️ Development](#-development)

## 📦 Installation

Install with `cargo` to run `urlsup` on your local machine.

```bash
cargo install urlsup
```

## 🚀 Usage
```bash
urlsup - CLI to validate URLs in files [version 2.4.0]

Usage: urlsup [OPTIONS] [FILES]... [COMMAND]

Example:

	$ urlsup . --recursive --include md,txt

Commands:
  completion-generate  Generate shell completions
  completion-install   Install shell completions to standard location
  config-wizard        Run interactive configuration wizard
  help                 Print this message or the help of the given subcommand(s)

Arguments:
  [FILES]...  Files or directories to check

Options:
  -h, --help     Print help
  -V, --version  Print version

Core Options:
  -r, --recursive            Recursively process directories. Will skip files/directories listed in .gitignore
  -t, --timeout <SECONDS>    Connection timeout in seconds (default: 5)
      --concurrency <COUNT>  Concurrent requests (default: CPU cores)

Filtering & Content:
      --include <EXTENSIONS>     File extensions to process (e.g., md,html,txt)
      --allowlist <URLS>         Hosts or URL prefixes to allow (comma-separated)
      --allow-status <CODES>     Status codes to allow (comma-separated)
      --exclude-pattern <REGEX>  URL patterns to exclude (regex)
      --no-ignore                Do not respect .gitignore/.ignore files when recursing

Retry & Rate Limiting:
      --retry <COUNT>                Retry attempts for failed requests (default: 0)
      --retry-delay <MS>             Delay between retries in ms (default: 1000)
      --rate-limit <MS>              Delay between requests in ms (default: 0)
      --allow-timeout                Allow URLs that timeout
      --no-allow-timeout             Do not allow URLs that timeout (overrides config file)
      --failure-threshold <PERCENT>  Fail only if more than X% URLs are broken (0-100)

Output & Verbosity:
  -q, --quiet            Suppress progress output
  -v, --verbose          Enable verbose logging
      --format <FORMAT>  Output format [default: text] [possible values: text, json, minimal]
      --no-progress      Disable progress bars

Network & Security:
      --user-agent <AGENT>  Custom User-Agent header
      --proxy <URL>         HTTP/HTTPS proxy URL
      --insecure            Skip SSL certificate verification
      --no-insecure         Enforce SSL certificate verification (overrides config file)

Configuration:
      --config <FILE>  Use specific config file
      --no-config      Ignore config files

Performance Analysis:
      --show-performance       Show memory usage and optimization suggestions
      --no-show-performance    Do not show memory usage and optimization suggestions (overrides config file)
      --html-dashboard <PATH>  Generate HTML dashboard report
```

Upgrading from v1.x? Four flags were renamed: `--white-list` → `--allowlist`,
`--allow` → `--allow-status`, `--threads` → `--concurrency`, and `--file-types`
→ `--include`.

## 📝 Examples

**Files vs. directories**: files are processed directly; directories require
`--recursive`, which also skips anything listed in `.gitignore` (build
artifacts, `node_modules/`, `target/`, and so on). `--no-ignore` disables that
skipping.

```bash
# Single file, multiple files, globs
$ urlsup README.md
$ urlsup README.md CHANGELOG.md
$ urlsup docs/*.md

# Directories need --recursive
$ urlsup docs/
error: 'docs/' is a directory. Use --recursive to process directories.
$ urlsup --recursive docs/

# Restrict to certain file extensions
$ urlsup --recursive --include md,txt .

# Allow specific status codes, and allow timeouts
$ urlsup README.md --allow-status 403,429
$ urlsup README.md --allow-timeout --timeout 10

# Allowlist a host (plus its subdomains), or a URL prefix (plus anything under it)
$ urlsup README.md --allowlist example.com,docs.rs
$ urlsup README.md --allowlist https://example.com/docs

# Exclude URLs by regex
$ urlsup --exclude-pattern ".*\.local$" --exclude-pattern "^http://localhost.*" docs/

# Tolerate up to 10% broken URLs
$ urlsup --recursive docs/ --failure-threshold 10
```

**In CI**, use a machine-readable format and let the exit code decide the build:

```bash
urlsup --recursive --include md . --format minimal --no-progress
```

## 🚦 Exit Codes

`urlsup` is designed for CI, so the exit status is the contract:

| Code | Meaning |
|------|---------|
| `0`  | No broken URLs (or failures stayed within `--failure-threshold`) |
| `1`  | Broken URLs found, or a fatal error (bad config, unreadable input, invalid proxy) |
| `2`  | Invalid command-line usage (reported by the argument parser) |

A file that cannot be read is reported as a warning on stderr and skipped; the
remaining files are still checked.

Machine-readable output (`--format json`, `--format minimal`) goes to stdout.
Progress bars, warnings, and the `--show-performance` summary go to stderr, so
piping stdout stays parseable:

```bash
urlsup --recursive docs/ --format json > report.json
```

## ⚙️ Configuration File

`urlsup` reads TOML configuration from a `.urlsup.toml` file. Every setting is
optional; the file below lists all supported keys with their defaults.

```toml
# .urlsup.toml
timeout = 5                    # Connection timeout in seconds
threads = 8                    # Concurrent requests (maps to --concurrency)
allow_timeout = false          # Treat timeouts as OK
file_types = ["md", "html", "txt"]  # Extensions to process (maps to --include)
no_ignore = false              # Ignore .gitignore/.ignore when recursing

# Filtering
exclude_patterns = [           # Regex patterns of URLs to skip
    "^https://example\\.com/private/.*",
    ".*\\.local$",
    "^http://localhost.*",
]
allowlist = [                  # Hosts (incl. subdomains) or URL prefixes to allow
    "https://api.github.com",
    "https://docs.rs",
]
allowed_status_codes = [403, 429]   # Status codes to treat as OK
failure_threshold = 10.0       # Fail only if more than X% of URLs are broken

# Network & retries
user_agent = "MyBot/1.0"
retry_attempts = 3
retry_delay = 1000             # Milliseconds between retries
rate_limit_delay = 100         # Minimum milliseconds between requests
proxy = "http://user:pass@proxy.company.com:8080"
skip_ssl_verification = false  # ⚠️ Only for trusted internal environments

# Performance
use_head_requests = false      # HEAD instead of GET (faster, less compatible)
show_performance = false       # Memory usage and optimization suggestions
html_dashboard_path = "report.html"

# Output
output_format = "text"         # "text", "json" or "minimal"
verbose = false
```

### Configuration Discovery

`urlsup` searches for a config file in this order, using the first one it finds:

1. `.urlsup.toml` in the current directory
2. `.urlsup.toml` in parent directories, up to 3 levels up
3. Built-in defaults if no file is found

`--config <FILE>` forces a specific file and `--no-config` ignores config files
entirely. CLI arguments always override config file settings; the `--no-*`
flags (`--no-allow-timeout`, `--no-insecure`, `--no-show-performance`) exist so
you can turn a config file setting back off from the command line.

### 🧙‍♂️ Configuration Wizard

`urlsup config-wizard` runs an interactive wizard that writes a commented
`.urlsup.toml` for you, with defaults tuned to your project type (documentation
site, GitHub repo, blog, API docs, wiki, CI pipeline, or a fully custom setup).

## 🔧 Advanced Features

### 🎯 Failure Threshold

`--failure-threshold <PERCENT>` makes `urlsup` exit `0` as long as no more than
that percentage of URLs are broken — useful for large documentation sets where
one or two stale external links shouldn't break the build. The default is `0`,
meaning any broken URL fails.

```bash
$ urlsup --recursive docs/ --failure-threshold 15
✅ Failure rate 12.5% is within threshold 15.0% (5/40 URLs failed)
❌ Failure rate 17.5% exceeds threshold 15.0% (7/40 URLs failed)
```

### 📤 Output Formats

| Format    | Description                                                              | Use case           |
|-----------|--------------------------------------------------------------------------|--------------------|
| `text`    | Default. Colors, emoji, config summary, progress bars, grouped issues.   | Interactive use    |
| `json`    | Single JSON object on stdout. No colors, config info or progress.        | Automation/scripts |
| `minimal` | One `<status> <url>` line per issue. No colors, config info or grouping. | Simple scripts/CI  |

```bash
$ urlsup --format json README.md
{"status": "success", "issues": []}

$ urlsup --format json docs/
{"status": "failure", "issues": [
  {"url": "https://example.com/404", "file": "docs/api.md", "line": 23, "status_code": 404, "description": ""},
  {"url": "https://broken.link", "file": "docs/guide.md", "line": 45, "status_code": null, "description": "connection timeout"}
]}

$ urlsup --format minimal README.md
404 https://example.com/broken
500 https://api.broken.com
```

Post-process the JSON with `jq`:

```bash
# All broken URLs
$ urlsup --format json docs/ | jq -r '.issues[].url'

# Issues grouped by status code
$ urlsup --format json docs/ | jq '.issues | group_by(.status_code) | map({status: .[0].status_code, count: length})'

# Export to CSV
$ urlsup --format json docs/ | jq -r '.issues[] | [.file, .line, .url, .status_code] | @csv'
```

### 🎨 Color Output

Colors and formatting are enabled when writing to a terminal that supports them.

- `NO_COLOR=1` (or `FORCE_COLOR=0`) disables all color output
- `FORCE_COLOR=1` enables color even when not writing to a terminal
- Output piped to a file or another process is plain text automatically
- `--format minimal` emits plain text with no colors or grouping

### 📈 Performance Analysis

`--show-performance` prints a timing, memory and CPU breakdown to stderr, along
with optimization suggestions:

```
⚡ Performance Analysis
Total execution time: 2.34s
Peak memory usage: 45.2 MB
Average CPU usage: 23.4%

📊 Operation Breakdown:
• File processing: 0.12s (156 files)
• URL discovery: 0.89s (1,247 URLs found)
• URL validation: 1.33s (987 unique URLs validated)

💡 Optimization Suggestions:
• Consider using --concurrency 8 for better performance
• Enable HEAD requests for faster validation (use_head_requests = true)
• Add .gitignore patterns to reduce file processing overhead
```

### 📊 HTML Dashboard

`--html-dashboard <PATH>` writes a self-contained HTML report with charts of the
validation results, a timing and resource breakdown (when combined with
`--show-performance`), per-file issue listings, and a configuration summary.

```yaml
# .github/workflows/urls.yml
- name: Validate URLs and generate report
  run: urlsup --html-dashboard validation-report.html --show-performance docs/

- name: Upload report artifact
  uses: actions/upload-artifact@v4
  with:
    name: url-validation-report
    path: validation-report.html
```

### 🚀 HEAD Request Optimization

Setting `use_head_requests = true` in `.urlsup.toml` sends `HEAD` instead of
`GET`, which only fetches headers and is noticeably faster on large URL sets.
Use it for internal docs and trusted CI URL sets; avoid it for public URL
validation, since some servers reject `HEAD` requests and will look broken.

## 🐚 Shell Completions

`urlsup completion-install <SHELL>` installs completions for `bash`, `zsh` or
`fish` into the standard location for that shell and prints the snippet you need
to add to your shell config.

```bash
$ urlsup completion-install zsh
✅ Shell completions installed successfully!
Completion installed to: ~/.local/share/zsh/site-functions/_urlsup
```

| Shell      | Auto-install | Standard location                                   |
|------------|--------------|-----------------------------------------------------|
| bash       | ✅ Yes        | `~/.local/share/bash-completion/completions/urlsup` |
| zsh        | ✅ Yes        | `~/.local/share/zsh/site-functions/_urlsup`         |
| fish       | ✅ Yes        | `~/.config/fish/completions/urlsup.fish`            |
| PowerShell | ❌ Manual     | Add to `$PROFILE` manually                          |
| Elvish     | ❌ Manual     | Add to `~/.elvish/rc.elv` manually                  |

For PowerShell, Elvish, or any unsupported shell, write the script yourself with
`urlsup completion-generate <SHELL>`:

```bash
$ urlsup completion-generate powershell > urlsup_completion.ps1
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
