# Changelog

## 2.0.0 - 2025-08-05

### 🎉 Major Version Release - Breaking Changes

This is a major release with significant improvements and modern CLI design. **Breaking changes** require updating command-line usage.

### 🔄 Breaking Changes

#### CLI Flag Renames
- `--white-list` → `--allowlist` (modern, inclusive terminology)
- `--allow` → `--allow-status` (clearer naming)
- `--threads` → `--concurrency` (industry standard terminology)
- `--file-types` → `--include` (shorter, more intuitive)

### ✨ New Features

#### 📄 Configuration System
- **TOML Configuration Files**: Support for `.urlsup.toml` configuration files
- **Automatic Discovery**: Searches current directory and up to 3 parent directories
- **Hierarchy-based Config**: CLI arguments override config file settings
- **Comprehensive Options**: All CLI flags available in config format

#### 📤 Output Formats & Modes
- **JSON Output**: New `--format json` for automation and scripting
- **Quiet Mode**: `--quiet` flag for minimal output
- **Verbose Mode**: `--verbose` flag for detailed logging
- **Clean Text Output**: Modern emoji-based status messages (`✓ No issues found!`)

#### 📊 Progress Reporting
- **Beautiful Progress Bars**: Real-time progress with `indicatif` integration
- **File Processing Progress**: Shows files being processed
- **URL Validation Progress**: Shows URLs being validated with timing
- **Configurable**: Can be disabled with `--no-progress`

#### 🔍 Advanced Filtering
- **URL Exclusion Patterns**: `--exclude-pattern` with regex support
- **Multiple Patterns**: Support for multiple exclusion patterns
- **Compiled Regex**: Efficient pattern matching with error handling

#### 🔄 Retry Logic & Rate Limiting
- **Configurable Retries**: `--retry` with exponential backoff
- **Retry Delay**: `--retry-delay` in milliseconds
- **Rate Limiting**: `--rate-limit` to throttle requests
- **Smart Backoff**: Prevents overwhelming servers

#### 🌐 Network & Security
- **Custom User Agents**: `--user-agent` for custom headers
- **Proxy Support**: `--proxy` for HTTP/HTTPS proxies
- **SSL Control**: `--insecure` to skip SSL verification
- **Connection Pooling**: Reuse connections for better performance

#### ⚙️ Configuration Management
- **Config File Loading**: `--config` to specify custom config file
- **No Config Mode**: `--no-config` to ignore all config files
- **Field Mapping**: Seamless mapping between CLI args and config

### ⚡ Performance Improvements

#### Optimized Core Operations
- **AHashSet Deduplication**: O(1) URL deduplication instead of O(n²) sorting
- **Async Processing**: Enhanced concurrent URL validation
- **Connection Pooling**: Reuse HTTP connections for better performance
- **Smart Caching**: Avoid redundant requests for duplicate URLs
- **Memory Efficiency**: Optimized data structures throughout

### 🚨 Enhanced Error Handling

#### Comprehensive Error Types
- **Custom Error Enum**: `UrlsUpError` with specific error variants
- **Error Context**: Detailed error messages with suggestions
- **Source Chain**: Proper error source tracking
- **Type Safety**: Better error handling throughout codebase

#### Error Categories
- **Configuration Errors**: Invalid TOML, missing files, bad regex
- **Network Errors**: Timeouts, connection failures, DNS issues
- **Path Errors**: Invalid file paths, permission problems
- **Validation Errors**: Malformed URLs, parsing failures

### 🔧 Code Quality & Architecture

#### Modern Rust Practices
- **Rust Edition 2024**: Updated to latest Rust edition
- **Enhanced Dependencies**: Updated to latest stable versions
- **Better Traits**: Improved trait implementations and bounds
- **Code Organization**: Clean module structure with proper separation

#### Testing & Coverage
- **88 Total Tests**: 75 unit tests + 13 CLI integration tests
- **Comprehensive Coverage**: Tests for all new features and edge cases
- **Mock Testing**: HTTP mocking with `mockito` for reliable tests
- **CI/CD Ready**: Full CI pipeline with linting, testing, and formatting

### 📚 Documentation Improvements

#### README Enhancements
- **Modern CLI Examples**: Updated all examples for v2.0 API
- **Configuration Guide**: Comprehensive TOML configuration documentation
- **Migration Guide**: Clear breaking changes documentation
- **Feature Showcase**: Detailed examples of new capabilities
- **Fun Emojis**: Added engaging emojis while maintaining professionalism

#### Code Documentation
- **Inline Documentation**: Comprehensive code comments and docs
- **API Documentation**: Clear function and module documentation
- **Configuration Schema**: Well-documented configuration options

### 🛠️ Development Experience

#### Build & Development
- **Makefile Interface**: Consistent development commands
- **CI Integration**: Comprehensive GitHub Actions workflow
- **Code Formatting**: Consistent code style with `rustfmt`
- **Linting**: Strict clippy rules for code quality

#### Compatibility
- **Backward Config**: Config files use consistent field names
- **Error Messages**: Helpful migration guidance in error output
- **CLI Help**: Updated help text with new flag names

### 🐛 Bug Fixes
- **Whitespace Handling**: Fixed trailing whitespace issues
- **Flag Consistency**: Resolved naming inconsistencies
- **Error Propagation**: Improved error handling and propagation
- **Memory Leaks**: Fixed potential memory issues in concurrent processing

## 1.0.1

* Separated code into modules
* Bumped versions of critical dependencies, hopefully having positive effects
