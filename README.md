# urlsup

`urlsup` (_urls up_) finds URLs in files and checks whether they are up by making a `GET` request and checking the response status code. This tool is useful for lists, repos or any type of project containing URLs that you want to be up.

It's written in Rust and executes the requests async in multiple threads, making it very fast. This in combination with its ease of use makes it the perfect tool for your CI pipeline.

## Usage
```bash
USAGE:
    urlsup [OPTIONS] <FILES>...

OPTIONS:
    -a, --allow <status codes>      Comma separated status code errors to allow
        --threads <thread count>    Thread count for making requests (default: CPU core count)
    -t, --timeout <seconds>         Connection timeout (default: 30)
    -w, --white-list <urls>         Comma separated URLs to white list

ARGS:
    <FILES>...    Files to check
```

## Examples
```bash
$ urlsup `find . -name "*.md"`
⠹ Finding URLs in files...
Found 2 unique URLs, 3 in total
   1. https://httpstat.us/401
   2. https://httpstat.us/404

⠏ Checking URLs...

> Issues
   1. 401 https://httpstat.us/401
   2. 404 https://httpstat.us/404

$ echo $?
1
```

```bash
$ urlsup `find . -name "*.md"`
⠹ Finding URLs in files...
Found 1 unique URLs, 1 in total
   1. https://httpstat.us/200

⠏ Checking URLs...

No issues!

$ echo $?
0
```

**Allow 404 status code**
```bash
$ urlsup `find . -name "*.md"` --allow 404
⠹ Finding URLs in files...

Allowing status codes
   1. 404

Found 2 unique URLs, 2 in total
   1. https://httpstat.us/401
   2. https://httpstat.us/404

⠏ Checking URLs...

> Issues
   1. 401 https://httpstat.us/401

$ echo $?
1
```

## Development

This repo uses a Makefile as an interface for common operations.

1) Do code changes
2) Run `make build link` to build the project and create a symlink from the built binary to the root
   of the project
3) Run `./urlsup` to execute the binary with your changes
4) Profit :star:
