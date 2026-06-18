use std::collections::{HashMap, HashSet};

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::Image;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::Hdr;

use nbe_cli::money::format_cents;
use nbe_geometry::{edge_curve, CurveParams};

use crate::anatomy::*;
use crate::components::*;
use crate::domain::*;
use crate::geometry::*;
use crate::interaction::{ClientRevenue, TargetVisualScale, revenue_to_scale};
use crate::nav::*;
use crate::now_unix;
use crate::shaders::{PulseWaveMaterial, SomaMaterial};
use crate::tuning::*;

/// Pre-built additive glow materials for the data anatomy (one palette, reused for every neuron).
struct AnatomyMats {
    session_done: Handle<StandardMaterial>,
    session_missed: Handle<StandardMaterial>,
    package_active: Handle<StandardMaterial>,
    package_done: Handle<StandardMaterial>,
    research: Handle<StandardMaterial>,
}

impl AnatomyMats {
    fn build(materials: &mut Assets<StandardMaterial>, glow: &Handle<Image>) -> Self {
        let mut glow_mat = |rgb: LinearRgba| {
            materials.add(StandardMaterial {
                base_color: Color::LinearRgba(rgb),
                base_color_texture: Some(glow.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                cull_mode: None,
                ..default()
            })
        };
        Self {
            session_done: glow_mat(LinearRgba::new(1.6, 1.1, 0.4, 1.0)), // warm gold
            session_missed: glow_mat(LinearRgba::new(1.4, 0.3, 0.2, 1.0)), // dim red (no-show/cancel)
            package_active: glow_mat(LinearRgba::new(1.7, 1.2, 0.5, 1.0)), // bright amber
            package_done: glow_mat(LinearRgba::new(0.5, 0.4, 0.3, 1.0)),  // spent/dim
            research: glow_mat(LinearRgba::new(0.6, 0.5, 1.6, 1.0)),      // indigo (cross-domain)
        }
    }
}

/// Weave a neuron's data anatomy directly onto its dendrite branches (no detached satellites):
/// session logs become a sequential bead chain along the trunk segments; packages and cross-domain
/// research proxies become terminal twigs at the branch tips. Every element carries a `LodReveal`
/// so it only materialises on deep zoom (the Micro view).
fn embed_anatomy(
    commands: &mut Commands,
    quad: &Handle<Mesh>,
    mats: &AnatomyMats,
    branches: &[DendriteBranch],
    an: &Anatomy,
) {
    // Sessions: a chain of beads strung sequentially along the trunk branches (depth 0).
    let trunks: Vec<&DendriteBranch> = branches.iter().filter(|b| b.depth == 0).collect();
    if !trunks.is_empty() && !an.sessions.is_empty() {
        let n = an.sessions.len().min(28);
        let per_trunk = n.div_ceil(trunks.len());
        for (k, st) in an.sessions.iter().take(n).enumerate() {
            let trunk = trunks[k % trunks.len()];
            let idx = k / trunks.len();
            let t = (idx as f32 + 1.0) / (per_trunk as f32 + 1.0);
            let pos = sample_path(&trunk.points, t);
            let mat = match st {
                SessionStatus::Completed => &mats.session_done,
                _ => &mats.session_missed,
            };
            commands.spawn((
                Mesh3d(quad.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(pos).with_scale(Vec3::ZERO),
                Billboard,
                LodReveal { base_scale: 0.45, start: 0.55, full: 0.85 },
                SceneItem,
            ));
        }
    }

    // Terminal twigs at the leaf tips: packages first, then cross-domain research proxies.
    let tips: Vec<Vec3> = branches
        .iter()
        .filter(|b| b.leaf)
        .filter_map(|b| b.points.last().copied())
        .collect();
    if !tips.is_empty() {
        let mut ti = 0usize;
        let mut place = |commands: &mut Commands, mat: &Handle<StandardMaterial>, scale: f32| {
            let pos = tips[ti % tips.len()];
            ti += 1;
            commands.spawn((
                Mesh3d(quad.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(pos).with_scale(Vec3::ZERO),
                Billboard,
                LodReveal { base_scale: scale, start: 0.45, full: 0.78 },
                SceneItem,
            ));
        };
        for pkg in &an.packages {
            let mat = if pkg.active { &mats.package_active } else { &mats.package_done };
            place(commands, mat, 0.9);
        }
        for _ in 0..an.research_proxies.min(8) {
            place(commands, &mats.research, 0.7);
        }
    }
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
        // Far plane reaches the distant Research network so it stays visible from Business.
        Projection::from(PerspectiveProjection {
            far: 20_000.0,
            near: 0.1,
            ..default()
        }),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.3,
            ..Bloom::NATURAL
        },
        // Deep blue-neutral haze so distant neurons melt into atmospheric depth (reference look).
        DistanceFog {
            color: Color::srgb(0.014, 0.017, 0.026),
            falloff: FogFalloff::ExponentialSquared { density: 0.00055 },
            ..default()
        },
        // Gentle depth-of-field: the focused cluster stays crisp, far/near soften like the refs.
        DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: cam.radius,
            aperture_f_stops: 2.8,
            ..default()
        },
        // Neutral ambient so unlit tissue stays colour-neutral (hue comes from the cores' glow).
        AmbientLight {
            color: Color::srgb(0.72, 0.76, 0.85),
            brightness: 50.0,
            ..default()
        },
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));
}

