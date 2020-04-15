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

    let matches = App::new("link_auditor")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(opt_word)
        .arg(opt_white_list)
        .get_matches();

    let auditor = Auditor {};
    let mut opts = AuditorOptions { white_list: None };

    if let Some(white_list_urls) = matches.value_of(OPT_WHITE_LIST) {
        let white_list: Vec<String> = white_list_urls
            .split(",")
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();
        opts.white_list = Some(white_list);
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
