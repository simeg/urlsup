extern crate async_trait;
extern crate clap;
extern crate futures;
extern crate grep;
extern crate linkify;
extern crate num_cpus;
extern crate reqwest;
extern crate spinners;
extern crate term;
extern crate urlsup;

use clap::{Arg, ArgAction, Command};
use urlsup::finder::Finder;
use urlsup::validator::Validator;
use urlsup::{UrlsUp, UrlsUpOptions};

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const OPT_FILES: &str = "FILES";
const OPT_WHITE_LIST: &str = "white-list";
const OPT_TIMEOUT: &str = "timeout";
const OPT_ALLOW: &str = "allow";
const OPT_THREADS: &str = "threads";
const OPT_ALLOW_TIMEOUT: &str = "allow-timeout";
const OPT_RECURSIVE: &str = "recursive";
const OPT_FILE_TYPES: &str = "file-types";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

fn expand_paths(
    input_paths: Vec<&Path>,
    recursive: bool,
    file_types: Option<&HashSet<String>>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut result_paths = Vec::new();

    for path in input_paths {
        if path.is_file() {
            // Check file extension if filtering is enabled
            if let Some(extensions) = file_types {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(ext) {
                        result_paths.push(path.to_path_buf());
                    }
                } else if extensions.contains("") {
                    // Include files without extensions if "" is in the set
                    result_paths.push(path.to_path_buf());
                }
            } else {
                result_paths.push(path.to_path_buf());
            }
        } else if path.is_dir() && recursive {
            let mut builder = ignore::WalkBuilder::new(path);
            builder.hidden(false); // Include hidden files

            for entry in builder.build() {
                let entry = entry?;
                let entry_path = entry.path();

                if entry_path.is_file() {
                    // Check file extension if filtering is enabled
                    if let Some(extensions) = file_types {
                        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                            if extensions.contains(ext) {
                                result_paths.push(entry_path.to_path_buf());
                            }
                        } else if extensions.contains("") {
                            // Include files without extensions if "" is in the set
                            result_paths.push(entry_path.to_path_buf());
                        }
                    } else {
                        result_paths.push(entry_path.to_path_buf());
                    }
                }
            }
        } else if path.is_dir() && !recursive {
            eprintln!(
                "error: '{}' is a directory. Use --recursive to process directories.",
                path.display()
            );
            std::process::exit(2);
        }
    }

    Ok(result_paths)
}

