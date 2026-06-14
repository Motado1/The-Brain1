//! The Neural Business Engine — renderer + navigation.
//!
//! Loads the graph from SQLite (`--db <path>`, default `brain.db`) and renders it as the
//! "celestial" model: three spatial **clusters** (CRM / Research / Financial), each an organic
//! web of glowing nodes, connected by curved filaments with travelling action-potential sparks.
//! A left **sidebar** lists the clusters and their nodes; clicking flies the camera there.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::PrimitiveTopology;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::view::Hdr;
use bevy::render::RenderPlugin;
use bevy::window::WindowResolution;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use nbe_geometry::{edge_curve, CurveParams};

#[derive(Resource)]
struct DbPath(String);

// ---- business panel (live reports from the hub) ----------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum BizTab {
    Agenda,
    Sessions,
    Renewals,
    Forecast,
    Revenue,
    Retention,
}

impl BizTab {
    const ALL: [BizTab; 6] = [
        BizTab::Agenda,
        BizTab::Sessions,
        BizTab::Renewals,
        BizTab::Forecast,
        BizTab::Revenue,
        BizTab::Retention,
    ];

    fn label(self) -> &'static str {
        match self {
            BizTab::Agenda => "Agenda",
            BizTab::Sessions => "Sessions",
            BizTab::Renewals => "Renewals",
            BizTab::Forecast => "Forecast",
            BizTab::Revenue => "Revenue",
            BizTab::Retention => "Retention",
        }
    }
}

#[derive(Resource)]
struct BusinessPanel {
    tab: BizTab,
    text: String,
}

impl Default for BusinessPanel {
    fn default() -> Self {
        Self {
            tab: BizTab::Agenda,
            text: String::new(),
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open the DB read-only and render the chosen report to text (reuses the CLI `ops` handlers).
fn run_report(path: &str, tab: BizTab) -> String {
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
        BizTab::Retention => nbe_cli::ops::report_retention(&db),
    };
    res.unwrap_or_else(|e| format!("error: {e}"))
}

// ---- clusters --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Cluster {
    Crm,
    Research,
    Financial,
}

impl Cluster {
    const ALL: [Cluster; 3] = [Cluster::Crm, Cluster::Research, Cluster::Financial];

    fn label(self) -> &'static str {
        match self {
            Cluster::Crm => "CRM — Clients",
            Cluster::Research => "Research — Knowledge",
            Cluster::Financial => "Financial — Ledger",
        }
    }

    /// Base emissive hue (pre activation/intensity).
    fn base_color(self) -> (f32, f32, f32) {
        match self {
            Cluster::Crm => (0.12, 0.6, 1.0),       // cyan
            Cluster::Research => (0.2, 1.0, 0.45),  // green
            Cluster::Financial => (1.0, 0.6, 0.12), // amber
        }
    }

    /// Base node radius — clients are the big "neurons".
    fn base_size(self) -> f32 {
        match self {
            Cluster::Crm => 0.95,
            Cluster::Research => 0.5,
            Cluster::Financial => 0.55,
        }
    }

    /// Nominal centre of this cluster's region within the brain.
    fn center(self) -> Vec3 {
        match self {
            Cluster::Crm => Vec3::new(-48.0, 0.0, 0.0),       // left hemisphere
            Cluster::Research => Vec3::new(48.0, 0.0, 0.0),   // right hemisphere
            Cluster::Financial => Vec3::new(0.0, -52.0, -30.0), // lower-central mass
        }
    }
}

const EDGE_WEIGHT_MIN: f64 = 0.55;

// ---- navigation registry + camera target ----------------------------------------------

struct NodeInfo {
    name: String,
    cluster: Cluster,
    pos: Vec3,
}

#[derive(Resource, Default)]
struct NodeRegistry {
    nodes: Vec<NodeInfo>,
    galaxy_center: Vec3,
    galaxy_radius: f32,
}

