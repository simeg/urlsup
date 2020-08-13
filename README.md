# urlsup [![Build Status][build_badge]][build_status] [![Code Coverage][coverage_badge]][coverage_report]

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

FLAGS:
        --allow-timeout             URLs that time out are allowed

OPTIONS:
    -a, --allow <status codes>      Comma separated status code errors to allow
        --threads <thread count>    Thread count for making requests (default: CPU core count)
    -t, --timeout <seconds>         Connection timeout in seconds (default: 30)
    -w, --white-list <urls>         Comma separated URLs to white list

ARGS:
    <FILES>...    Files to check
```

## Examples
```bash
$ urlsup `find . -name "*.md"`
> Using threads: 8
> Using timeout (seconds): 30
> Allow timeout: false
> Will check URLs in 1 file
   1. ./README.md

⠹ Finding URLs in files...

> Found 2 unique URLs, 3 in total
   1. https://httpstat.us/401
   2. https://httpstat.us/404

⠏ Checking URLs...

> Issues
   1. 401 https://httpstat.us/401
   2. 404 https://httpstat.us/404
```

```bash
$ urlsup `find . -name "*.md"`
> Using threads: 8
> Using timeout (seconds): 30
> Allow timeout: false
> Will check URLs in 1 file
   1. ./README.md

⠹ Finding URLs in files...

> Found 1 unique URL, 1 in total
   1. https://httpstat.us/200

⠏ Checking URLs...

> No issues!
```

```bash
$ urlsup README.md --white-list rust,crates
# white list all links starting with rust or crates

$ urlsup README.md,README-zh.md
# check links in 2 files

$ urlsup docs/*.md
# check all markdown files in docs/ directory

$ urlsup README.md --allow-timeout -t 5
# speed up validation by setting a timeout of 5 seconds per link request and allowing timeouts

$ urlsup README.md --allow 403,429
# allow status code errors 403 and 429
```

## Installation

Install with `cargo` to run `urlsup` on your local machine.

```bash
cargo install urlsup
```

### Docker
If you do not want to install `urlsup` you can simply use Docker to run it.

```shell
docker run -it --rm -v $PWD:/mnt:ro simeg/urlsup `find . -name "*.md"`
```

## Travis CI

```yaml
services:
  - docker

script:
  - docker run -ti --rm -v $PWD:/mnt:ro simeg/urlsup `find . -name "*.md"`
```

## GitHub Actions

Run `urlsup` as part of your GitHub Actions workflow. Here is an example of a
full workflow using the `urlsup` action.

**Note: You have to specify exactly what files you want to check and pass them
in as an argument, due to a limitation with GitHub Actions.**

```yaml
name: my-workflow
on:
  push:
    branches: [ master ]
  pull_request:
    branches: [ master ]

jobs:
  validate-links:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2

    # This is the part specific to urlsup. Make sure to also run the checkout action before you run urlsup.
    - name: Link Validator
      id: urlsup
      uses: simeg/urlsup@v1
      with:
        files: "README.md OTHER.md"  # Because of limitations with GitHub Actions you _can't_ pass in an argument such as `find . -name "*.md"` here
```

## Development

This repo uses a Makefile as an interface for common operations.

1) Do code changes
2) Run `make build link` to build the project and create a symlink from the built binary to the root
   of the project
3) Run `./urlsup` to execute the binary with your changes
4) Profit :star:

[build_badge]: https://travis-ci.org/simeg/urlsup.svg?branch=master
[build_status]: https://travis-ci.org/simeg/urlsup
[coverage_badge]: https://codecov.io/gh/simeg/urlsup/branch/master/graph/badge.svg?token=2bsQKkD1zg
[coverage_report]: https://codecov.io/gh/simeg/urlsup/branch/master