#[tokio::main]
async fn main() {
    let opt_word = Arg::new(OPT_FILES)
        .help("Files to check")
        .action(ArgAction::Append)
        .num_args(1)
        .required(true)
        .index(1);

    let opt_white_list = Arg::new(OPT_WHITE_LIST)
        .help("Comma separated URLs to white list")
        .short('w')
        .long(OPT_WHITE_LIST)
        .value_name("urls")
        .action(ArgAction::Set)
        .required(false);

    let opt_timeout = Arg::new(OPT_TIMEOUT)
        .help("Connection timeout in seconds (default: 30)")
        .short('t')
        .long(OPT_TIMEOUT)
        .value_name("seconds")
        .action(ArgAction::Set)
        .required(false);

    let opt_allow = Arg::new(OPT_ALLOW)
        .help("Comma separated status code errors to allow")
        .short('a')
        .long(OPT_ALLOW)
        .value_name("status codes")
        .action(ArgAction::Set)
        .required(false);

    let opt_threads = Arg::new(OPT_THREADS)
        .help("Thread count for making requests (default: CPU core count)")
        .long(OPT_THREADS)
        .value_name("thread count")
        .action(ArgAction::Set)
        .required(false);

    let opt_allow_timeout = Arg::new(OPT_ALLOW_TIMEOUT)
        .help("URLs that time out are allowed")
        .long(OPT_ALLOW_TIMEOUT)
        .action(ArgAction::SetTrue)
        .num_args(0)
        .required(false);

    let opt_recursive = Arg::new(OPT_RECURSIVE)
        .help("Recursively process directories")
        .short('r')
        .long(OPT_RECURSIVE)
        .action(ArgAction::SetTrue)
        .num_args(0)
        .required(false);

    let opt_file_types = Arg::new(OPT_FILE_TYPES)
        .help("Comma separated file extensions to process (e.g., md,html,txt)")
        .long(OPT_FILE_TYPES)
        .value_name("extensions")
        .action(ArgAction::Set)
        .required(false);

    let matches = Command::new("urlsup")
        .version("1.0.1")
        .author("Simon Egersand <s.egersand@gmail.com>")
        .about("CLI to validate URLs in files")
        .arg(opt_word)
        .arg(opt_white_list)
        .arg(opt_timeout)
        .arg(opt_allow)
        .arg(opt_threads)
        .arg(opt_allow_timeout)
        .arg(opt_recursive)
        .arg(opt_file_types)
        .get_matches();

    let urls_up = UrlsUp::new(Finder::default(), Validator::default());
    let mut opts = UrlsUpOptions {
        white_list: None,
        timeout: DEFAULT_TIMEOUT,
        allowed_status_codes: None,
        thread_count: num_cpus::get(),
        allow_timeout: matches.get_flag(OPT_ALLOW_TIMEOUT),
    };

    if let Some(white_list_urls) = matches.get_one::<String>(OPT_WHITE_LIST) {
        let white_list: Vec<String> = white_list_urls
            .split(',')
            .filter_map(|s| match s.is_empty() {
                true => None,
                false => Some(s.to_string()),
            })
            .collect();
        opts.white_list = Some(white_list);
    }

    if let Some(str_timeout) = matches.get_one::<String>(OPT_TIMEOUT) {
        let timeout: Duration = str_timeout
            .parse()
            .map(Duration::from_secs)
            .unwrap_or_else(|_| {
                eprintln!("Error: Could not parse timeout '{str_timeout}' as a valid number");
                std::process::exit(1);
            });
        opts.timeout = timeout;
    }

    if let Some(allowed_status_codes) = matches.get_one::<String>(OPT_ALLOW) {
        let allowed: Vec<u16> = allowed_status_codes
            .split(',')
            .filter_map(|s| match s.is_empty() {
                true => None,
                false => match s.parse::<u16>() {
                    Ok(code) => Some(code),
                    Err(_) => {
                        eprintln!("Error: Could not parse status code '{s}' as a valid number");
                        std::process::exit(1);
                    }
                },
            })
            .collect();
        opts.allowed_status_codes = Some(allowed);
    }

    if let Some(thread_count) = matches.get_one::<String>(OPT_THREADS) {
        opts.thread_count = thread_count.parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse thread count '{thread_count}' as a valid number");
            std::process::exit(1);
        });
    }

    if let Some(files) = matches.get_many::<String>(OPT_FILES) {
        let input_paths = files.map(Path::new).collect::<Vec<&Path>>();

        // Validate input paths exist
        for path in &input_paths {
            if !path.exists() {
                eprintln!(
                    "error: invalid value '{}' for '<FILES>...': File not found [\"{}\"]\n\nFor more information, try '--help'.",
                    path.display(),
                    path.display()
                );
                std::process::exit(2);
            }
        }

        // Parse file type filter
        let file_types = if let Some(types_str) = matches.get_one::<String>(OPT_FILE_TYPES) {
            let types: HashSet<String> =
                types_str.split(',').map(|s| s.trim().to_string()).collect();
            Some(types)
        } else {
            None
        };

        // Get recursive flag
        let recursive = matches.get_flag(OPT_RECURSIVE);

        // Expand directories to file paths
        let expanded_paths = match expand_paths(input_paths, recursive, file_types.as_ref()) {
            Ok(paths) => paths,
            Err(e) => {
                eprintln!("Error expanding paths: {e}");
                std::process::exit(1);
            }
        };

        if expanded_paths.is_empty() {
            eprintln!("No files found to process");
            std::process::exit(1);
        }

        // Convert PathBuf to &Path for the run method
        let paths: Vec<&Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

        match urls_up.run(paths, opts).await {
            Ok(result) => {
                if result.is_empty() {
                    println!("\n\n> No issues!");
                } else {
                    println!("\n\n> Issues");
                    for (i, validation_result) in result.iter().enumerate() {
                        println!("{:4}. {}", i + 1, validation_result);
                    }

                    std::process::exit(1)
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("No files provided");
        std::process::exit(1);
    }
}
