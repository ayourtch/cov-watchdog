#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
use clap::Clap;
use env_logger;
use serde::{Deserialize, Serialize};
#[macro_use]
extern crate lazy_static;
use regex::Regex;

/// read the json data from Coverity and do something with it
#[clap(version = env!("GIT_VERSION"), author = "Andrew Yourtchenko <ayourtch@gmail.com>")]
#[derive(Clap, Debug, Serialize, Deserialize)]
struct Opts {
    /// Input JSON file name saved from a URL similar to https://scan9.coverity.com/api/viewContents/issues/v1/28863?projectId=12999&rowCount=-1
    #[clap(short, long)]
    in_file: String,

    /// MAINTAINERS file name
    #[clap(short, long)]
    maintainers_file: String,

    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CovRecord {
    cid: u64,
    displayType: String,
    displayImpact: String,
    status: String,
    firstDetected: String,
    classification: String,
    owner: String,
    severity: String,
    action: String,
    displayComponent: String,
    displayCategory: String,
    displayFile: String,
    displayFunction: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CovColumn {
    name: String,
    label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ViewContentsV1 {
    offset: usize,
    totalRows: usize,
    columns: Vec<CovColumn>,
    rows: Vec<CovRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CovReport {
    #[serde(rename = "viewContentsV1")]
    v1: ViewContentsV1,
}

#[derive(Clone, Debug)]
struct Maintainer {
    id: String,
}

#[derive(Clone, Debug)]
enum FilePattern {
    Include(String),
    Exclude(String),
}

#[derive(Clone, Debug)]
struct MaintainerEntry {
    title: String,
    single_word_name: String,
    maintainers: Vec<Maintainer>,
    files: Vec<FilePattern>,
    comments: Vec<String>,
    feature_yaml: String,
}

#[derive(Clone, Debug)]
struct MaintainerFile {
    preamble: Vec<String>,
    entries: Vec<MaintainerEntry>,
}

fn read_maintainer_file(fname: &str) -> MaintainerFile {
    lazy_static! {
        static ref REentry: Regex =
            Regex::new(r"(?x)^(?P<type>[MFECIY]+):\s+(?P<text>.*)$").unwrap();
        static ref REempty: Regex = Regex::new(r"(?x)^\s*$").unwrap();
        static ref REspace_or_empty: Regex = Regex::new(r"^\s+").unwrap();
        static ref REtitle: Regex = Regex::new(r"^\S+").unwrap();
    };
    /*
     */
    let mut next_title = "".to_string();
    let mut entry: Option<MaintainerEntry> = None;
    let mut in_intro: bool = true;
    let mut mfile = MaintainerFile {
        preamble: vec![],
        entries: vec![],
    };

    let data = std::fs::read_to_string(&fname).unwrap();
    for line in data.lines() {
        if REentry.is_match(&line) {
            log::debug!("E: {}", &line);
            if let Some(e) = &mut entry {
                for cap in REentry.captures_iter(line) {
                    match cap.name("type").unwrap().as_str() {
                        "M" => e.maintainers.push(Maintainer {
                            id: cap.name("text").unwrap().as_str().to_string(),
                        }),
                        "F" => e.files.push(FilePattern::Include(
                            cap.name("text").unwrap().as_str().to_string(),
                        )),
                        "E" => e.files.push(FilePattern::Exclude(
                            cap.name("text").unwrap().as_str().to_string(),
                        )),
                        "C" => e
                            .comments
                            .push(cap.name("text").unwrap().as_str().to_string()),
                        "I" => e.single_word_name = cap.name("text").unwrap().as_str().to_string(),
                        "Y" => e.feature_yaml = cap.name("text").unwrap().as_str().to_string(),
                        x => {
                            panic!("Type {} is not handled!", x);
                        }
                    }
                }
            }
        } else if REtitle.is_match(&line) {
            log::debug!("T: {}", &line);
            if in_intro {
                if next_title == "".to_string() {
                    next_title = line.to_string().clone();
                    mfile.preamble.push(line.to_string());
                } else {
                    entry = Some(MaintainerEntry {
                        title: line.to_string(),
                        single_word_name: "".to_string(),
                        maintainers: vec![],
                        files: vec![],
                        comments: vec![],
                        feature_yaml: "".to_string(),
                    });
                    in_intro = false;
                }
            } else {
                mfile.entries.push(entry.unwrap());
                entry = Some(MaintainerEntry {
                    title: line.to_string(),
                    single_word_name: "".to_string(),
                    maintainers: vec![],
                    files: vec![],
                    comments: vec![],
                    feature_yaml: "".to_string(),
                });
            }
        } else {
            log::debug!("L: {}", &line);
            if in_intro {
                mfile.preamble.push(line.to_string());
            }
        }
    }
    mfile
}

fn is_wildcard(fpc: &std::path::Component) -> bool {
    return false;
}

fn component_match(fpc: &std::path::Component, fc: &std::path::Component) -> bool {
    if fpc == fc {
        return true;
    }
    return false;
}

/// FIXME - incomplete
fn match_pattern(fp: &str, fname: &str) -> bool {
    use std::path::PathBuf;
    let mut match_subtree = fp.ends_with("/");

    let fp_pb = PathBuf::from(fp);
    let fname_pb = PathBuf::from(fname);

    // account for a bug or feature where the folders don't end with "/"
    if !match_subtree {
        if fp_pb.components().count() > 0 && !is_wildcard(&fp_pb.components().last().unwrap()) {
            match_subtree = true;
        }
    }

    for (f, p) in fname_pb.components().zip(fp_pb.components()) {
        if !component_match(&p, &f) {
            return false;
        }
    }

    if fname_pb.components().count() == fp_pb.components().count() {
        return true;
    }

    if fname_pb.components().count() > fp_pb.components().count() {
        return match_subtree;
    }

    return false;
}

fn match_maintainer_entry(mentry: &MaintainerEntry, fname: &str) -> bool {
    let mut match_include = false;
    let mut match_exclude = false;
    for e in &mentry.files {
        match e {
            FilePattern::Include(fp) => {
                if match_pattern(fp, fname) {
                    match_include = true;
                }
            }
            FilePattern::Exclude(fp) => {
                if match_pattern(fp, fname) {
                    match_exclude = true;
                }
            }
        }
    }
    match_include && !match_exclude
}

fn get_mentry_for_file(mf: &MaintainerFile, fname: &str) -> Vec<MaintainerEntry> {
    let mut meo: Vec<MaintainerEntry> = vec![];
    for e in &mf.entries {
        if match_maintainer_entry(e, fname) {
            meo.push(e.clone());
        }
    }
    meo
}

fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();
    if let Ok(data) = std::fs::read_to_string(&opts.in_file) {
        let cov: CovReport = serde_json::from_str(&data).unwrap();
        if opts.verbose > 2 {
            log::warn!("Cov report: {:#?}", &cov);
        }

        let mf = read_maintainer_file(&opts.maintainers_file);
        if opts.verbose > 2 {
            log::warn!("maintainers file: {:#?}", &mf);
        }

        for bug in cov.v1.rows {
            let fname = bug.displayFile.trim_start_matches("/");
            let mes = get_mentry_for_file(&mf, &fname);
            println!(
                "BUG in function: {}, file: {}",
                &bug.displayFunction, &fname
            );
            // let matches: Vec<String> = mes.iter().map(|x| x.single_word_name.clone()).collect();
            let matches = mes;
            println!("Matches: {:#?}", &matches);
        }
    }
}
