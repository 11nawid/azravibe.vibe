//! System time built-ins and UTC formatting.

use super::num;
use crate::ast::Builtin;
use crate::error::AzraError;
use crate::token::Num;
use crate::value::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn eval_time(name: Builtin, args: &[Value], line: usize) -> Result<Value, AzraError> {
    match name {
        Builtin::CurrentTime => {
            if !args.is_empty() {
                return Err(AzraError::runtime_at(
                    "`current time` takes no arguments",
                    line,
                ));
            }
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            Ok(num(Num::Int(secs)))
        }
        Builtin::FormatTime => {
            let secs = match args {
                [] => SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                [v] => v.as_number("format time", line)?.as_f64() as i64,
                _ => {
                    return Err(AzraError::runtime_at(
                        "`format time` takes at most 1 argument",
                        line,
                    ));
                }
            };
            Ok(Value::Str(format_utc(secs)))
        }
        _ => unreachable!("non-time builtin dispatched to eval_time"),
    }
}

/// Format a unix timestamp as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, h, m, s
    )
}

/// Howard Hinnant's `civil_from_days` (proleptic Gregorian calendar).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}
