//! Weekday and time-of-day parsing/formatting for the weekly schedule (slots).

/// Parse a weekday name or index into 0=Mon .. 6=Sun.
pub fn parse_weekday(s: &str) -> Result<i64, String> {
    match s.trim().to_lowercase().as_str() {
        "mon" | "monday" | "0" => Ok(0),
        "tue" | "tues" | "tuesday" | "1" => Ok(1),
        "wed" | "weds" | "wednesday" | "2" => Ok(2),
        "thu" | "thur" | "thurs" | "thursday" | "3" => Ok(3),
        "fri" | "friday" | "4" => Ok(4),
        "sat" | "saturday" | "5" => Ok(5),
        "sun" | "sunday" | "6" => Ok(6),
        _ => Err(format!("invalid weekday: {s}")),
    }
}

pub fn weekday_name(d: i64) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .get(d as usize)
        .copied()
        .unwrap_or("?")
}

/// Parse `HH:MM` into minutes from midnight.
pub fn parse_hhmm(s: &str) -> Result<i64, String> {
    let (h, m) = s
        .trim()
        .split_once(':')
        .ok_or_else(|| format!("invalid time (want HH:MM): {s}"))?;
    let h: i64 = h.parse().map_err(|_| format!("bad hour: {s}"))?;
    let m: i64 = m.parse().map_err(|_| format!("bad minute: {s}"))?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(format!("time out of range: {s}"));
    }
    Ok(h * 60 + m)
}

pub fn fmt_hhmm(min: i64) -> String {
    format!("{:02}:{:02}", min / 60, min % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays() {
        assert_eq!(parse_weekday("Tue").unwrap(), 1);
        assert_eq!(parse_weekday("sunday").unwrap(), 6);
        assert_eq!(weekday_name(1), "Tue");
        assert!(parse_weekday("Funday").is_err());
    }

    #[test]
    fn times() {
        assert_eq!(parse_hhmm("09:00").unwrap(), 540);
        assert_eq!(fmt_hhmm(540), "09:00");
        assert!(parse_hhmm("25:00").is_err());
    }
}
