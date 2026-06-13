//! Where ICS text comes from. The mapping logic depends only on the [`EventSource`] trait, so
//! it's testable with in-memory fixtures; the real network fetch is one small adapter.

/// A source of raw iCalendar text.
pub trait EventSource {
    fn fetch(&self) -> Result<String, String>;
}

/// In-memory source (tests, or `--file` reading a saved `.ics`).
pub struct StaticSource(pub String);

impl EventSource for StaticSource {
    fn fetch(&self) -> Result<String, String> {
        Ok(self.0.clone())
    }
}

/// Fetches a private Google Calendar iCal URL over HTTPS (no OAuth). Only compiled with the
/// `http` feature; this is the single piece verified on a networked machine rather than here.
#[cfg(feature = "http")]
pub struct HttpIcsSource {
    pub url: String,
}

#[cfg(feature = "http")]
impl EventSource for HttpIcsSource {
    fn fetch(&self) -> Result<String, String> {
        ureq::get(&self.url)
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_source_returns_its_text() {
        let s = StaticSource("BEGIN:VCALENDAR".into());
        assert_eq!(s.fetch().unwrap(), "BEGIN:VCALENDAR");
    }
}
