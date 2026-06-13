//! Dependency-free date handling (Howard Hinnant's civil <-> days algorithm), so the CLI parses
//! and formats `YYYY-MM-DD` without pulling in a date crate. All dates are UTC midnight.

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - (m <= 2) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mm = m as i64;
    let doy = (153 * (if mm > 2 { mm - 3 } else { mm + 9 }) + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (y + (m <= 2) as i64, m, d)
}

/// Parse `YYYY-MM-DD` into unix seconds (UTC midnight).
pub fn parse_date(s: &str) -> Result<i64, String> {
    let parts: Vec<&str> = s.trim().split('-').collect();
    if parts.len() != 3 {
        return Err(format!("invalid date (want YYYY-MM-DD): {s}"));
    }
    let y = parts[0].parse::<i64>().map_err(|_| format!("bad year: {s}"))?;
    let m = parts[1].parse::<u32>().map_err(|_| format!("bad month: {s}"))?;
    let d = parts[2].parse::<u32>().map_err(|_| format!("bad day: {s}"))?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(format!("date out of range: {s}"));
    }
    Ok(days_from_civil(y, m, d) * 86_400)
}

/// Format unix seconds as `YYYY-MM-DD` (UTC).
pub fn format_date(epoch: i64) -> String {
    let (y, m, d) = civil_from_days(epoch.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970() {
        assert_eq!(parse_date("1970-01-01").unwrap(), 0);
        assert_eq!(format_date(0), "1970-01-01");
    }

    #[test]
    fn round_trips() {
        for s in ["2026-06-13", "2000-02-29", "1999-12-31", "2024-01-01"] {
            assert_eq!(format_date(parse_date(s).unwrap()), s);
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("2026-13-01").is_err());
        assert!(parse_date("2026-06").is_err());
    }
}