impl NodeRegistry {
    /// Focus + radius to frame a whole cluster.
    fn cluster_view(&self, cluster: Cluster) -> (Vec3, f32) {
        let pts: Vec<Vec3> = self
            .nodes
            .iter()
            .filter(|n| n.cluster == cluster)
            .map(|n| n.pos)
            .collect();
        if pts.is_empty() {
            return (cluster.center(), 120.0);
        }
        let center = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
        let radius = pts
            .iter()
            .map(|p| p.distance(center))
            .fold(0.0_f32, f32::max)
            .max(20.0);
        (center, radius * 1.4)
    }
}

/// When set, the camera smoothly flies to this (focus, radius).
#[derive(Resource, Default)]
struct CameraTarget(Option<(Vec3, f32)>);

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
    let backends = if cfg!(target_os = "windows") {
        Some(Backends::DX12)
    } else {
        None
    };

    App::new()
        .add_plugins(
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
        .insert_resource(ClearColor(Color::srgb(0.008, 0.015, 0.04)))
        .insert_resource(DbPath(db_path_from_args()))
        .insert_resource(CameraTarget::default())
        .insert_resource(NodeRegistry::default())
        .insert_resource(BusinessPanel::default())
        .add_systems(Startup, (load_graph, setup_hud))
        .add_systems(EguiPrimaryContextPass, (sidebar_ui, business_panel_ui))
        .add_systems(
            Update,
            (orbit_camera, update_hud, animate_breath, animate_sparks),
        )
        .run();
}

// ---- components / helpers --------------------------------------------------------------

#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

impl OrbitCamera {
    fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.focus + Vec3::new(self.radius * cp * sy, self.radius * sp, self.radius * cp * cy)
    }
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct Breath {
    base: f32,
    phase: f32,
    speed: f32,
}

#[derive(Component)]
struct Spark {
    path: usize,
    t: f32,
    speed: f32,
    rng: u64,
}

#[derive(Resource, Default)]
struct EdgePaths(Vec<Vec<Vec3>>);

