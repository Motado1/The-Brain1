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
    /// List notes.
    NoteList,

    /// Link two entities by short id.
    Link {
        source: String,
        target: String,
        #[arg(long = "type", default_value = "reference")]
        edge_type: String,
        #[arg(long, default_value_t = 1.0)]
        weight: f64,
    },
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
    /// Revenue: cash-in by month + earned over time.
    ReportRevenue,
    /// Weekly work hours from slots.
    ReportHours,
    /// Active packages: remaining sessions + renewal ETA.
    ReportSessions,
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
        Command::NoteList => ops::note_list(&db),

        Command::Link {
            source,
            target,
            edge_type,
            weight,
        } => ops::link(&mut db, &source, &target, &edge_type, weight),
        Command::Show { id } => ops::show(&db, &id, now),

        Command::ReportFinance => ops::report_finance(&db),
        Command::ReportRenewals { within } => ops::report_renewals(&db, within, now),
        Command::ReportActivation { top } => ops::report_activation(&db, top, now),

        Command::PackageAdd {
            client,
            kind,
            price,
            date,
        } => ops::package_add(&mut db, &client, &kind, &price, date.as_deref(), now),
        Command::SessionLog {
            client,
            date,
            status,
            note,
        } => ops::session_log(&mut db, &client, date.as_deref(), &status, note.as_deref(), now),
        Command::SlotAdd {
            client,
            day,
            time,
            duration,
            cadence,
        } => ops::slot_add(&mut db, &client, &day, &time, duration, cadence),
        Command::SlotList => ops::slot_list(&db),
        Command::ReportRevenue => ops::report_revenue(&db),
        Command::ReportHours => ops::report_hours(&db),
        Command::ReportSessions => ops::report_sessions(&db, now),
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