// ---- scene build -----------------------------------------------------------------------

/// Recompute and persist every neuron's activation from its facets (idempotent). Best-effort —
/// a missing/locked DB just leaves the stored values in place.
fn recompute_activations(path: &str) {
    match nbe_data::Db::open(path, None) {
        Ok(mut db) => {
            if let Err(e) = nbe_cli::ops::recompute_activation(&mut db, now_unix()) {
                warn!("recompute activation failed: {e}");
            }
        }
        Err(e) => warn!("recompute: cannot open '{path}': {e}"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut somas: ResMut<Assets<SomaMaterial>>,
    mut waves: ResMut<Assets<PulseWaveMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
) {
    let (center, radius) = build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
        somas.as_mut(),
        waves.as_mut(),
        images.as_mut(),
        registry.as_mut(),
        &db_path.0,
    )
    .unwrap_or((Vec3::ZERO, 200.0));
    spawn_camera(&mut commands, center, radius);
}

/// Rebuild the graph from the DB when a button has changed it: despawn the old scene and re-create
/// it. The camera persists (untagged), so the view stays put while new neurons pop in.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_reload(
    mut control: ResMut<SceneControl>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut somas: ResMut<Assets<SomaMaterial>>,
    mut waves: ResMut<Assets<PulseWaveMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
    old: Query<Entity, With<SceneItem>>,
) {
    if !control.reload {
        return;
    }
    control.reload = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
        somas.as_mut(),
        waves.as_mut(),
        images.as_mut(),
        registry.as_mut(),
        &db_path.0,
    );
}

