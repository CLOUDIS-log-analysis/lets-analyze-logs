use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

mod inferer;
mod parser;
pub mod types;
mod utils;

pub use types::*;

use clap::Parser;

use crate::{
    inferer::dummy::dummy_inferer,
    parser::{mongodb::parse_mongodb_json, postgres::parse_postgres},
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let Cli {
        log_path,
        src_path,
        gap,
    } = Cli::parse();

    let ctx = Ctxt {
        log_path,
        src_path,
        gap,
    };

    let log = Log::try_new(&ctx.log_path)?;

    // Step 1: Parse log and find suspicious code positions
    let starting_locs = parse_log(&ctx, &log);

    // Step 2: Infer root cause of bug using the code positions and generate reports
    let bug_locs = infer_root_of_bug(&ctx, &starting_locs);

    print_reports(&ctx, &bug_locs)?;

    Ok(())
}

fn print_reports(ctx: &Ctxt, bug_locs: &[BugLocation]) -> anyhow::Result<()> {
    for (index, loc) in bug_locs.iter().enumerate() {
        let file = File::open(Path::new(&ctx.src_path).join(&loc.loc.file_path))?;

        let mut msg = String::new();
        use std::fmt::Write;

        writeln!(&mut msg, "line: {}", loc.loc.line_nr)?;

        writeln!(&mut msg, "content: ")?;

        let reader = BufReader::new(file);
        let center_line_nr = loc.loc.line_nr - 1;
        let upper_bound = center_line_nr.saturating_sub(ctx.gap);
        let iter = reader.lines().skip(upper_bound);
        for (index, line) in iter.take(ctx.gap * 2 + 1).enumerate() {
            let line_nr = upper_bound + index;
            write!(&mut msg, "{}: {}", line_nr + 1, line?)?;
            if line_nr == center_line_nr {
                write!(&mut msg, " <<")?;
            }
            writeln!(&mut msg)?;
        }

        println!("-- report {} --", index + 1);
        println!("-- path {} --", loc.loc.file_path);
        println!("-- reliability: {:.2} --", loc.reliability);
        println!("{}", msg);
        println!();
    }
    Ok(())
}

fn parse_log(ctx: &Ctxt, log: &Log) -> Vec<StartingLocation> {
    let mut total_starting_locs = Vec::new();

    let result = parse_mongodb_json(log);
    if let Ok(mut starting_locs) = result {
        total_starting_locs.append(&mut starting_locs);
    }
    let result = parse_postgres(ctx, log);
    if let Ok(mut starting_locs) = result {
        total_starting_locs.append(&mut starting_locs);
    }

    // more...

    total_starting_locs
}

fn infer_root_of_bug(ctx: &Ctxt, sps: &[StartingLocation]) -> Vec<BugLocation> {
    let mut total_reports = Vec::new();

    log::info!("running dummy_inferer()...");
    let result = dummy_inferer(ctx, sps);
    match result {
        Ok(mut reports) => {
            total_reports.append(&mut reports);
        }
        Err(_) => {
            log::info!("failed");
        }
    }

    // more...

    total_reports
}
