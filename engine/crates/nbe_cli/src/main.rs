//! `nbe` — terminal hub. Parses args, opens the (optionally encrypted) database, dispatches to
//! a handler in `nbe_cli::ops`, and prints the result.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use nbe_cli::ops;
use nbe_data::Db;

#[derive(Parser)]
#[command(name = "nbe", about = "Neural Business Engine — local hub for finance, research & clients")]
struct Cli {
    /// Path to the single-file database.
    #[arg(long, default_value = "brain.db", global = true)]
    db: PathBuf,
    /// Encryption passphrase (or set NBE_PASSPHRASE). Omit for a plaintext database.
    #[arg(long, global = true)]
    passphrase: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create / open the database and report.
    Init,
    /// Show row counts.
    Stats,
    /// Fill with a demo graph for the visual engine.
    Seed {
        #[arg(long, default_value_t = 500)]
        entities: usize,
        #[arg(long, default_value_t = 1200)]
        edges: usize,
    },

    /// Add a client.
    ClientAdd {
        name: String,
        #[arg(long, default_value = "active")]
        stage: String,
        /// Renewal date, YYYY-MM-DD.
        #[arg(long)]
        renewal: Option<String>,
        #[arg(long)]
        schedule: Option<String>,
    },
    /// List clients.
    ClientList,
    /// Update a client in place (only the flags you pass change; --renewal "" clears it).
    ClientUpdate {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        stage: Option<String>,
        /// Renewal date YYYY-MM-DD, or "" to clear.
        #[arg(long)]
        renewal: Option<String>,
        #[arg(long)]
        schedule: Option<String>,
    },

    /// Add an invoice (income).
    InvoiceAdd {
        amount: String,
        #[arg(long, default_value = "draft")]
        status: String,
        /// Short id of the client to link this invoice to.
        #[arg(long)]
        client: Option<String>,
        #[arg(long)]
        tax_bucket: Option<String>,
    },
    /// Add an expense.
    ExpenseAdd {
        amount: String,
        #[arg(long, default_value = "expense")]
        tax_bucket: String,
    },

