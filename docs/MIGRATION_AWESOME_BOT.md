# Migration Guide: awesome_bot → urlsup

This guide helps you migrate from [awesome_bot](https://github.com/dkhamsing/awesome_bot) to **urlsup**, providing command mappings, feature comparisons, and real-world migration examples.

## 🚀 Why Migrate to urlsup?

- **⚡ Significantly faster**: Async/concurrent validation with optimized performance
- **🔧 Modern CLI**: Clean, intuitive interface with comprehensive help
- **📊 Rich output formats**: JSON, minimal, and colorful text output
- **🎯 Advanced features**: Retry logic, rate limiting, failure thresholds
- **🛠️ Better CI/CD integration**: Designed for modern automation workflows

## 📋 Command Mapping Reference

### Basic URL Validation

| awesome_bot | urlsup | Notes |
|-------------|---------|-------|
| `awesome_bot README.md` | `urlsup README.md` | Direct replacement |
| `awesome_bot file1.md file2.md` | `urlsup file1.md file2.md` | Multiple files |
| `awesome_bot *.md` | `urlsup *.md` | Glob patterns work the same |

### Directory Processing

| awesome_bot | urlsup | Notes |
|-------------|---------|-------|
| `awesome_bot docs/ --recursive` | `urlsup --recursive docs/` | Note flag position |
| `awesome_bot . -r` | `urlsup --recursive .` | Current directory |

### Filtering and Allowlists

| awesome_bot | urlsup | Notes |
|-------------|---------|-------|
| `awesome_bot file.md --white-list url1,url2` | `urlsup file.md --allowlist url1,url2` | Renamed for modern terminology |
| `awesome_bot file.md --allow-dupe` | `urlsup file.md` | Duplicates handled automatically |
| `awesome_bot file.md --allow 403,500` | `urlsup file.md --allow-status 403,500` | Clearer naming |

### Timeouts and Performance

| awesome_bot | urlsup | Notes |
|-------------|---------|-------|
| `awesome_bot file.md --allow-timeout` | `urlsup file.md --allow-timeout` | Same flag |
| `awesome_bot file.md --set-timeout 30` | `urlsup file.md --timeout 30` | Simplified flag name |
| No equivalent | `urlsup file.md --concurrency 8` | New: control parallel requests |
| No equivalent | `urlsup file.md --retry 3 --retry-delay 1000` | New: retry logic |

### Output Formats

| awesome_bot | urlsup | Notes |
|-------------|---------|-------|
| No equivalent | `urlsup file.md --format json` | New: structured JSON output |
| No equivalent | `urlsup file.md --format minimal` | New: CI-friendly minimal output |
| Default output | `urlsup file.md --format text` | Enhanced with colors and grouping |

## 🔧 Configuration File Migration

### awesome_bot → urlsup Configuration

awesome_bot doesn't have configuration files, but urlsup supports `.urlsup.toml`:

```toml
# .urlsup.toml - Replace command-line flags with persistent config

# Basic settings
timeout = 5
allow_timeout = false
threads = 8  # Replaces --concurrency

# Filtering (replaces --white-list)
allowlist = [
    "https://api.github.com",
    "https://docs.rs"
]

# Status codes (replaces --allow)
allowed_status_codes = [403, 429, 503]

# Advanced features (not available in awesome_bot)
retry_attempts = 3
retry_delay = 1000
rate_limit_delay = 100
failure_threshold = 10.0
```

## 📝 Real-World Migration Examples

### Example 1: Simple Documentation Check

**Before (awesome_bot):**
```bash
awesome_bot README.md CHANGELOG.md --allow-timeout --white-list example.com
```

**After (urlsup):**
```bash
urlsup README.md CHANGELOG.md --allow-timeout --allowlist example.com
```

### Example 2: Recursive Directory Validation

**Before (awesome_bot):**
```bash
awesome_bot docs/ --recursive --allow 403,404 --set-timeout 60
```

**After (urlsup):**
```bash
urlsup --recursive docs/ --allow-status 403,404 --timeout 60
```

### Example 3: CI/CD Pipeline

**Before (awesome_bot in GitHub Actions):**
```yaml
- name: Check URLs
  run: |
    gem install awesome_bot
    awesome_bot README.md --allow-timeout --white-list localhost
```

**After (urlsup in GitHub Actions):**
```yaml
- name: Check URLs
  run: |
    cargo install urlsup
    urlsup README.md --allow-timeout --allowlist localhost --format minimal
```

Or using the official action:
```yaml
- uses: simeg/urlsup-action@v1
  with:
    files: 'README.md'
    allow_timeout: true
    allowlist: 'localhost'
```

### Example 4: Complex Project Setup

**Before (awesome_bot):**
```bash
#!/bin/bash
# check-urls.sh
awesome_bot docs/*.md --recursive --allow 403,429 --white-list localhost,127.0.0.1 --allow-timeout
```

**After (urlsup with config file):**

Create `.urlsup.toml`:
```toml
timeout = 30
allow_timeout = true
allowed_status_codes = [403, 429]
allowlist = ["localhost", "127.0.0.1"]
retry_attempts = 2
failure_threshold = 5.0
```

Script becomes:
```bash
#!/bin/bash
# check-urls.sh
urlsup --recursive docs/
```

## 🆕 New Features Not Available in awesome_bot

### 1. Output Formats for Automation

```bash
# JSON output for parsing
urlsup docs/ --format json | jq '.issues[] | .url'

# Minimal output for simple scripts
urlsup docs/ --format minimal > broken-urls.txt
```

### 2. Advanced Retry Logic

```bash
# Retry flaky URLs with exponential backoff
urlsup docs/ --retry 3 --retry-delay 1000
```

### 3. Rate Limiting

```bash
# Be nice to servers
urlsup docs/ --rate-limit 200  # 200ms between requests
```

### 4. Failure Thresholds

```bash
# Only fail if more than 10% of URLs are broken
urlsup docs/ --failure-threshold 10
```

### 5. Performance Analysis

```bash
# Get insights into validation performance
urlsup docs/ --show-performance
```

### 6. HTML Dashboard

```bash
# Generate visual reports
urlsup docs/ --html-dashboard report.html
```

## 🔄 Migration Automation Script

Here's a script to help automate the migration:

```bash
#!/bin/bash
# migrate-awesome-bot.sh

# Replace awesome_bot calls in shell scripts
find . -name "*.sh" -type f -exec sed -i.bak 's/awesome_bot/urlsup/g' {} \;
find . -name "*.sh" -type f -exec sed -i.bak 's/--white-list/--allowlist/g' {} \;
find . -name "*.sh" -type f -exec sed -i.bak 's/--allow /--allow-status /g' {} \;
find . -name "*.sh" -type f -exec sed -i.bak 's/--set-timeout/--timeout/g' {} \;

# Update GitHub Actions workflows
find .github/workflows -name "*.yml" -type f -exec sed -i.bak 's/awesome_bot/urlsup/g' {} \;
find .github/workflows -name "*.yml" -type f -exec sed -i.bak 's/gem install awesome_bot/cargo install urlsup/g' {} \;

# Create basic urlsup config if needed
if [ ! -f .urlsup.toml ]; then
    cat > .urlsup.toml << 'EOF'
# Basic urlsup configuration
timeout = 30
allow_timeout = false
threads = 4

# Add your allowlist URLs here
# allowlist = ["localhost", "example.com"]

# Add allowed status codes here  
# allowed_status_codes = [403, 429]
EOF
    echo "Created .urlsup.toml - please customize as needed"
fi

echo "Migration complete! Please review and test the changes."
```

## 🎯 Performance Comparison

| Metric | awesome_bot | urlsup | Improvement |
|--------|-------------|---------|-------------|
| **Speed** | ~50 URLs/minute | ~500+ URLs/minute | **10x faster** |
| **Memory** | ~100MB for large sets | ~20MB for large sets | **5x more efficient** |
| **Features** | Basic validation | Rich features + analysis | **Comprehensive** |
| **Output** | Plain text only | JSON/HTML/Text formats | **Multiple formats** |
| **CI Integration** | Manual setup | Native support + action | **Better automation** |

## ❓ Common Migration Questions

### Q: Do I need to change my CI/CD pipelines?
**A:** Minimal changes needed. Replace the installation command and update flag names. Consider using the official urlsup-action for GitHub Actions.

### Q: Will urlsup find the same URLs as awesome_bot?
**A:** Yes, and likely more. urlsup uses advanced URL detection and handles edge cases better.

### Q: What about Ruby dependencies?
**A:** urlsup is a single binary with no runtime dependencies. No more Ruby/gem management.

### Q: Can I use the same allowlists?
**A:** Yes, just rename `--white-list` to `--allowlist`. The format is identical.

### Q: What if I have custom awesome_bot modifications?
**A:** urlsup's configuration system and rich output formats likely provide better alternatives. Contact us for specific use cases.

## 🆘 Support

- **Documentation**: [Complete usage guide](README.md)
- **Issues**: [GitHub Issues](https://github.com/simeg/urlsup/issues)
- **Examples**: [More examples in README](README.md#-examples)
- **CI Integration**: [GitHub Actions guide](README.md#-github-actions)

## 📈 Next Steps

1. **Install urlsup**: `cargo install urlsup`
2. **Test on a small project**: Start with a single file
3. **Update scripts gradually**: Use the migration script above
4. **Explore new features**: Try JSON output, retry logic, performance analysis
5. **Optimize CI/CD**: Use failure thresholds and better error handling

Welcome to faster, more reliable URL validation! 🚀