/// Load the DB and spawn both networks (nodes, edges, sparks), all tagged `SceneItem`. Returns the
/// framing bounds `(center, radius)` of the **Business** network for the initial camera, or `None`
/// if the DB is missing/empty.
#[allow(clippy::too_many_arguments)]
fn build_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    somas: &mut Assets<SomaMaterial>,
    waves: &mut Assets<PulseWaveMaterial>,
    images: &mut Assets<Image>,
    registry: &mut NodeRegistry,
    path: &str,
) -> Option<(Vec3, f32)> {
    registry.nodes.clear();
    // Make activation honest before we read it: recompute each neuron's value from its real facet
    // urgency (recency, invoice status, renewal proximity) so firing rate reflects what matters.
    recompute_activations(path);
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => {
            warn!("could not open '{path}' ({e}); empty scene.");
            return None;
        }
    };
    let snap = match nbe_data::snapshot::export(&db) {
        Ok(s) => s,
        Err(e) => {
            warn!("could not read '{path}' ({e})");
            return None;
        }
    };
    let anatomy = build_anatomy(&snap);
    if snap.entities.is_empty() {
        warn!("no data in '{path}'. Seed with:  nbe --db {path} seed");
        return None;
    }
    info!("loaded {} entities, {} edges", snap.entities.len(), snap.edges.len());

    // Kind + display name per entity (priority Client > Knowledge > Ledger).
    let mut kind_of: HashMap<String, Kind> = HashMap::new();
    let mut name_of: HashMap<String, String> = HashMap::new();
    for c in &snap.crm {
        kind_of.insert(c.entity_id.clone(), Kind::Client);
        name_of.insert(
            c.entity_id.clone(),
            c.contact.clone().unwrap_or_else(|| "client".into()),
        );
    }
    for k in &snap.knowledge {
        kind_of.entry(k.entity_id.clone()).or_insert(Kind::Knowledge);
        let title = k.body_md.lines().next().unwrap_or("note").trim_start_matches("# ");
        name_of.entry(k.entity_id.clone()).or_insert_with(|| title.to_string());
    }
    for l in &snap.ledger {
        kind_of.entry(l.entity_id.clone()).or_insert(Kind::Ledger);
        name_of
            .entry(l.entity_id.clone())
            .or_insert_with(|| format!("{} [{}]", format_cents(l.amount_cents), l.invoice_status));
    }

    let activation: HashMap<String, f64> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.clone(), a.value))
        .collect();

    // Financial roll-up: per-client revenue from connected (non-expense) ledger entries, plus a
    // grand total. This is the "combine financial into the client network" surfacing.
    let ledger_amt: HashMap<&str, (i64, bool)> = snap
        .ledger
        .iter()
        .map(|l| (l.entity_id.as_str(), (l.amount_cents, l.is_expense)))
        .collect();
    let client_ids: HashSet<&str> = snap.crm.iter().map(|c| c.entity_id.as_str()).collect();
    let renewal_of: HashMap<&str, Option<i64>> =
        snap.crm.iter().map(|c| (c.entity_id.as_str(), c.renewal_date)).collect();
    let mut revenue_of: HashMap<String, i64> = HashMap::new();
    for e in &snap.edges {
        if client_ids.contains(e.source_id.as_str()) {
            if let Some(&(amt, exp)) = ledger_amt.get(e.target_id.as_str()) {
                if !exp {
                    *revenue_of.entry(e.source_id.clone()).or_default() += amt;
                }
            }
        }
        if client_ids.contains(e.target_id.as_str()) {
            if let Some(&(amt, exp)) = ledger_amt.get(e.source_id.as_str()) {
                if !exp {
                    *revenue_of.entry(e.target_id.clone()).or_default() += amt;
                }
            }
        }
    }
    registry.total_revenue_cents = snap
        .ledger
        .iter()
        .filter(|l| !l.is_expense)
        .map(|l| l.amount_cents)
        .sum();

    // Group entity ids by network (sorted for deterministic positions).
    let mut groups: HashMap<Network, Vec<String>> = HashMap::new();
    for e in &snap.entities {
        let kind = *kind_of.get(&e.id).unwrap_or(&Kind::Knowledge);
        // Ledger entries are folded into their client (money already rolls into the client neuron),
        // so the overview reads as ~one neuron per client, not client + every invoice.
        if kind == Kind::Ledger {
            continue;
        }
        groups.entry(kind.network()).or_default().push(e.id.clone());
    }
    for ids in groups.values_mut() {
        ids.sort();
    }

    // Position each network's nodes inside an ellipsoid sized to its node count (constant density).
    let mut pos: HashMap<String, Vec3> = HashMap::new();
    let mut net_radii: HashMap<Network, Vec3> = HashMap::new();
    for (&network, ids) in &groups {
        let n = ids.len().max(1);
        let radii = density_radii(n);
        net_radii.insert(network, radii);
        for (i, id) in ids.iter().enumerate() {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            pos.insert(id.clone(), net_pos(network.center(), radii, kind, i, n, id));
        }
    }

    // Spawn nodes + build the navigation registry. Each soma = a granular, lumpy structural mass
    // (real 3D form, Target 1) under additive glow billboards (the light from within). A small shared
    // pool of displaced-sphere variants gives organic variety without a unique mesh per neuron.
    let soma_pool: Vec<Handle<Mesh>> = (0..SOMA_VARIANTS)
        .map(|k| meshes.add(displaced_sphere(SOMA_SUBDIV, SOMA_BUMP, k as u64 + 1)))
        .collect();
    let halo_quad = meshes.add(Rectangle::new(1.0, 1.0));
    let glow = images.add(glow_texture());
    let anatomy_mats = AnatomyMats::build(materials, &glow);
    let halo_for = |kind: Kind, materials: &mut Assets<StandardMaterial>| {
        let (r, g, b) = kind.base_color();
        materials.add(StandardMaterial {
            // Subtle bloom — a soft glow, not a star. Multiplied by the radial-gradient texture.
            base_color: Color::LinearRgba(LinearRgba::new(r * 1.3, g * 1.3, b * 1.3, 1.0)),
            base_color_texture: Some(glow.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            ..default()
        })
    };
    let halo_mats = [
        (Kind::Client, halo_for(Kind::Client, materials)),
        (Kind::Ledger, halo_for(Kind::Ledger, materials)),
        (Kind::Knowledge, halo_for(Kind::Knowledge, materials)),
    ];
    // Per-network theme: warm amber for Business, cool blue-purple for Research. Every accent
    // material (the translucent cell membrane, the filament edges, the dendrites, and the
    // travelling pulses) is tinted from this single hue, so the whole cluster reads in one colour.
    // Only the node cores carry the per-kind base colour. The membrane is see-through so the bright
    // core inside reads as light glowing from within, not the node being a bare light source.
    let theme_rgb = |net: Network| match net {
        // Deeper, hotter red-orange (was a pale amber 1.0,0.55,0.15) — the molten neuron look.
        Network::Business => (1.0, 0.30, 0.07),
        Network::Research => (0.5, 0.42, 1.0),
    };
    type ThemeMats = (
        Handle<StandardMaterial>, // membrane
        Handle<StandardMaterial>, // edge
        Handle<StandardMaterial>, // dendrite
        Handle<StandardMaterial>, // pulse
    );
    let mut themes: HashMap<Network, ThemeMats> = HashMap::new();
    // Per-network Fresnel "cell-wall" material for the soma sphere (custom shader).
    let mut soma_of: HashMap<Network, Handle<SomaMaterial>> = HashMap::new();
    // Dendrites wear the unified glassy wave material (a per-soma instance is built at spawn time so
    // each tree can carry its own firing surge) — see the node loop. The soma keeps its own
    // Fresnel cell-wall material here.
    for net in Network::ALL {
        let (r, g, b) = theme_rgb(net);
        soma_of.insert(
            net,
            somas.add(SomaMaterial {
                rim_color: LinearRgba::new(r, g, b, 1.0),
                params: Vec4::new(RIM_POWER, RIM_INTENSITY, RIM_ALPHA, 0.0),
            }),
        );
        // The tissue (membrane / edges / dendrites) is near-neutral translucent glass that takes its
        // colour from the LIGHT — the bright cores blooming through/over it and the scene lights —
        // rather than glowing its own hue. Only a faint theme whisper so dim, far-from-core tissue
        // still hints at its network. (The cores, halos, pulses, and motes remain the real emitters.)
        // Translucent structural body — kept very faint so it gives form without over-blending
        // (and thus flickering) against the additive glow that sits at the same point.
        let membrane = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.48, 0.46, 0.13),
            emissive: LinearRgba::rgb(r * 0.04, g * 0.04, b * 0.04),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            perceptual_roughness: 0.4,
            metallic: 0.0,
            ..default()
        });
        // Thin, *glowing* filaments — like lit fibre-optic threads (the reference look). Now that
        // they're thin, a colour glow reads as light, not orange plastic. Blend + emissive + bloom.
        let edge = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.10),
            emissive: LinearRgba::rgb(r * 0.6, g * 0.6, b * 0.6),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            perceptual_roughness: 0.06,
            metallic: 0.0,
            reflectance: 0.7,
            ..default()
        });
        let dendrite = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.10),
            emissive: LinearRgba::rgb(r * 0.45, g * 0.45, b * 0.45),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            perceptual_roughness: 0.1,
            metallic: 0.0,
            ..default()
        });
        let pulse = materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::new(r * 2.6, g * 2.6, b * 2.6, 1.0)),
            base_color_texture: Some(glow.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            ..default()
        });
        themes.insert(net, (membrane, edge, dendrite, pulse));
    }

    // Per-entity firing threshold (defaults 0.5).
    let threshold_of: HashMap<&str, f32> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.as_str(), a.threshold as f32))
        .collect();

    let mut graph = BrainGraph::default();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut radius_of: HashMap<String, f32> = HashMap::new();

    for (&network, ids) in &groups {
        for id in ids {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            let act = *activation.get(id).unwrap_or(&0.0) as f32;
            let thr = threshold_of.get(id.as_str()).copied().unwrap_or(0.5);
            let r = kind.base_size() + act * 0.45;
            let p = pos[id];
            let base_emissive = node_emissive(kind, act);
            // Soma = a pure additive light volume (no solid mesh): a tight bright nucleus billboard
            // plus a wide soft halo, so it reads as a glowing cloud (refs) instead of a hard sphere
            // or a white blob. The nucleus material is the per-node light, flared by firing.
            let core_mat = materials.add(StandardMaterial {
                base_color: Color::LinearRgba(LinearRgba::new(
                    base_emissive.red,
                    base_emissive.green,
                    base_emissive.blue,
                    1.0,
                )),
                base_color_texture: Some(glow.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                cull_mode: None,
                ..default()
            });
            // Wide soft outer volume (camera-facing) — spawned first so the neuron can flare it.
            let halo_mat = halo_mats
                .iter()
                .find(|(k, _)| *k == kind)
                .map(|(_, h)| h.clone())
                .unwrap();
            let halo = commands
                .spawn((
                    Mesh3d(halo_quad.clone()),
                    MeshMaterial3d(halo_mat),
                    Transform::from_translation(p).with_scale(Vec3::splat(r * 3.5)),
                    Billboard,
                    SceneItem,
                ))
                .id();

            let phase = rand_unit(id, 9) * std::f32::consts::TAU;
            let speed = 0.6 + rand01(id, 10) * 0.8;
            let idx = graph.nodes.len();
            // Bright nucleus billboard — carries the firing state and is the picking target.
            let node = commands
                .spawn((
                    Mesh3d(halo_quad.clone()),
                    MeshMaterial3d(core_mat.clone()),
                    // Core kept smaller than the ~r membrane sphere so the clear membrane perfectly
                    // surrounds the glowing nucleus (was r*1.6, which spilled past the shell).
                    Transform::from_translation(p).with_scale(Vec3::splat(r * 0.9)),
                    Billboard,
                    Breath { base: Vec3::splat(r * 0.9), phase, speed },
                    Neuron(idx),
                    // Random initial charge so neurons don't all fire in lockstep.
                    Firing {
                        accumulator: rand01(id, 7) * thr,
                        intensity: 0.0,
                    },
                    NodeViz {
                        base_emissive,
                        base_radius: r,
                        halo,
                        mat: core_mat,
                        phase: rand_unit(id, 8) * std::f32::consts::TAU,
                        twinkle: 0.08 + rand01(id, 15) * 0.08,
                    },
                    SceneItem,
                ))
                .id();
            // Translucent cell body via the Fresnel "cell-wall" shader: glowing rim, clear centre —
            // gives the soma real 3D structure while the additive nucleus reads as light within it.
            let shape = Vec3::new(
                r * (0.8 + rand01(id, 21) * 0.45),
                r * (0.8 + rand01(id, 22) * 0.45),
                r * (0.8 + rand01(id, 23) * 0.45),
            );
            let tilt = Quat::from_euler(
                EulerRot::XYZ,
                rand_unit(id, 24) * std::f32::consts::PI,
                rand_unit(id, 25) * std::f32::consts::PI,
                rand_unit(id, 26) * std::f32::consts::PI,
            );
            commands.spawn((
                Mesh3d(soma_pool[(hash_u64(id) as usize) % SOMA_VARIANTS].clone()),
                MeshMaterial3d(soma_of[&network].clone()),
                Transform { translation: p, rotation: tilt, scale: shape },
                Breath { base: shape, phase, speed },
                SceneItem,
            ));

            graph.nodes.push(GraphNode {
                entity: node,
                activation: act,
                threshold: thr,
                out: Vec::new(),
            });
            index.insert(id.clone(), idx);
            radius_of.insert(id.clone(), r);

            // Clients carry their earned revenue → a target visual weight (financial aggregator).
            if kind == Kind::Client {
                let rev = revenue_of.get(id).copied().unwrap_or(0);
                commands
                    .entity(node)
                    .insert((ClientRevenue(rev), TargetVisualScale(revenue_to_scale(rev))));
            }

            // Radiating dendrites — clients and notes sprout identically (ledger folded out).
            let dcount = match kind {
                Kind::Client | Kind::Knowledge => 5,
                Kind::Ledger => 0,
            };
            if dcount > 0 {
                let tree = dendrite_tree(p, dcount, r, hash_u64(id));
                // Embed this neuron's data anatomy (session beads + package/research twigs) directly
                // onto the branches before the mesh is consumed — woven in, never orbiting.
                if let Some(an) = anatomy.get(id) {
                    embed_anatomy(commands, &halo_quad, &anatomy_mats, &tree.branches, an);
                }
                let (dr, dg, db) = theme_rgb(network);
                let dwave_mat = waves.add(PulseWaveMaterial {
                    color: LinearRgba::new(dr, dg, db, 1.0),
                    rest: Vec4::new(DEND_RIM_POWER, DEND_RIM_INTENSITY, DEND_RIM_ALPHA, 0.0),
                    wave: Vec4::new(0.0, 0.0, PULSE_WIDTH, 0.0),
                });
                commands.spawn((
                    Mesh3d(meshes.add(tree.mesh)),
                    MeshMaterial3d(dwave_mat.clone()),
                    Transform::default(),
                    // Start past the tip so it's idle until the soma first fires.
                    DendriteWave { soma: node, mat: dwave_mat, t: 99.0, prev_intensity: 0.0 },
                    SceneItem,
                ));
            }
            registry.nodes.push(NodeInfo {
                id: id.clone(),
                entity: node,
                name: name_of.get(id).cloned().unwrap_or_else(|| id[..8].to_string()),
                kind,
                network,
                pos: p,
                revenue_cents: (kind == Kind::Client).then(|| revenue_of.get(id).copied().unwrap_or(0)),
                renewal: if kind == Kind::Client {
                    renewal_of.get(id.as_str()).copied().flatten()
                } else {
                    None
                },
            });
        }
    }

    // Edges: thin tapered glass filaments, drawn only between two nodes in the same network and
    // tinted with that network's theme colour. Low roughness + emissive give them a faint glow.
    let curve_params = CurveParams {
        samples: 14,
        bow: 0.1,
        sag: 0.03,
        jitter: 0.015,
        seed: 0x00E6,
    };
    // Wire an undirected connection into the graph as two directed edges sharing one `channel`
    // (so traffic control can lock both directions together) for propagation.
    fn wire(graph: &mut BrainGraph, si: usize, di: usize, channel: usize) {
        let e1 = graph.edges.len();
        graph.edges.push(GraphEdge { target: di, channel });
        graph.nodes[si].out.push(e1);
        let e2 = graph.edges.len();
        graph.edges.push(GraphEdge { target: si, channel });
        graph.nodes[di].out.push(e2);
    }

    // Organic mesh: connect each neuron to its few nearest neighbours within its own network. This
    // gives the delicate, evenly-woven web of the reference images (and makes both networks read
    // alike) instead of relying on the sparse, mostly-cross-domain data links. Firing propagates
    // along this mesh. (Semantic links return in the per-client zoom-in.)
    const NEIGHBOURS: usize = 3;
    let mut channel_count = 0usize;
    for (&network, ids) in &groups {
        let (nr, ng, nb) = theme_rgb(network);
        // Small additive glow dots strung along each filament — the "beads of light" of the refs.
        let bead_mat = themes[&network].3.clone();
        let idxs: Vec<usize> = ids.iter().map(|id| index[id]).collect();
        let ps: Vec<Vec3> = ids.iter().map(|id| pos[id]).collect();
        let mut linked: HashSet<(usize, usize)> = HashSet::new();
        for i in 0..ps.len() {
            let mut nearest: Vec<(f32, usize)> = (0..ps.len())
                .filter(|&j| j != i)
                .map(|j| (ps[i].distance_squared(ps[j]), j))
                .collect();
            nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(_, j) in nearest.iter().take(NEIGHBOURS) {
                let key = (i.min(j), i.max(j));
                if !linked.insert(key) {
                    continue;
                }
                // Anchor each axon *inside* the soma surface (ROOT_EMBED) and flare its ends wide
                // (ROOT_FLARE) so it fuses into the cell body like a tree root, not a clean pipe
                // kissing the surface (Targets 2/3).
                let ri = radius_of.get(&ids[i]).copied().unwrap_or(1.0);
                let rj = radius_of.get(&ids[j]).copied().unwrap_or(1.0);
                let dir = (ps[j] - ps[i]).normalize_or_zero();
                let (a, b) = if ps[i].distance(ps[j]) > ri + rj + 1.0 {
                    (ps[i] + dir * ri * ROOT_EMBED, ps[j] - dir * rj * ROOT_EMBED)
                } else {
                    (ps[i], ps[j])
                };
                let seed = (key.0 as u64).wrapping_shl(20) ^ key.1 as u64 ^ idxs[i] as u64;
                let curve = edge_curve(a, b, 0.7, &curve_params, seed);
                // Thin waist, flaring wide where it grips each soma.
                let radii = branch_radii(curve.len(), ri * ROOT_FLARE, rj * ROOT_FLARE, BRANCH_TIP_RATIO);
                // Unified glassy tube material (same Fresnel as the dendrites) that also carries the
                // travelling pulse wave. `fwd_edge` = the i→j directed edge wire() is about to push,
                // so the wave system can orient a pulse's `t` to this tube's uv.x.
                let fwd_edge = graph.edges.len();
                let wave_mat = waves.add(PulseWaveMaterial {
                    color: LinearRgba::new(nr, ng, nb, 1.0),
                    rest: Vec4::new(DEND_RIM_POWER, DEND_RIM_INTENSITY, DEND_RIM_ALPHA, 0.0),
                    wave: Vec4::new(0.0, 0.0, PULSE_WIDTH, 0.0),
                });
                commands.spawn((
                    Mesh3d(meshes.add(tube_mesh(&curve, &radii, 8))),
                    MeshMaterial3d(wave_mat.clone()),
                    Transform::default(),
                    ConnectionWave { channel: channel_count, fwd_edge, mat: wave_mat },
                    SceneItem,
                ));
                // Additive glow dot at each junction — light compounds where roots meet the surface.
                for jp in [a, b] {
                    commands.spawn((
                        Mesh3d(halo_quad.clone()),
                        MeshMaterial3d(bead_mat.clone()),
                        Transform::from_translation(jp).with_scale(Vec3::splat(JUNCTION_GLOW)),
                        Billboard,
                        SceneItem,
                    ));
                }
                // Beads of light strung along the filament (a signature of the reference images).
                for bt in [0.25f32, 0.5, 0.75] {
                    commands.spawn((
                        Mesh3d(halo_quad.clone()),
                        MeshMaterial3d(bead_mat.clone()),
                        Transform::from_translation(sample_path(&curve, bt))
                            .with_scale(Vec3::splat(0.55)),
                        Billboard,
                        SceneItem,
                    ));
                }
                wire(&mut graph, idxs[i], idxs[j], channel_count);
                channel_count += 1;
            }
        }
    }

    // Ambient dust motes drifting in each network's volume (the bokeh specks).
    let mote_mat = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(0.6, 0.34, 0.16, 1.0)),
        base_color_texture: Some(glow.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let mut ms = 0xD1B5_4A32u64;
    for network in Network::ALL {
        let center = network.center();
        let radius = net_radii.get(&network).map(|r| r.max_element()).unwrap_or(120.0) * 1.6;
        for _ in 0..MOTES_PER_NETWORK {
            let dir = Vec3::new(
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
            );
            let p = center + dir * radius;
            let vel = Vec3::new(
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
            ) * 2.0;
            // Mostly tiny specks with an occasional larger bokeh orb (cubic falloff).
            let l = lcg(&mut ms);
            let sz = 0.2 + l * l * l * 2.8;
            commands.spawn((
                Mesh3d(halo_quad.clone()),
                MeshMaterial3d(mote_mat.clone()),
                Transform::from_translation(p).with_scale(Vec3::splat(sz)),
                Billboard,
                Mote {
                    vel,
                    center,
                    radius,
                },
                SceneItem,
            ));
        }
    }

    // Background micro-fibers: a dim, hair-thin layer per network for biological depth (static).
    for network in Network::ALL {
        let Some(&nr) = net_radii.get(&network) else {
            continue;
        };
        let (tr, tg, tb) = theme_rgb(network);
        // Neutral, near-unlit background tissue — only a whisper of theme hue.
        let fiber_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.48, 0.46, 0.05),
            emissive: LinearRgba::rgb(tr * 0.03, tg * 0.03, tb * 0.03),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });
        let seed = match network {
            Network::Business => 0xA1F0_3C7B,
            Network::Research => 0xB2E1_9D45,
        };
        commands.spawn((
            Mesh3d(meshes.add(background_fibers_mesh(network.center(), nr, 200, seed))),
            MeshMaterial3d(fiber_mat),
            Transform::default(),
            SceneItem,
        ));
    }

    commands.insert_resource(graph);
    commands.insert_resource(EdgeTraffic {
        channels: (0..channel_count).map(|_| Channel::default()).collect(),
    });

    let all: Vec<Vec3> = pos.values().copied().collect();
    let (center, radius) = bounds(&all);
    registry.galaxy_center = center;
    registry.galaxy_radius = radius;
    // Frame the Business network for the opening shot (Research is a distant cluster behind it).
    Some(registry.network_view(Network::Business))
}
