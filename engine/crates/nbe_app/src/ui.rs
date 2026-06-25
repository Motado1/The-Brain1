use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use nbe_cli::datetime::format_date;
use nbe_cli::money::format_cents;

use crate::components::*;
use crate::domain::*;
use crate::interaction::{SessionOutcome, UiRequestSessionLog};
use crate::nav::*;
use crate::now_unix;
use crate::panel::*;
use crate::search::fuzzy_rank;
use crate::tuning::*;

// ---- UI + camera -----------------------------------------------------------------------

/// Global navigation dock (top strip): one-click jumps to the three regions of the brain. CRM and
/// Research frame their network; Finance frames the business cluster and flips the right-hand panel
/// to the Forecast report so the money view is one click away.
pub(crate) fn dock_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut glide: ResMut<CameraGlide>,
    mut panel: ResMut<BusinessPanel>,
    db_path: Res<DbPath>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::TopBottomPanel::top("dock").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("The Brain").strong());
            ui.separator();
            ui.label("Jump to:");
            if ui.button("🧑 CRM").clicked() {
                let (c, r) = registry.network_view(Network::Business);
                glide.to(c, r);
            }
            if ui.button("💰 Finance").clicked() {
                let (c, r) = registry.network_view(Network::Business);
                glide.to(c, r);
                panel.tab = BizTab::Forecast;
                panel.text = run_report(&db_path.0, BizTab::Forecast);
            }
            if ui.button("📚 Research").clicked() {
                let (c, r) = registry.network_view(Network::Research);
                glide.to(c, r);
            }
            ui.separator();
            if ui.button("🌌 Galaxy").clicked() {
                glide.to(registry.galaxy_center, registry.galaxy_radius * 1.35);
            }
        });
    });
}

pub(crate) fn sidebar_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut glide: ResMut<CameraGlide>,
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
                glide.to(registry.galaxy_center, registry.galaxy_radius * 1.35);
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
                            glide.to(c, r);
                        }
                        if network == Network::Business {
                            ui.label(format!(
                                "Total revenue: {}",
                                format_cents(registry.total_revenue_cents)
                            ));
                            client_list(ui, &registry, &mut glide);
                        } else {
                            knowledge_list(ui, &registry, &mut glide);
                        }
                    });
            }
        });
}

/// Business clients, each expandable to its revenue + renewal date, with a fly-to button.
fn client_list(ui: &mut egui::Ui, registry: &NodeRegistry, glide: &mut CameraGlide) {
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
                            glide.to(ni.pos, 22.0);
                        }
                    });
            }
        });
}

/// Research notes, each a fly-to button.
fn knowledge_list(ui: &mut egui::Ui, registry: &NodeRegistry, glide: &mut CameraGlide) {
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
                    glide.to(ni.pos, 18.0);
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

/// Floating detail panel for the selected node — its profile "planets" (aspects), one-click session
/// actions for clients, and the full `ops::show` view. Click a neuron to open.
pub(crate) fn detail_panel_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut picker: ResMut<Picker>,
    mut log_tx: MessageWriter<UiRequestSessionLog>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Some(sel) = picker.selected else {
        return;
    };
    let Some(node) = registry.nodes.get(sel) else {
        return;
    };
    let title = node.name.clone();
    let is_client = node.kind == Kind::Client;
    let aspects = &node.aspects;
    let mut open = true;
    let mut outcome: Option<SessionOutcome> = None;
    egui::Window::new(format!("🔎 {title}"))
        .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -12.0])
        .default_width(320.0)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            // Profile planets, as text: a filled dot + value bar for present aspects, a hollow dot
            // for the dim placeholders.
            if !aspects.is_empty() {
                ui.label(egui::RichText::new("Profile").strong());
                for a in aspects {
                    ui.horizontal(|ui| {
                        ui.monospace(if a.present { "●" } else { "○" });
                        ui.label(&a.label);
                        if a.present {
                            ui.add(egui::ProgressBar::new(a.value).desired_width(70.0));
                        }
                    });
                }
                ui.separator();
            }
            // One-click session logging for clients → routes through `ops::session_log`.
            if is_client {
                ui.label(egui::RichText::new("Log a session").strong());
                ui.horizontal_wrapped(|ui| {
                    for o in [SessionOutcome::Completed, SessionOutcome::NoShow, SessionOutcome::Cancelled] {
                        if ui.button(o.label()).clicked() {
                            outcome = Some(o);
                        }
                    }
                });
                ui.separator();
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                ui.monospace(&picker.detail);
            });
        });
    if let Some(o) = outcome {
        log_tx.write(UiRequestSessionLog { node: sel, outcome: o });
    }
    if !open {
        picker.selected = None;
        picker.detail.clear();
    }
}

