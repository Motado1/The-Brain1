use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use nbe_cli::datetime::format_date;
use nbe_cli::money::format_cents;

use crate::components::*;
use crate::domain::*;
use crate::nav::*;
use crate::now_unix;
use crate::panel::*;

// ---- UI + camera -----------------------------------------------------------------------

pub(crate) fn sidebar_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut target: ResMut<CameraTarget>,
    mut control: ResMut<SceneControl>,
    db_path: Res<DbPath>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::left("nav")
        .default_width(270.0)
        .show(ctx, |ui| {
            ui.heading("The Brain");
            if ui.button("➕ Add Research").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown", "txt"])
                    .pick_file()
                {
                    let mut reload = false;
                    let status = match nbe_data::Db::open(&db_path.0, None) {
                        Ok(mut db) => {
                            match nbe_cli::ops::note_import(&mut db, &file, &[], None, "draft", now_unix()) {
                                Ok(msg) => {
                                    reload = true;
                                    msg
                                }
                                Err(e) => format!("import failed: {e}"),
                            }
                        }
                        Err(e) => format!("cannot open db: {e}"),
                    };
                    control.status = status;
                    control.reload = reload;
                }
            }
            if !control.status.is_empty() {
                ui.label(&control.status);
            }
            if ui.button("Galaxy view").clicked() {
                target.0 = Some((registry.galaxy_center, registry.galaxy_radius * 1.35));
            }
            ui.small("left-drag orbit · right-drag pan · scroll fly · Esc unfocus");
            ui.separator();

            for network in Network::ALL {
                let count = registry.nodes.iter().filter(|n| n.network == network).count();
                egui::CollapsingHeader::new(format!("{} ({count})", network.label()))
                    .default_open(network == Network::Business)
                    .show(ui, |ui| {
                        if ui.button("→ go to network").clicked() {
                            let (c, r) = registry.network_view(network);
                            target.0 = Some((c, r));
                        }
                        if network == Network::Business {
                            ui.label(format!(
                                "Total revenue: {}",
                                format_cents(registry.total_revenue_cents)
                            ));
                            client_list(ui, &registry, &mut target);
                        } else {
                            knowledge_list(ui, &registry, &mut target);
                        }
                    });
            }
        });
}

/// Business clients, each expandable to its revenue + renewal date, with a fly-to button.
fn client_list(ui: &mut egui::Ui, registry: &NodeRegistry, target: &mut CameraTarget) {
    let mut items: Vec<(usize, &NodeInfo)> = registry
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == Kind::Client)
        .collect();
    items.sort_by(|(_, a), (_, b)| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    egui::ScrollArea::vertical()
        .max_height(320.0)
        .id_salt("clients")
        .show(ui, |ui| {
            for (idx, ni) in items {
                egui::CollapsingHeader::new(&ni.name)
                    .id_salt(idx)
                    .show(ui, |ui| {
                        ui.label(format!(
                            "Revenue: {}",
                            format_cents(ni.revenue_cents.unwrap_or(0))
                        ));
                        match ni.renewal {
                            Some(d) => ui.label(format!("Renewal: {}", format_date(d))),
                            None => ui.label("Renewal: —"),
                        };
                        if ui.button("→ fly to").clicked() {
                            target.0 = Some((ni.pos, 22.0));
                        }
                    });
            }
        });
}

/// Research notes, each a fly-to button.
fn knowledge_list(ui: &mut egui::Ui, registry: &NodeRegistry, target: &mut CameraTarget) {
    let mut items: Vec<&NodeInfo> = registry
        .nodes
        .iter()
        .filter(|n| n.kind == Kind::Knowledge)
        .collect();
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    egui::ScrollArea::vertical()
        .max_height(320.0)
        .id_salt("knowledge")
        .show(ui, |ui| {
            for ni in items {
                if ui.button(&ni.name).clicked() {
                    target.0 = Some((ni.pos, 18.0));
                }
            }
        });
}

/// Right-hand panel: live business reports from the hub, with a tab button per report and a
/// refresh button. Read-only — opens the DB on demand when a tab is clicked.
pub(crate) fn business_panel_ui(
    mut contexts: EguiContexts,
    mut panel: ResMut<BusinessPanel>,
    db_path: Res<DbPath>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    // Populate on first frame.
    if panel.text.is_empty() {
        let tab = panel.tab;
        panel.text = run_report(&db_path.0, tab);
    }
    egui::SidePanel::right("business")
        .default_width(330.0)
        .show(ctx, |ui| {
            ui.heading("Business");
            ui.horizontal_wrapped(|ui| {
                for tab in BizTab::ALL {
                    if ui.selectable_label(panel.tab == tab, tab.label()).clicked() {
                        panel.tab = tab;
                        panel.text = run_report(&db_path.0, tab);
                    }
                }
            });
            if ui.button("⟳ Refresh").clicked() {
                let tab = panel.tab;
                panel.text = run_report(&db_path.0, tab);
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.monospace(&panel.text);
            });
        });
}

/// Mark whether the pointer is over an egui panel, so the camera ignores scroll/drag there.
pub(crate) fn sync_ui_pointer(mut contexts: EguiContexts, mut ui: ResMut<UiPointer>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        ui.over = ctx.is_pointer_over_area() || ctx.wants_pointer_input();
    }
}
