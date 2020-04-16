extern crate link_auditor;
#[macro_use]
extern crate clap;
extern crate futures;
extern crate grep;
extern crate linkify;
extern crate num_cpus;
extern crate reqwest;
extern crate spinners;

use clap::{App, Arg};
use link_auditor::{Auditor, AuditorOptions};
use std::path::Path;

static OPT_FILES: &str = "FILES";
static OPT_WHITE_LIST: &str = "white-list";
static OPT_TIMEOUT: &str = "timeout";
static OPT_ALLOW: &str = "allow";

#[tokio::main]
async fn main() {
    let opt_word = Arg::with_name(OPT_FILES)
        .help("Files to check")
        //        .validator_os(exists_on_filesystem)
        .multiple(true)
        .required(true)
        .index(1);

    let opt_white_list = Arg::with_name(OPT_WHITE_LIST)
        .help("Comma separated URLs to white list")
        .short("w")
        .long(OPT_WHITE_LIST)
        .value_name("urls")
        .takes_value(true)
        .required(false);

    let opt_timeout = Arg::with_name(OPT_TIMEOUT)
        .help("Connection timeout (default: 30)")
        .short("t")
        .long(OPT_TIMEOUT)
        .value_name("seconds")
        .takes_value(true)
        .required(false);

    let opt_allow = Arg::with_name(OPT_ALLOW)
        .help("Comma separated status code errors to allow")
        .short("a")
        .long(OPT_ALLOW)
        .value_name("status codes")
        .takes_value(true)
        .required(false);

    let matches = App::new("link_auditor")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(opt_word)
        .arg(opt_white_list)
        .arg(opt_timeout)
        .arg(opt_allow)
        .get_matches();

    let auditor = Auditor {};
    let mut opts = AuditorOptions {
        white_list: None,
        timeout: None,
        allowed_status_codes: None,
    };

    if let Some(white_list_urls) = matches.value_of(OPT_WHITE_LIST) {
        let white_list: Vec<String> = white_list_urls
            .split(",")
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();
        opts.white_list = Some(white_list);
    }

    if let Some(str_timeout) = matches.value_of(OPT_TIMEOUT) {
        let timeout: u64 = str_timeout
            .parse()
            .expect(format!("Could not parse {} into an int (u64)", str_timeout).as_str());
        opts.timeout = Some(timeout);
    }

    if let Some(allowed_status_codes) = matches.value_of(OPT_ALLOW) {
        let allowed: Vec<u16> = allowed_status_codes
            .split(",")
            .filter(|s| !s.is_empty())
            .map(|a| a.parse::<u16>())
            .map(|a| a.expect("Could not parse status code to int (u16)"))
            .collect();
        opts.allowed_status_codes = Some(allowed);
    }

    if let Some(files) = matches.values_of(OPT_FILES) {
        let paths: Vec<&Path> = files.map(Path::new).collect::<Vec<&Path>>();

        auditor.check(paths, opts).await;
    }
}

//fn exists_on_filesystem(path: &OsStr) -> Result<(), OsString> {
//    match path.to_str() {
//        None => Err(OsString::from("Could not convert input file path -> &str")),
//        Some(p) => {
//            if Path::new(p).exists() {
//                return Ok(());
//            }
//            Err(OsString::from(format!(
//                "File not found [{}]",
//                path.to_str().unwrap()
//            )))
//        }
//    }
//}