/// Mark whether the pointer is over an egui panel, so the camera ignores scroll/drag there.
pub(crate) fn sync_ui_pointer(mut contexts: EguiContexts, mut ui: ResMut<UiPointer>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        ui.over = ctx.is_pointer_over_area() || ctx.wants_pointer_input();
    }
}

/// Toggle the omni-search overlay with Ctrl+P or Cmd+K (the Raycast/Spotlight gesture).
pub(crate) fn omni_toggle(keys: Res<ButtonInput<KeyCode>>, mut omni: ResMut<OmniSearch>) {
    let modifier = keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight);
    if modifier && (keys.just_pressed(KeyCode::KeyP) || keys.just_pressed(KeyCode::KeyK)) {
        omni.open = !omni.open;
        omni.selected = 0;
    }
}

fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Client => "client",
        Kind::Ledger => "ledger",
        Kind::Knowledge => "note",
    }
}

/// Centred floating omni-search command bar (Ctrl+P / Cmd+K). Types run an offline fuzzy scan across
/// the loaded node corpus; ↑/↓ moves the selection and Enter flies the camera to the chosen node with
/// a cinematic glide. Esc (or clicking away / toggling again) dismisses it.
pub(crate) fn omni_search_ui(
    mut contexts: EguiContexts,
    mut omni: ResMut<OmniSearch>,
    registry: Res<NodeRegistry>,
    mut glide: ResMut<CameraGlide>,
) {
    if !omni.open {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        omni.open = false;
        return;
    }
    let move_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let move_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
    let submit = ctx.input(|i| i.key_pressed(egui::Key::Enter));

    let labels: Vec<&str> = registry.nodes.iter().map(|n| n.name.as_str()).collect();
    let mut chosen: Option<usize> = None;

    egui::Window::new("omni_search")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 140.0])
        .fixed_size([560.0, 0.0])
        .show(ctx, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut omni.query)
                    .hint_text("Search the brain…   ↑↓ to move · Enter to fly")
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Heading),
            );
            edit.request_focus();
            ui.separator();

            let ranked = fuzzy_rank(&omni.query, &labels);
            if move_down {
                omni.selected = omni.selected.saturating_add(1);
            }
            if move_up {
                omni.selected = omni.selected.saturating_sub(1);
            }
            omni.selected = if ranked.is_empty() {
                0
            } else {
                omni.selected.min(ranked.len() - 1)
            };

            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for (row, &ni) in ranked.iter().take(20).enumerate() {
                    let n = &registry.nodes[ni];
                    let label = format!("{}    ·    {} / {}", n.name, n.network.label(), kind_word(n.kind));
                    if ui.selectable_label(row == omni.selected, label).clicked() {
                        chosen = Some(ni);
                    }
                }
                if ranked.is_empty() && !omni.query.trim().is_empty() {
                    ui.weak("no matches");
                }
            });

            if submit && !ranked.is_empty() {
                chosen = Some(ranked[omni.selected.min(ranked.len() - 1)]);
            }
        });

    if let Some(ni) = chosen {
        if let Some(n) = registry.nodes.get(ni) {
            glide.to(n.pos, OMNI_FLY_RADIUS);
        }
        omni.open = false;
        omni.query.clear();
        omni.selected = 0;
    }
}

/// One-shot glassmorphic theme: deeply translucent dark panels so the glowing 3D dendrites and
/// background bokeh drift behind the UI, with paper-thin rim-glow borders echoing the soma Fresnel
/// look. (egui can't truly *blur* the framebuffer behind a panel — that needs a custom render pass —
/// so this delivers the translucency + thin emissive trim; real frosted blur is a later slice.)
pub(crate) fn configure_theme(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut visuals = egui::Visuals::dark();
    let glass = egui::Color32::from_rgba_unmultiplied(10, 15, 25, 140); // ≈ rgba(10,15,25,0.55)
    visuals.panel_fill = glass;
    visuals.window_fill = glass;
    visuals.extreme_bg_color = egui::Color32::from_rgba_unmultiplied(6, 10, 18, 160);
    // Paper-thin border with a soft cool rim-glow, mimicking the cell-wall Fresnel.
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 165, 225, 70));
    ctx.set_visuals(visuals);
    *done = true;
}