fn hash_u64(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Deterministic value in [-1, 1] from an id + salt.
fn rand_unit(id: &str, salt: u64) -> f32 {
    let mut h = hash_u64(id) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 32;
    (h >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0
}

fn rand01(id: &str, salt: u64) -> f32 {
    rand_unit(id, salt) * 0.5 + 0.5
}

/// Evenly distributed direction on the unit sphere (Fibonacci).
fn fib_dir(i: usize, n: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * ((i as f32 + 0.5) / n as f32);
    let r = (1.0 - y * y).max(0.0).sqrt();
    let th = i as f32 * golden;
    Vec3::new(r * th.cos(), y, r * th.sin())
}

/// Place a node inside its cluster's region of the brain: two hemispheres (CRM left,
/// Research right) + a lower-central mass (Financial), with a shell bias so nodes gather
/// near the cortex surface like the reference brain.
fn brain_pos(cluster: Cluster, i: usize, n: usize, id: &str) -> Vec3 {
    let dir = fib_dir(i, n);
    let jitter = Vec3::new(rand_unit(id, 12), rand_unit(id, 13), rand_unit(id, 14)) * 4.0;
    let (center, radii, shell) = match cluster {
        Cluster::Crm => (Vec3::new(-48.0, 0.0, 0.0), Vec3::new(50.0, 70.0, 85.0), 0.5),
        Cluster::Research => (Vec3::new(48.0, 0.0, 0.0), Vec3::new(50.0, 70.0, 85.0), 0.5),
        Cluster::Financial => (Vec3::new(0.0, -52.0, -30.0), Vec3::new(42.0, 34.0, 44.0), 0.4),
    };
    let rr = shell + (1.0 - shell) * rand01(id, 11);
    center + (dir * rr) * radii + jitter
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn sample_path(points: &[Vec3], t: f32) -> Vec3 {
    match points.len() {
        0 => Vec3::ZERO,
        1 => points[0],
        n => {
            let tt = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = tt.floor() as usize;
            let f = tt - i as f32;
            let j = (i + 1).min(n - 1);
            points[i].lerp(points[j], f)
        }
    }
}

fn money(cents: i64) -> String {
    let a = cents.abs();
    format!(
        "{}${}.{:02}",
        if cents < 0 { "-" } else { "" },
        a / 100,
        a % 100
    )
}

/// Emissive (HDR) for a node: cluster hue at rest, shifting to a warm amber hotspot as it
/// activates (matching the bright orange clusters in the reference brain).
fn node_emissive(cluster: Cluster, activation: f32) -> LinearRgba {
    let (br, bg, bb) = cluster.base_color();
    let (hr, hg, hb) = (1.0, 0.72, 0.32); // warm amber-white hot
    let t = activation.clamp(0.0, 1.0);
    let mix = |b: f32, h: f32| b + (h - b) * t * 0.9;
    let intensity = 0.9 + activation * 7.0;
    LinearRgba::rgb(
        mix(br, hr) * intensity,
        mix(bg, hg) * intensity,
        mix(bb, hb) * intensity,
    )
}

fn bounds(points: &[Vec3]) -> (Vec3, f32) {
    if points.is_empty() {
        return (Vec3::ZERO, 200.0);
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(20.0);
    (center, radius)
}

fn line_mesh(points: &[Vec3]) -> Mesh {
    let positions: Vec<[f32; 3]> = points.iter().map(|v| v.to_array()).collect();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; points.len()];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; points.len()];
    Mesh::new(
        PrimitiveTopology::LineStrip,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

fn spawn_camera(commands: &mut Commands, focus: Vec3, radius: f32) {
    let cam = OrbitCamera {
        focus,
        radius: radius * 1.35,
        yaw: 0.5,
        pitch: 0.35,
    };
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));
}

// ---- scene build -----------------------------------------------------------------------

fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
) {
    let path = &db_path.0;
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => {
            warn!("could not open '{path}' ({e}); empty scene.");
            spawn_camera(&mut commands, Vec3::ZERO, 200.0);
            return;
        }
    };
    let snap = match nbe_data::snapshot::export(&db) {
        Ok(s) => s,
        Err(e) => {
            warn!("could not read '{path}' ({e})");
            spawn_camera(&mut commands, Vec3::ZERO, 200.0);
            return;
        }
    };
    if snap.entities.is_empty() {
        warn!("no data in '{path}'. Seed with:  nbe --db {path} seed");
        spawn_camera(&mut commands, Vec3::ZERO, 200.0);
        return;
    }
    info!("loaded {} entities, {} edges", snap.entities.len(), snap.edges.len());

    // Cluster + display name per entity (priority CRM > Research > Financial).
    let mut cluster_of: HashMap<String, Cluster> = HashMap::new();
    let mut name_of: HashMap<String, String> = HashMap::new();
    for c in &snap.crm {
        cluster_of.insert(c.entity_id.clone(), Cluster::Crm);
        name_of.insert(
            c.entity_id.clone(),
            c.contact.clone().unwrap_or_else(|| "client".into()),
        );
    }
    for k in &snap.knowledge {
        cluster_of.entry(k.entity_id.clone()).or_insert(Cluster::Research);
        let title = k.body_md.lines().next().unwrap_or("note").trim_start_matches("# ");
        name_of.entry(k.entity_id.clone()).or_insert_with(|| title.to_string());
    }
    for l in &snap.ledger {
        cluster_of.entry(l.entity_id.clone()).or_insert(Cluster::Financial);
        name_of
            .entry(l.entity_id.clone())
            .or_insert_with(|| format!("{} [{}]", money(l.amount_cents), l.invoice_status));
    }

    let activation: HashMap<String, f64> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.clone(), a.value))
        .collect();

    // Group entity ids by cluster (sorted for deterministic positions).
    let mut groups: HashMap<Cluster, Vec<String>> = HashMap::new();
    for e in &snap.entities {
        let cl = *cluster_of.get(&e.id).unwrap_or(&Cluster::Research);
        groups.entry(cl).or_default().push(e.id.clone());
    }
    for ids in groups.values_mut() {
        ids.sort();
    }

    // Position each cluster's nodes inside its region of the brain.
    let mut pos: HashMap<String, Vec3> = HashMap::new();
    for (&cluster, ids) in &groups {
        let n = ids.len().max(1);
        for (i, id) in ids.iter().enumerate() {
            pos.insert(id.clone(), brain_pos(cluster, i, n, id));
        }
    }

    // Spawn nodes + build the navigation registry.
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    registry.nodes.clear();
    for (&cluster, ids) in &groups {
        for id in ids {
            let a = *activation.get(id).unwrap_or(&0.0) as f32;
            let r = cluster.base_size() + a * 0.45;
            let p = pos[id];
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.02, 0.06, 0.14),
                emissive: node_emissive(cluster, a),
                perceptual_roughness: 0.4,
                ..default()
            });
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(p).with_scale(Vec3::splat(r)),
                Breath {
                    base: r,
                    phase: rand_unit(id, 9) * std::f32::consts::TAU,
                    speed: 0.6 + rand01(id, 10) * 0.8,
                },
            ));
            registry.nodes.push(NodeInfo {
                name: name_of.get(id).cloned().unwrap_or_else(|| id[..8].to_string()),
                cluster,
                pos: p,
            });
        }
    }

    // Edges: curved filaments (stronger ones only), reused by the sparks.
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.4),
        emissive: LinearRgba::rgb(0.05, 0.30, 0.70),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let curve_params = CurveParams {
        samples: 18,
        bow: 0.1,
        sag: 0.03,
        jitter: 0.015,
        seed: 0x00E6,
    };
    let mut edge_paths: Vec<Vec<Vec3>> = Vec::new();
    for e in &snap.edges {
        let (Some(&a), Some(&b)) = (pos.get(&e.source_id), pos.get(&e.target_id)) else {
            continue;
        };
        if e.weight < EDGE_WEIGHT_MIN {
            continue;
        }
        let curve = edge_curve(a, b, e.weight as f32, &curve_params, hash_u64(&e.id));
        commands.spawn((
            Mesh3d(meshes.add(line_mesh(&curve))),
            MeshMaterial3d(edge_mat.clone()),
            Transform::default(),
        ));
        edge_paths.push(curve);
    }

    // Action-potential sparks.
    if !edge_paths.is_empty() {
        let spark_mat = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(7.0, 3.0, 0.6),
            ..default()
        });
        let spark_count = (edge_paths.len() * 3 / 2).clamp(20, 400);
        let mut s = 0x9E37_79B9u64;
        for k in 0..spark_count {
            let path = (lcg(&mut s) * edge_paths.len() as f32) as usize % edge_paths.len();
            let t = lcg(&mut s);
            let speed = 0.12 + lcg(&mut s) * 0.35;
            let p = sample_path(&edge_paths[path], t);
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(spark_mat.clone()),
                Transform::from_translation(p).with_scale(Vec3::splat(0.32)),
                Spark {
                    path,
                    t,
                    speed,
                    rng: s ^ (k as u64 + 1),
                },
            ));
        }
    }
    commands.insert_resource(EdgePaths(edge_paths));

    let all: Vec<Vec3> = pos.values().copied().collect();
    let (center, radius) = bounds(&all);
    registry.galaxy_center = center;
    registry.galaxy_radius = radius;
    spawn_camera(&mut commands, center, radius);
}

