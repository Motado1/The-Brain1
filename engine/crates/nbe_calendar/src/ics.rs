//! Minimal iCalendar (.ics) parsing — enough for Google Calendar VEVENTs (UID, SUMMARY,
//! DTSTART, DTEND), including RFC 5545 line unfolding and the common datetime forms.

/// One calendar event. Times are unix seconds (ICS values without a `Z` are treated as UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub start: i64,
    pub end: Option<i64>,
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - (m <= 2) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse an ICS datetime: `YYYYMMDD`, `YYYYMMDDTHHMMSS`, or `YYYYMMDDTHHMMSSZ`.
pub fn parse_ics_datetime(value: &str) -> Option<i64> {
    let v = value.trim();
    let v = v.strip_suffix('Z').unwrap_or(v);
    let p = |s: &str| s.parse::<i64>().ok();

    if v.len() == 8 {
        let (y, m, d) = (p(&v[0..4])?, p(&v[4..6])?, p(&v[6..8])?);
        return Some(days_from_civil(y, m, d) * 86_400);
    }
    if v.len() >= 15 && v.as_bytes()[8] == b'T' {
        let (y, m, d) = (p(&v[0..4])?, p(&v[4..6])?, p(&v[6..8])?);
        let (hh, mm, ss) = (p(&v[9..11])?, p(&v[11..13])?, p(&v[13..15])?);
        return Some(days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss);
    }
    None
}

/// Join RFC 5545 folded lines (continuation lines begin with a space or tab).
fn unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t')) {
            if let Some(last) = out.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        out.push(line.to_string());
    }
    out
}

/// Split `NAME;param=…:VALUE` into the upper-cased property name and its raw value.
fn split_prop(line: &str) -> (String, &str) {
    let colon = line.find(':').unwrap_or(line.len());
    let (head, rest) = line.split_at(colon);
    let value = rest.strip_prefix(':').unwrap_or("");
    let name = head.split(';').next().unwrap_or("").to_uppercase();
    (name, value)
}

/// Unescape ICS TEXT values (`\n`, `\,`, `\;`, `\\`).
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Default)]
struct Builder {
    uid: Option<String>,
    summary: Option<String>,
    start: Option<i64>,
    end: Option<i64>,
}

impl Builder {
    fn build(self) -> Option<CalendarEvent> {
        Some(CalendarEvent {
            uid: self.uid?,
            summary: self.summary.unwrap_or_default(),
            start: self.start?,
            end: self.end,
        })
    }
}

/// Parse all VEVENTs from an iCalendar document. Events missing a UID or DTSTART are skipped.
pub fn parse_ics(text: &str) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut cur: Option<Builder> = None;

    for line in unfold(text) {
        let (name, value) = split_prop(&line);
        match name.as_str() {
            "BEGIN" if value == "VEVENT" => cur = Some(Builder::default()),
            "END" if value == "VEVENT" => {
                if let Some(b) = cur.take() {
                    if let Some(e) = b.build() {
                        events.push(e);
                    }
                }
            }
            "UID" => {
                if let Some(b) = cur.as_mut() {
                    b.uid = Some(value.to_string());
                }
            }
            "SUMMARY" => {
                if let Some(b) = cur.as_mut() {
                    b.summary = Some(unescape(value));
                }
            }
            "DTSTART" => {
                if let Some(b) = cur.as_mut() {
                    b.start = parse_ics_datetime(value);
                }
            }
            "DTEND" => {
                if let Some(b) = cur.as_mut() {
                    b.end = parse_ics_datetime(value);
                }
            }
            _ => {}
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utc_and_date_only() {
        assert_eq!(parse_ics_datetime("19700101"), Some(0));
        assert_eq!(parse_ics_datetime("19700101T000000Z"), Some(0));
        assert_eq!(parse_ics_datetime("19700102T000100Z"), Some(86_400 + 60));
    }

    #[test]
    fn parses_vevents_with_folding() {
        let ics = "BEGIN:VCALENDAR\r\n\
                   BEGIN:VEVENT\r\n\
                   UID:abc-123\r\n\
                   SUMMARY:James Bywater\r\n \
                   PT session\r\n\
                   DTSTART:20260613T090000Z\r\n\
                   DTEND:20260613T100000Z\r\n\
                   END:VEVENT\r\n\
                   END:VCALENDAR\r\n";
        let events = parse_ics(ics);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].uid, "abc-123");
        // folded continuation joined
        assert_eq!(events[0].summary, "James BywaterPT session");
        assert!(events[0].end.unwrap() - events[0].start == 3600);
    }

    #[test]
    fn skips_events_without_required_fields() {
        let ics = "BEGIN:VEVENT\nSUMMARY:no uid or start\nEND:VEVENT\n";
        assert!(parse_ics(ics).is_empty());
    }
}
