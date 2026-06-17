//! The Neural Business Engine — renderer + navigation.
//!
//! Loads the graph from SQLite (`--db <path>`, default `brain.db`) and renders it as two distinct
//! **networks** floating far apart in space, like separate constellations:
//!   * **Business** — CRM clients + their financial ledger, one interconnected web. Per-client
//!     revenue (summed from connected ledger entries) and renewal dates surface in the sidebar.
//!   * **Research** — the knowledge base, off in the distance, growing its own web as notes link
//!     to topic hubs.
//!
//! Nodes are glowing neurons that fire and flare from their activation; firing propagates pulses
//! of light along the glass-tube edges to neighbours, a real cascade. Ambient twinkle + drifting
//! dust keep it alive. Camera: left-drag orbits, right/middle-drag pans, scroll flies, Esc unfocuses.

mod anatomy;
mod components;
mod domain;
mod geometry;
mod interaction;
mod lod;
mod nav;
mod panel;
mod scene;
mod shaders;
mod systems;
mod tuning;
mod ui;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::WindowResolution;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use crate::components::*;
use crate::interaction::*;
use crate::lod::*;
use crate::nav::*;
use crate::panel::*;
use crate::scene::*;
use crate::systems::*;
use crate::ui::*;

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn db_path_from_args() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--db" {
            if let Some(v) = args.next() {
                return v;
            }
        }
    }
    "brain.db".to_string()
}

fn main() {
    // Render backend. Default **Vulkan** on Windows (DX12's swapchain `ResizeBuffers` panics on
    // window maximize/resize in this Bevy/wgpu version — Vulkan is rock-solid on the RTX 5080 and is
    // still native wgpu, no browser/Electron). `NBE_BACKEND=dx12|vulkan|gl` overrides.
    let backends = match std::env::var("NBE_BACKEND").ok().as_deref() {
        Some("vulkan") | Some("vk") => Some(Backends::VULKAN),
        Some("dx12") | Some("dx") => Some(Backends::DX12),
        Some("gl") => Some(Backends::GL),
        _ if cfg!(target_os = "windows") => Some(Backends::VULKAN),
        _ => None,
    };

    let mut app = App::new();
    app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "The Neural Business Engine".into(),
                        resolution: WindowResolution::new(1600, 900),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends,
                        power_preference: PowerPreference::HighPerformance,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::srgb(0.009, 0.011, 0.017)))
        .insert_resource(DbPath(db_path_from_args()))
        .insert_resource(CameraTarget::default())
        .insert_resource(NodeRegistry::default())
        .insert_resource(BrainGraph::default())
        .insert_resource(EdgeTraffic::default())
        .insert_resource(BusinessPanel::default())
        .insert_resource(SceneControl::default())
        .insert_resource(UiPointer::default())
        .insert_resource(Picker::default())
        .insert_resource(LodState::default())
        .insert_resource(InteractionState::default())
        .add_message::<UiRequestSprout>()
        .add_message::<UiRequestLink>()
        .add_message::<UiRequestEdit>()
        .add_message::<UiRequestDissolve>()
        .add_systems(Startup, (load_graph, setup_hud, spawn_lights))
        .add_systems(
            EguiPrimaryContextPass,
            (sidebar_ui, business_panel_ui, detail_panel_ui, sync_ui_pointer).chain(),
        )
        .add_systems(
            Update,
            (
                pick_node,
                orbit_camera,
                compute_lod,
                apply_lod_reveal,
                update_dof,
                face_camera,
                update_hud,
                animate_breath,
                fire_scheduler,
                fire_render,
                advance_pulses,
                drift_motes,
                apply_reload,
            ),
        )
        .add_systems(
            Update,
            (
                update_interaction,
                purge_dissolving,
                update_financial_scale,
                on_sprout,
                on_link,
                on_edit,
                on_dissolve,
            ),
        );

    // Custom WGSL materials (soma Fresnel rim, …) — registered after DefaultPlugins.
    shaders::register(&mut app);
    app.run();
}