// ---- UI + camera -----------------------------------------------------------------------

fn sidebar_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut target: ResMut<CameraTarget>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::left("nav")
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("The Brain");
            if ui.button("Galaxy view").clicked() {
                target.0 = Some((registry.galaxy_center, registry.galaxy_radius * 1.35));
            }
            ui.separator();
            for cluster in Cluster::ALL {
                let count = registry.nodes.iter().filter(|n| n.cluster == cluster).count();
                egui::CollapsingHeader::new(format!("{} ({count})", cluster.label()))
                    .default_open(false)
                    .show(ui, |ui| {
                        if ui.button("→ go to cluster").clicked() {
                            let (c, r) = registry.cluster_view(cluster);
                            target.0 = Some((c, r));
                        }
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .id_salt(cluster.label())
                            .show(ui, |ui| {
                                let mut items: Vec<&NodeInfo> = registry
                                    .nodes
                                    .iter()
                                    .filter(|n| n.cluster == cluster)
                                    .collect();
                                items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                                for ni in items {
                                    if ui.button(&ni.name).clicked() {
                                        target.0 = Some((ni.pos, 22.0));
                                    }
                                }
                            });
                    });
            }
        });
}

/// Right-hand panel: live business reports from the hub, with a tab button per report and a
/// refresh button. Read-only — opens the DB on demand when a tab is clicked.
fn business_panel_ui(
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

fn orbit_camera(
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut target: ResMut<CameraTarget>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let dragging = mouse_buttons.pressed(MouseButton::Left);
    let mut drag = Vec2::ZERO;
    if dragging {
        for ev in motion.read() {
            drag += ev.delta;
        }
    } else {
        motion.clear();
    }
    let mut scroll = 0.0;
    for ev in wheel.read() {
        scroll += ev.y;
    }

    // Manual interaction cancels an active fly-to.
    if drag != Vec2::ZERO || scroll != 0.0 {
        target.0 = None;
    }
    let idle = if dragging || target.0.is_some() {
        0.0
    } else {
        time.delta_secs() * 0.05
    };

    for (mut orbit, mut transform) in &mut query {
        if let Some((focus, radius)) = target.0 {
            let k = (time.delta_secs() * 3.5).min(1.0);
            orbit.focus = orbit.focus.lerp(focus, k);
            orbit.radius += (radius - orbit.radius) * k;
            if orbit.focus.distance(focus) < 0.5 && (orbit.radius - radius).abs() < 0.5 {
                target.0 = None;
            }
        }
        orbit.yaw += idle - drag.x * 0.005;
        orbit.pitch = (orbit.pitch + drag.y * 0.005).clamp(-1.4, 1.4);
        let factor = (1.0 - scroll * 0.08).clamp(0.5, 2.0);
        orbit.radius = (orbit.radius * factor).clamp(3.0, 1200.0);
        *transform = Transform::from_translation(orbit.eye()).looking_at(orbit.focus, Vec3::Y);
    }
}

fn animate_breath(time: Res<Time>, mut query: Query<(&Breath, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (b, mut transform) in &mut query {
        let s = b.base * (1.0 + 0.12 * (t * b.speed + b.phase).sin());
        transform.scale = Vec3::splat(s);
    }
}

fn animate_sparks(
    time: Res<Time>,
    paths: Res<EdgePaths>,
    mut query: Query<(&mut Spark, &mut Transform)>,
) {
    if paths.0.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (mut spark, mut transform) in &mut query {
        spark.t += spark.speed * dt;
        if spark.t >= 1.0 {
            spark.t -= 1.0;
            spark.path = (lcg(&mut spark.rng) * paths.0.len() as f32) as usize % paths.0.len();
            spark.speed = 0.12 + lcg(&mut spark.rng) * 0.35;
        }
        transform.translation = sample_path(&paths.0[spark.path], spark.t);
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("loading…"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            right: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));
}

fn update_hud(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<HudText>>) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    for mut text in &mut query {
        text.0 = format!("{fps:5.0} FPS");
    }
}
