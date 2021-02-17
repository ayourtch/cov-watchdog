#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
use clap::Clap;
use env_logger;
use serde::{Deserialize, Serialize};
#[macro_use]
extern crate lazy_static;
use regex::Regex;
use std::collections::HashMap;

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

    /// Show the bugs that match maintainer(s) (substring search)
    #[clap(short, long)]
    person: Vec<String>,

    /// Show the bugs that match component(s) (exact match)
    #[clap(short, long)]
    component_word: Vec<String>,

    /// Print the list of all maintainers whose components have open issues
    #[clap(short, long)]
    list_emails: bool,

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
    let fpc = fpc.as_os_str().to_str().unwrap();
    return fpc.contains("*") || fpc.contains("[") || fpc.contains("?");
}

fn component_match(fpc: &std::path::Component, fc: &std::path::Component) -> bool {
    use glob::Pattern;
    if fpc == fc {
        return true;
    }
    Pattern::new(fpc.as_os_str().to_str().unwrap())
        .unwrap()
        .matches(fc.as_os_str().to_str().unwrap())
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

fn check_tree_ownership(
    real_root: &str,
    root: &str,
    mf: &MaintainerFile,
    orphans: &mut Vec<String>,
) {
    use std::fs;
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        let metadata = fs::metadata(&path).unwrap();
        if metadata.is_file() {
            let path_str = path.to_str().unwrap().to_string();
            let fname = path_str.trim_start_matches(real_root);
            let mentries = get_mentry_for_file(mf, &fname);
            if mentries.len() == 0 {
                println!("Orphan file {} (root {})", &fname, real_root);
                orphans.push(path_str.clone());
            }
        }
        if metadata.is_dir() && entry.file_name() != "." && entry.file_name() != ".." {
            check_tree_ownership(real_root, &path.to_str().unwrap(), mf, orphans);
        }
    }
}

fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();
    let mf = read_maintainer_file(&opts.maintainers_file);
    if let Ok(data) = std::fs::read_to_string(&opts.in_file) {
        let cov: CovReport = serde_json::from_str(&data).unwrap();
        let mut out_bugs: HashMap<u64, CovRecord> = HashMap::new();
        let mut personal_out_bugs: HashMap<String, HashMap<u64, CovRecord>> = HashMap::new();
        let mut some_query = false;

        if opts.verbose > 2 {
            log::warn!("Cov report: {:#?}", &cov);
        }

        if opts.verbose > 2 {
            log::warn!("maintainers file: {:#?}", &mf);
        }

        for bug in cov.v1.rows {
            let fname = bug.displayFile.trim_start_matches("/");
            let mentries = get_mentry_for_file(&mf, &fname);
            let mut orphan = true;

            for person in &opts.person {
                some_query = true;
                for comp in &mentries {
                    for m in &comp.maintainers {
                        if m.id.contains(person) {
                            out_bugs.insert(bug.cid, bug.clone());
                            orphan = false;
                        }
                    }
                }
            }

            for component_word in &opts.component_word {
                some_query = true;
                for comp in &mentries {
                    if &comp.single_word_name == component_word {
                        out_bugs.insert(bug.cid, bug.clone());
                        orphan = false;
                    }
                }
            }

            /* insert into per-person lists */

            for comp in &mentries {
                for m in &comp.maintainers {
                    personal_out_bugs
                        .entry(m.id.clone())
                        .or_insert(HashMap::new())
                        .insert(bug.cid, bug.clone());
                    orphan = false;
                }
            }

            if orphan {
                personal_out_bugs
                    .entry("Unidentified owner".to_string())
                    .or_insert(HashMap::new())
                    .insert(bug.cid, bug.clone());
            }
        }
        for (_, v) in out_bugs {
            println!(
                "BUG {} in function: {}, file: {}",
                &v.cid, &v.displayFunction, &v.displayFile
            );
        }

        if !some_query {
            /* no other queries specified - show the per-person table */

            let mut all_emails: Vec<String> = vec![];

            for (person, bugs) in personal_out_bugs {
                println!("### {}:", &person);
                for (_, v) in bugs {
                    println!(
                        "  * BUG {} in function: {}, file: {}",
                        &v.cid, &v.displayFunction, &v.displayFile
                    );
                }
                all_emails.push(person);
            }
            if opts.list_emails {
                all_emails.sort();
                all_emails.dedup();
                all_emails.retain(|x| x != "Unidentified owner");
                all_emails.retain(|x| !x.contains("Mailing List"));
                println!("\n\nall emails: {}", all_emails.join("; "));
            }
        }
    } else {
        // assume it is a directory and attempt to traverse it
        let mut orphans: Vec<String> = vec![];
        check_tree_ownership(&opts.in_file, &opts.in_file, &mf, &mut orphans);
    }
}
