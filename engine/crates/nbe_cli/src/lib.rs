//! `nbe_cli` — the terminal hub for The Neural Business Engine.
//!
//! Command handlers ([`ops`]) operate on an `nbe_data::Db` and return the text to print, which
//! keeps them headlessly testable. The thin binary (`src/main.rs`) just parses args and
//! dispatches. Data lands in the same single SQLite file the visual engine renders.

pub mod datetime;
pub mod money;
pub mod ops;
pub mod schedule;
