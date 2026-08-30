use std::{path::Path, sync::LazyLock};

use chrono::{DateTime, FixedOffset, ParseResult};
use regex::Regex;

use crate::{Ctxt, Log, SourceLocation, StartingLocation, utils::find_file_path_from_file_name};

struct PgLogLine {
    _date: DateTime<FixedOffset>,
    _marker: String,
    severity: String,
    msg: String,
}
/// parse postgres log
/// extract file path and line number if log line has '"attr":{"file":"...","line":..., ...}'
pub fn parse_postgres(ctx: &Ctxt, log: &Log) -> anyhow::Result<Vec<StartingLocation>> {
    log::info!("running parse_postgres()...");

    let first_line: &str = log.get_line(0).ok_or(anyhow::Error::msg("no lines"))?;
    let result = parse_postgres_log_line(first_line);

    match result {
        Some(_) => parse(ctx, log),
        None => {
            log::info!("parse_postgres(): failed");
            log::debug!("{}", first_line);
            Ok(vec![])
        }
    }
}

fn parse(ctx: &Ctxt, log: &Log) -> anyhow::Result<Vec<StartingLocation>> {
    let mut starting_points = Vec::new();

    let log_lines = log.iter();
    for (index, line) in log_lines.enumerate() {
        log::trace!("{}", line);
        let line = parse_postgres_log_line(line);
        match line {
            Some(line) => {
                if line.severity == "PANIC" {
                    let detail = log.get_line(index - 1);
                    match detail {
                        Some(detail) => {
                            let detail = parse_postgres_log_line(detail).unwrap();
                            // RE_LINE.captures(loc_line).unwrap().extract();
                            static RE_LOC: LazyLock<Regex> =
                                LazyLock::new(|| Regex::new(r"^([^,]+), ([^:]+):(\d+)$").unwrap());

                            let (_, [func, file_name, line_nr]) = RE_LOC
                                .captures(&detail.msg)
                                .expect("cannot parse loc msg")
                                .extract();
                            log::debug!("{} {} {}", func, file_name, line_nr);

                            let path_candidates =
                                find_file_path_from_file_name(file_name, Path::new(&ctx.src_path));
                            let iter = path_candidates.into_iter().map(|file_path| {
                                let loc = SourceLocation {
                                    file_path,
                                    line_nr: line_nr.parse::<usize>().unwrap(),
                                };
                                StartingLocation {
                                    loc,
                                    reliability: 1.0,
                                }
                            });
                            starting_points.extend(iter);
                        }
                        None => {
                            log::debug!("parse_postgres(): no location line...");
                        }
                    }
                }
            }
            None => {
                log::debug!("parse_postgres(): skip failed parse line...");
            }
        }
    }

    Ok(starting_points)
}

fn parse_postgres_log_line(loc_line: &str) -> Option<PgLogLine> {
    static RE_LINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(.+{27}) \[(\d+)\] ([^:]+):  (.+)$").unwrap());
    let result = RE_LINE.captures(loc_line);
    result.map(|cap| {
        let (_, [date, marker, severity, msg]) = cap.extract();
        let date = parse_postgres_date(date).unwrap();
        PgLogLine {
            _date: date,
            _marker: marker.to_owned(),
            severity: severity.to_owned(),
            msg: msg.to_owned(),
        }
    })
}

fn parse_postgres_date(date: &str) -> ParseResult<DateTime<FixedOffset>> {
    let (datetime, tz) = date.rsplit_once(' ').unwrap();

    let naive = chrono::naive::NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M:%S%.3f")?;

    let tz = match tz {
        "KST" => FixedOffset::east_opt(9 * 3600).unwrap(),
        _ => {
            // handle other PostgreSQL timezone abbreviations
            unimplemented!()
        }
    };

    Ok(naive.and_local_timezone(tz).unwrap())
}
