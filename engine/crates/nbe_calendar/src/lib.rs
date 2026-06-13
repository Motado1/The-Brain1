//! `nbe_calendar` — pull-only Google Calendar sync via the private iCal (.ics) URL.
//!
//! Parsing ([`ics`]) and event→client matching ([`matcher`]) are pure and headlessly tested;
//! the live HTTPS fetch ([`source::HttpIcsSource`], behind the `http` feature) is the only part
//! that touches the network, and only when the user explicitly syncs.

pub mod ics;
pub mod matcher;
pub mod source;

pub use ics::{parse_ics, CalendarEvent};
pub use matcher::{match_events, MatchedSession};
pub use source::{EventSource, StaticSource};

#[cfg(feature = "http")]
pub use source::HttpIcsSource;
