use bevy::prelude::*;

use crate::now_unix;

// ---- business panel (live reports from the hub) ----------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BizTab {
    Agenda,
    Sessions,
    Renewals,
    Forecast,
    Revenue,
    Tax,
    Retention,
}

impl BizTab {
    pub(crate) const ALL: [BizTab; 7] = [
        BizTab::Agenda,
        BizTab::Sessions,
        BizTab::Renewals,
        BizTab::Forecast,
        BizTab::Revenue,
        BizTab::Tax,
        BizTab::Retention,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            BizTab::Agenda => "Agenda",
            BizTab::Sessions => "Sessions",
            BizTab::Renewals => "Renewals",
            BizTab::Forecast => "Forecast",
            BizTab::Revenue => "Revenue",
            BizTab::Tax => "Tax",
            BizTab::Retention => "Retention",
        }
    }
}

#[derive(Resource)]
pub(crate) struct BusinessPanel {
    pub(crate) tab: BizTab,
    pub(crate) text: String,
}

impl Default for BusinessPanel {
    fn default() -> Self {
        Self {
            tab: BizTab::Agenda,
            text: String::new(),
        }
    }
}

/// Open the DB read-only and render the chosen report to text (reuses the CLI `ops` handlers).
pub(crate) fn run_report(path: &str, tab: BizTab) -> String {
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => return format!("cannot open {path}: {e}"),
    };
    let now = now_unix();
    let res = match tab {
        BizTab::Agenda => nbe_cli::ops::agenda(&db, 7, now),
        BizTab::Sessions => nbe_cli::ops::report_sessions(&db, now),
        BizTab::Renewals => nbe_cli::ops::report_renewals(&db, 30, now),
        BizTab::Forecast => nbe_cli::ops::report_forecast(&db, 6, now),
        BizTab::Revenue => nbe_cli::ops::report_revenue(&db),
        BizTab::Tax => nbe_cli::ops::report_tax(&db),
        BizTab::Retention => nbe_cli::ops::report_retention(&db),
    };
    res.unwrap_or_else(|e| format!("error: {e}"))
}