    /// Add a research note.
    NoteAdd {
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long)]
        template: Option<String>,
    },
    /// Import a markdown file (e.g. a Gemini doc) as a note, linked to topics from its
    /// Tags: line/front-matter plus any --topic given.
    NoteImport {
        path: PathBuf,
        #[arg(long)]
        topic: Vec<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, default_value = "draft")]
        status: String,
    },
    /// List notes (optionally only those tagged with a topic).
    NoteList {
        #[arg(long)]
        tag: Option<String>,
    },
    /// Tag a note with a topic (creates the topic on first use).
    NoteTag { note: String, topic: String },
    /// Remove a topic tag from a note.
    NoteUntag { note: String, topic: String },
    /// List all research topics with note counts.
    TopicList,
    /// Brush-up view: notes least-recently reviewed first (optionally within a topic).
    Review {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    /// Mark a note reviewed now (fires its neuron so it lights up, then cools over 7 days).
    NoteReview { id: String },
    /// Update a note in place (title / body / review status).
    NoteUpdate {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        /// draft | reviewed | archived
        #[arg(long)]
        status: Option<String>,
    },

    /// Link two entities by short id.
    Link {
        source: String,
        target: String,
        #[arg(long = "type", default_value = "reference")]
        edge_type: String,
        #[arg(long, default_value_t = 1.0)]
        weight: f64,
    },
    /// Remove link(s) between two entities (optionally only one --type).
    Unlink {
        source: String,
        target: String,
        #[arg(long = "type")]
        edge_type: Option<String>,
    },
    /// Delete an entity and everything that cascades from it (irreversible).
    Delete { id: String },
    /// Show one entity (facets + links).
    Show { id: String },

    /// Financial roll-up.
    ReportFinance,
    /// Upcoming client renewals.
    ReportRenewals {
        #[arg(long, default_value_t = 30)]
        within: i64,
    },
    /// Live activation ranking.
    ReportActivation {
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// Recompute & persist every entity's activation from its facets (refreshes the renderer).
    RecomputeActivation,

    /// Record a package purchase / renewal (paid up front).
    PackageAdd {
        client: String,
        /// e.g. PT10, PT20, PT30
        kind: String,
        #[arg(long)]
        price: String,
        /// Purchase date YYYY-MM-DD (defaults to today).
        #[arg(long)]
        date: Option<String>,
    },
    /// List packages (optionally for one client) with their short ids.
    PackageList {
        #[arg(long)]
        client: Option<String>,
    },
    /// Delete a package by short id (restores the previous package if it was active).
    PackageDelete { id: String },
    /// Log a training session as it occurs.
    SessionLog {
        client: String,
        #[arg(long)]
        date: Option<String>,
        #[arg(long, default_value = "completed")]
        status: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// List sessions (optionally for one client) with their short ids.
    SessionList {
        #[arg(long)]
        client: Option<String>,
    },
    /// Correct a logged session by short id (date / status / note).
    SessionUpdate {
        id: String,
        #[arg(long)]
        date: Option<String>,
        /// completed | no_show | cancelled
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Delete a logged session by short id.
    SessionDelete { id: String },
    /// Add a recurring weekly slot.
    SlotAdd {
        client: String,
        #[arg(long)]
        day: String,
        #[arg(long)]
        time: String,
        #[arg(long, default_value_t = 60)]
        duration: i64,
        #[arg(long, default_value_t = 1.0)]
        cadence: f64,
    },
    /// List all weekly slots.
    SlotList,
    /// Edit a weekly slot by short id (day / time / duration / cadence).
    SlotUpdate {
        id: String,
        #[arg(long)]
        day: Option<String>,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        duration: Option<i64>,
        #[arg(long)]
        cadence: Option<f64>,
    },
    /// Delete a weekly slot by short id.
    SlotDelete { id: String },
    /// Revenue: cash-in by month + earned over time.
    ReportRevenue,
    /// Project monthly income forward from active packages + slot cadence (cash paid up front).
    ReportForecast {
        #[arg(long, default_value_t = 6)]
        months: i64,
    },
    /// Weekly work hours from slots.
    ReportHours,
    /// Retention: renewal rate, repeat clients, avg packages/client, lifecycle breakdown.
    ReportRetention,
    /// Day-by-day schedule for the days ahead (from recurring slots), marking logged sessions.
    Agenda {
        #[arg(long, default_value_t = 7)]
        days: i64,
    },
    /// Active packages: remaining sessions + renewal ETA.
    ReportSessions,
    /// Clients about to run out of sessions — re-sell before they lapse.
    Nudges {
        #[arg(long, default_value_t = 2)]
        within_sessions: i64,
        #[arg(long, default_value_t = 2.0)]
        within_weeks: f64,
    },
    /// Save the Google Calendar private ICS URL.
    CalendarSetUrl { url: String },
    /// Pull the calendar and log matching sessions (offline-by-default; runs only on demand).
    CalendarSync {
        /// Read a local .ics file instead of the saved URL.
        #[arg(long)]
        file: Option<PathBuf>,
    },

    /// Export a JSON snapshot.
    Export { path: PathBuf },
    /// Import a JSON snapshot.
    Import { path: PathBuf },
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn run(cli: Cli) -> nbe_data::Result<String> {
    let pass = cli
        .passphrase
        .clone()
        .or_else(|| std::env::var("NBE_PASSPHRASE").ok());
    let mut db = Db::open(&cli.db, pass.as_deref())?;
    let now = now_unix();

    match cli.command {
        Command::Init => Ok(format!("database ready at {}", cli.db.display())),
        Command::Stats => ops::stats(&db),
        Command::Seed { entities, edges } => ops::seed_demo(&mut db, entities, edges),

        Command::ClientAdd {
            name,
            stage,
            renewal,
            schedule,
        } => ops::client_add(
            &mut db,
            &name,
            &stage,
            renewal.as_deref(),
            schedule.as_deref(),
            now,
        ),
        Command::ClientList => ops::client_list(&db),
        Command::ClientUpdate {
            id,
            name,
            stage,
            renewal,
            schedule,
        } => ops::client_update(
            &mut db,
            &id,
            name.as_deref(),
            stage.as_deref(),
            renewal.as_deref(),
            schedule.as_deref(),
            now,
        ),

        Command::InvoiceAdd {
            amount,
            status,
            client,
            tax_bucket,
        } => ops::invoice_add(
            &mut db,
            &amount,
            &status,
            client.as_deref(),
            tax_bucket.as_deref(),
            now,
        ),
        Command::ExpenseAdd { amount, tax_bucket } => {
            ops::expense_add(&mut db, &amount, &tax_bucket, now)
        }

        Command::NoteAdd {
            title,
            body,
            template,
        } => ops::note_add(&mut db, &title, &body, template.as_deref(), now),
        Command::NoteImport {
            path,
            topic,
            title,
            status,
        } => ops::note_import(&mut db, &path, &topic, title.as_deref(), &status, now),
        Command::NoteList { tag } => ops::note_list(&db, tag.as_deref()),
        Command::NoteTag { note, topic } => ops::note_tag(&mut db, &note, &topic, now),
        Command::NoteUntag { note, topic } => ops::note_untag(&mut db, &note, &topic),
        Command::TopicList => ops::topic_list(&db),
        Command::Review { tag, limit } => ops::review(&db, tag.as_deref(), limit, now),
        Command::NoteReview { id } => ops::note_review(&mut db, &id, now),
        Command::NoteUpdate {
            id,
            title,
            body,
            status,
        } => ops::note_update(
            &mut db,
            &id,
            title.as_deref(),
            body.as_deref(),
            status.as_deref(),
        ),

        Command::Link {
            source,
            target,
            edge_type,
            weight,
        } => ops::link(&mut db, &source, &target, &edge_type, weight),
        Command::Unlink {
            source,
            target,
            edge_type,
        } => ops::unlink(&mut db, &source, &target, edge_type.as_deref()),
        Command::Delete { id } => ops::delete(&mut db, &id),
        Command::Show { id } => ops::show(&db, &id, now),

        Command::ReportFinance => ops::report_finance(&db),
        Command::ReportRenewals { within } => ops::report_renewals(&db, within, now),
        Command::ReportActivation { top } => ops::report_activation(&db, top, now),
        Command::RecomputeActivation => ops::recompute_activation(&mut db, now),

        Command::PackageAdd {
            client,
            kind,
            price,
            date,
        } => ops::package_add(&mut db, &client, &kind, &price, date.as_deref(), now),
        Command::PackageList { client } => ops::package_list(&db, client.as_deref()),
        Command::PackageDelete { id } => ops::package_delete(&mut db, &id, now),
        Command::SessionLog {
            client,
            date,
            status,
            note,
        } => ops::session_log(&mut db, &client, date.as_deref(), &status, note.as_deref(), now),
        Command::SessionList { client } => ops::session_list(&db, client.as_deref()),
        Command::SessionUpdate {
            id,
            date,
            status,
            note,
        } => ops::session_update(
            &mut db,
            &id,
            date.as_deref(),
            status.as_deref(),
            note.as_deref(),
            now,
        ),
        Command::SessionDelete { id } => ops::session_delete(&mut db, &id, now),
        Command::SlotAdd {
            client,
            day,
            time,
            duration,
            cadence,
        } => ops::slot_add(&mut db, &client, &day, &time, duration, cadence, now),
        Command::SlotList => ops::slot_list(&db),
        Command::SlotUpdate {
            id,
            day,
            time,
            duration,
            cadence,
        } => ops::slot_update(
            &mut db,
            &id,
            day.as_deref(),
            time.as_deref(),
            duration,
            cadence,
            now,
        ),
        Command::SlotDelete { id } => ops::slot_delete(&mut db, &id, now),
        Command::ReportRevenue => ops::report_revenue(&db),
        Command::ReportForecast { months } => ops::report_forecast(&db, months, now),
        Command::ReportHours => ops::report_hours(&db),
        Command::ReportRetention => ops::report_retention(&db),
        Command::Agenda { days } => ops::agenda(&db, days, now),
        Command::ReportSessions => ops::report_sessions(&db, now),
        Command::Nudges {
            within_sessions,
            within_weeks,
        } => ops::nudges(&db, within_sessions, within_weeks, now),
        Command::CalendarSetUrl { url } => ops::calendar_set_url(&db, &url),
        Command::CalendarSync { file } => ops::calendar_sync(&mut db, file.as_deref(), now),

        Command::Export { path } => ops::export(&db, &path),
        Command::Import { path } => ops::import(&mut db, &path),
    }
}

fn main() {
    match run(Cli::parse()) {
        Ok(msg) => println!("{msg}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
