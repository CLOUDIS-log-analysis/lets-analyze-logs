use chrono::{DateTime, FixedOffset, ParseResult};
use serde_json::Value;

use crate::{Log, SourceLocation, StartingLocation};

/// parse mongodb json log
/// extract file path and line number if log line has '"attr":{"file":"...","line":..., ...}'
pub fn parse_mongodb_json(log: &Log) -> anyhow::Result<Vec<StartingLocation>> {
    log::info!("running parse_mongodb()...");

    let mut starting_points = Vec::new();

    let mut log_lines = log.iter();

    let first_line = log_lines.next().ok_or(anyhow::Error::msg("no lines"))?;

    let v = serde_json::from_str(first_line);

    match v {
        Ok(v) => {
            let origin = parse_mongodb_date(&v)?;

            for log_line in log_lines {
                let v: Value = serde_json::from_str(log_line).unwrap();
                let date = &v["t"]["$date"];
                let date = date.as_str().unwrap();

                let date = DateTime::parse_from_rfc3339(date)?;

                if matches!(v["s"].as_str().unwrap(), "F") {
                    let sp = (|| {
                        let attr = v.get("attr")?;
                        let file = attr.get("file")?.as_str()?.to_owned();
                        let line_nr = attr.get("line")?.as_i64()? as usize;

                        let loc = SourceLocation {
                            file_path: file,
                            line_nr,
                        };
                        Some(StartingLocation {
                            loc,
                            reliability: 1.0,
                        })
                    })();
                    if let Some(sp) = sp {
                        log::info!("{:?}", &sp);
                        starting_points.push(sp);
                    }
                }

                if origin.signed_duration_since(&date) > chrono::TimeDelta::seconds(3) {
                    break;
                }
            }

            Ok(starting_points)
        }
        Err(_) => {
            log::info!("parse_mongodb_json(): failed");
            Ok(vec![])
        }
    }
}

fn parse_mongodb_date(v: &Value) -> ParseResult<DateTime<FixedOffset>> {
    let date = &v["t"]["$date"];
    let date = date.to_string();
    let date = &date[1..(date.len() - 1)]; // "str" -> str

    DateTime::parse_from_rfc3339(date)
}
