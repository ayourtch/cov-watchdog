use clap::Clap;
use env_logger;
use serde::{Deserialize, Serialize};
#[macro_use]
extern crate lazy_static;
use regex::Regex;

/// read the json data from Coverity and do something with it
#[clap(version = "0.1", author = "Andrew Yourtchenko <ayourtch@gmail.com>")]
#[derive(Clap, Debug, Serialize, Deserialize)]
struct Opts {
    /// Input file name
    #[clap(short, long)]
    in_file: Option<String>,

    /// Maintainers file name
    #[clap(short, long)]
    maintainers_file: Option<String>,

    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct CovColumn {
    name: String,
    label: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViewContentsV1 {
    offset: usize,
    totalRows: usize,
    columns: Vec<CovColumn>,
    rows: Vec<CovRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CovReport {
    #[serde(rename = "viewContentsV1")]
    v1: ViewContentsV1,
}

#[derive(Debug)]
struct Maintainer {
    id: String,
}

#[derive(Debug)]
enum FilePattern {
    Include(String),
    Exclude(String),
}

#[derive(Debug)]
struct MaintainerEntry {
    title: String,
    single_word_name: String,
    maintainers: Vec<Maintainer>,
    files: Vec<FilePattern>,
    comments: Vec<String>,
    feature_yaml: String,
}

#[derive(Debug)]
struct MaintainerFile {
    preamble: Vec<String>,
    entries: Vec<MaintainerEntry>,
}

fn read_maintainer_file(fname: &str) {
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
            println!("E: {}", &line);
            if let Some(e) = &mut entry {
                for cap in REentry.captures_iter(line) {
                    match (cap.name("type").unwrap().as_str()) {
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
            println!("T: {}", &line);
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
            println!("L: {}", &line);
            if in_intro {
                mfile.preamble.push(line.to_string());
            }
        }
    }
    println!("MFile: {:#?}", &mfile);
}

fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();
    if let Some(fname) = opts.in_file {
        if let Ok(data) = std::fs::read_to_string(&fname) {
            let d: CovReport = serde_json::from_str(&data).unwrap();
            println!("Hello, world, there are {} defects", d.v1.rows.len());
        }
    }
    if let Some(fname) = opts.maintainers_file {
        read_maintainer_file(&fname);
    }
}
