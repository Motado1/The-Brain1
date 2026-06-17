//! Money is stored as integer cents everywhere; the CLI converts to/from dollars at the edges.

/// Parse a dollar amount (`"500"`, `"$1,250.50"`) into integer cents.
pub fn parse_dollars(s: &str) -> Result<i64, String> {
    let cleaned: String = s
        .trim()
        .trim_start_matches('$')
        .chars()
        .filter(|c| *c != ',')
        .collect();
    let dollars: f64 = cleaned
        .parse()
        .map_err(|_| format!("invalid amount: {s}"))?;
    Ok((dollars * 100.0).round() as i64)
}

/// Format integer cents as `$D.CC`.
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let a = cents.abs();
    format!("{sign}${}.{:02}", a / 100, a % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_various_forms() {
        assert_eq!(parse_dollars("500").unwrap(), 50_000);
        assert_eq!(parse_dollars("$1,250.50").unwrap(), 125_050);
        assert_eq!(parse_dollars("0.99").unwrap(), 99);
        assert!(parse_dollars("abc").is_err());
    }

    #[test]
    fn formats_cents() {
        assert_eq!(format_cents(50_000), "$500.00");
        assert_eq!(format_cents(125_050), "$1250.50");
        assert_eq!(format_cents(-99), "-$0.99");
    }

    #[test]
    fn round_trips() {
        for d in ["500.00", "1250.50", "0.99"] {
            let cents = parse_dollars(d).unwrap();
            assert_eq!(format_cents(cents), format!("${d}"));
        }
    }
}
