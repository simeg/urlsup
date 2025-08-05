# urlsup ![CI][build_badge] [![Code Coverage][coverage_badge]][coverage_report]

`urlsup` (_urls up_) finds URLs in files and checks whether they are up by
making a `GET` request and checking the response status code. This tool is
useful for lists, repos or any type of project containing URLs that you want to
be up.

It's written in Rust (stable) and executes the requests async in multiple
threads, making it very fast. This in combination with its ease of use makes
it the perfect tool for your CI pipeline.

This project is a slim version of
[`awesome_bot`](https://github.com/dkhamsing/awesome_bot) but aims to be faster.

## Usage
```bash
USAGE:
    urlsup [OPTIONS] <FILES>...

ARGUMENTS:
    <FILES>...    Files or directories to check

OPTIONS:
    -w, --white-list <urls>        Comma separated URLs to white list
    -t, --timeout <seconds>        Connection timeout in seconds (default: 30)
    -a, --allow <status codes>     Comma separated status code errors to allow
        --threads <thread count>   Thread count for making requests (default: CPU core count)
        --allow-timeout            URLs that time out are allowed
    -r, --recursive                Recursively process directories
        --file-types <extensions>  Comma separated file extensions to process (e.g., md,html,txt)
    -h, --help                     Print help
    -V, --version                  Print version
```

## Examples

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
$ urlsup --recursive --file-types md,txt docs/

# ✅ Process current directory recursively
$ urlsup --recursive .
```

### File Type Filtering
```bash
# Only check markdown and text files
$ urlsup --recursive --file-types md,txt .

# Only check web files
$ urlsup --recursive --file-types html,css,js website/

# Multiple extensions
$ urlsup --recursive --file-types md,rst,txt docs/
```

### Advanced Options
```bash
# Allow specific status codes
$ urlsup README.md --allow 403,429

# Set timeout and allow timeouts
$ urlsup README.md --allow-timeout -t 5

# Whitelist URLs (partial matches)
$ urlsup README.md --white-list rust,crates

# Combine recursive with filtering and options
$ urlsup --recursive --file-types md --allow 403 --timeout 10 docs/
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

## Installation

Install with `cargo` to run `urlsup` on your local machine.

```bash
cargo install urlsup
```

## GitHub Actions

See [`urlsup-action`](https://github.com/simeg/urlsup-action).

## Development

This repo uses a Makefile as an interface for common operations.

1) Do code changes
2) Run `make build link` to build the project and create a symlink from the built binary to the root
   of the project
3) Run `./urlsup` to execute the binary with your changes
4) Profit :star:

[build_badge]: https://github.com/simeg/urlsup/workflows/CI/badge.svg
[coverage_badge]: https://codecov.io/gh/simeg/urlsup/branch/master/graph/badge.svg?token=2bsQKkD1zg
[coverage_report]: https://codecov.io/gh/simeg/urlsup/branch/master
