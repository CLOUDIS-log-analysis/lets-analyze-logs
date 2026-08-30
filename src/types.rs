use std::fs::File;

use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug)]

pub struct Cli {
    pub log_path: String,
    pub src_path: String,

    #[arg(short, long, default_value = "5")]
    pub gap: usize,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file_path: String,
    pub line_nr: usize,
}

#[derive(Debug, Clone)]
pub struct StartingLocation {
    pub loc: SourceLocation,
    pub reliability: f64,
}

#[derive(Debug, Clone)]
pub struct BugLocation {
    pub loc: SourceLocation,
    pub reliability: f64,
}

pub struct Ctxt {
    pub log_path: String,
    pub src_path: String,
    pub gap: usize,
}

pub struct Log {
    pub lines: Vec<String>,
}

impl Log {
    pub fn try_new(path: &str) -> anyhow::Result<Self> {
        let mut log_file = File::open(&path).context(format!("{}: not found", path))?;

        let mut str = String::new();
        use std::io::Read;
        log_file.read_to_string(&mut str)?;

        let lines = str
            .split_terminator("\n")
            .map(|str| str.to_owned())
            .collect::<Vec<_>>();
        let lines = lines.into_iter().rev().collect::<Vec<String>>();
        Ok(Log { lines })
    }

    pub fn get_line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|x| x.as_str())
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(|x| x.as_str())
    }
}
