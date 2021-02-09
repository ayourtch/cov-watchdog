use serde::{Deserialize, Serialize};
use clap::Clap;
use env_logger;


/// read the json data from Coverity and do something with it
#[clap(version = "0.1", author = "Andrew Yourtchenko <ayourtch@gmail.com>")]
#[derive(Clap, Debug, Serialize, Deserialize)]
struct Opts {
    /// Input file name
    #[clap(short, long)]
    in_file: Option<String>,

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


fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();
    if let Some(fname) = opts.in_file {
         if let Ok(data) = std::fs::read_to_string(&fname) {
             let d: CovReport = serde_json::from_str(&data).unwrap();
             println!("Hello, world, there are {} defects", d.v1.rows.len());
         }
    }
}


