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

use crate::components::*;
use crate::domain::*;
use crate::geometry::*;
use crate::nav::*;
use crate::now_unix;
use crate::tuning::*;

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
        // Warm dark haze so distant neurons melt into amber depth rather than a hard cutoff.
        DistanceFog {
            color: Color::srgb(0.05, 0.022, 0.012),
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
        // A touch of warm ambient so the glass tubes aren't pure black where unlit.
        AmbientLight {
            color: Color::srgb(1.0, 0.7, 0.45),
            brightness: 60.0,
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

pub(crate) fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
) {
    let (center, radius) = build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
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
        images.as_mut(),
        registry.as_mut(),
        &db_path.0,
    );
}

/// Load the DB and spawn both networks (nodes, edges, sparks), all tagged `SceneItem`. Returns the
/// framing bounds `(center, radius)` of the **Business** network for the initial camera, or `None`
/// if the DB is missing/empty.
fn build_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
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
        groups.entry(kind.network()).or_default().push(e.id.clone());
    }
    for ids in groups.values_mut() {
        ids.sort();
    }

    // Position each network's nodes inside its ellipsoid.
    let mut pos: HashMap<String, Vec3> = HashMap::new();
    for (&network, ids) in &groups {
        let n = ids.len().max(1);
        for (i, id) in ids.iter().enumerate() {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            pos.insert(id.clone(), net_pos(network, kind, i, n, id));
        }
    }

    // Spawn nodes + build the navigation registry.
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    let halo_quad = meshes.add(Rectangle::new(1.0, 1.0));
    let glow = images.add(glow_texture());
    let halo_for = |kind: Kind, materials: &mut Assets<StandardMaterial>| {
        let (r, g, b) = kind.base_color();
        materials.add(StandardMaterial {
            // base_color > 1 → HDR halo that blooms; multiplied by the radial-gradient texture.
            base_color: Color::LinearRgba(LinearRgba::new(r * 2.6, g * 2.6, b * 2.6, 1.0)),
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
    // Warm translucent glass for the dendrite filaments.
    let dendrite_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.22, 0.16),
        emissive: LinearRgba::rgb(0.5, 0.17, 0.04),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.1,
        metallic: 0.0,
        ..default()
    });

    // Per-entity firing threshold (defaults 0.5).
    let threshold_of: HashMap<&str, f32> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.as_str(), a.threshold as f32))
        .collect();

    let mut graph = BrainGraph::default();
    let mut index: HashMap<String, usize> = HashMap::new();

    for (&network, ids) in &groups {
        for id in ids {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            let act = *activation.get(id).unwrap_or(&0.0) as f32;
            let thr = threshold_of.get(id.as_str()).copied().unwrap_or(0.5);
            let r = kind.base_size() + act * 0.45;
            let p = pos[id];
            let base_emissive = node_emissive(kind, act);
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.03, 0.01),
                emissive: base_emissive,
                perceptual_roughness: 0.4,
                ..default()
            });
            // Soft glow halo (camera-facing) — spawned first so the neuron can flare it.
            let halo_mat = halo_mats
                .iter()
                .find(|(k, _)| *k == kind)
                .map(|(_, h)| h.clone())
                .unwrap();
            let halo = commands
                .spawn((
                    Mesh3d(halo_quad.clone()),
                    MeshMaterial3d(halo_mat),
                    Transform::from_translation(p).with_scale(Vec3::splat(r * 5.5)),
                    Billboard,
                    SceneItem,
                ))
                .id();

            let idx = graph.nodes.len();
            let node = commands
                .spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(r)),
                    Breath {
                        base: r,
                        phase: rand_unit(id, 9) * std::f32::consts::TAU,
                        speed: 0.6 + rand01(id, 10) * 0.8,
                    },
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
                        mat: mat.clone(),
                        phase: rand_unit(id, 8) * std::f32::consts::TAU,
                        twinkle: 0.08 + rand01(id, 15) * 0.08,
                    },
                    SceneItem,
                ))
                .id();

            graph.nodes.push(GraphNode {
                entity: node,
                activation: act,
                threshold: thr,
                out: Vec::new(),
            });
            index.insert(id.clone(), idx);

            // Radiating dendrites — clients sprout the most, ledger entries none (keeps it clean).
            let dcount = match kind {
                Kind::Client => 5,
                Kind::Knowledge => 4,
                Kind::Ledger => 0,
            };
            if dcount > 0 {
                commands.spawn((
                    Mesh3d(meshes.add(dendrite_mesh(p, dcount, r, hash_u64(id)))),
                    MeshMaterial3d(dendrite_mat.clone()),
                    Transform::default(),
                    SceneItem,
                ));
            }
            registry.nodes.push(NodeInfo {
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

    // Edges: tapered glass tubes (stronger ones only). Only ever drawn between two nodes in the
    // *same* network — no lines stretch across the void. Clear amber glass: low roughness gives a
    // sharp specular streak under the lights (the round-tube cue), a faint warm tint + low alpha
    // let you see through, emissive keeps a glow.
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.62, 0.28, 0.18),
        emissive: LinearRgba::rgb(0.8, 0.32, 0.08),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.06,
        metallic: 0.0,
        reflectance: 0.7,
        ..default()
    });
    let curve_params = CurveParams {
        samples: 14,
        bow: 0.1,
        sag: 0.03,
        jitter: 0.015,
        seed: 0x00E6,
    };
    let add_tube = |commands: &mut Commands, meshes: &mut Assets<Mesh>, curve: &[Vec3]| {
        let radii = connection_radii(curve.len(), 0.32);
        commands.spawn((
            Mesh3d(meshes.add(tube_mesh(curve, &radii, 8))),
            MeshMaterial3d(edge_mat.clone()),
            Transform::default(),
            SceneItem,
        ));
    };
    // Wire a path into the graph as directed src→dst (and the reverse if undirected) for propagation.
    fn wire(graph: &mut BrainGraph, si: usize, di: usize, curve: &[Vec3], directed: bool) {
        let e1 = graph.edges.len();
        graph.edges.push(GraphEdge {
            path: curve.to_vec(),
            target: di,
        });
        graph.nodes[si].out.push(e1);
        if !directed {
            let mut rev = curve.to_vec();
            rev.reverse();
            let e2 = graph.edges.len();
            graph.edges.push(GraphEdge { path: rev, target: si });
            graph.nodes[di].out.push(e2);
        }
    }

    for e in &snap.edges {
        if e.weight < EDGE_WEIGHT_MIN {
            continue;
        }
        let (Some(&a), Some(&b)) = (pos.get(&e.source_id), pos.get(&e.target_id)) else {
            continue;
        };
        let (Some(&si), Some(&di)) = (index.get(&e.source_id), index.get(&e.target_id)) else {
            continue;
        };
        let net_a = kind_of.get(&e.source_id).map(|k| k.network());
        let net_b = kind_of.get(&e.target_id).map(|k| k.network());
        if net_a != net_b {
            continue;
        }
        let curve = edge_curve(a, b, e.weight as f32, &curve_params, hash_u64(&e.id));
        add_tube(commands, meshes, &curve);
        wire(&mut graph, si, di, &curve, e.directed);
    }

    // The Research network's native links all point across to clients/ledger (dropped above), so
    // it arrives as loose dust. Weave a proximity web — each note to its 2 nearest neighbours — so
    // it reads as its own constellation. (Real Add-Research notes link to topic hubs over time.)
    if let Some(rids) = groups.get(&Network::Research) {
        let ridx: Vec<usize> = rids.iter().map(|id| index[id]).collect();
        let rpos: Vec<Vec3> = rids.iter().map(|id| pos[id]).collect();
        let mut linked: HashSet<(usize, usize)> = HashSet::new();
        for i in 0..rpos.len() {
            let mut nearest: Vec<(f32, usize)> = (0..rpos.len())
                .filter(|&j| j != i)
                .map(|j| (rpos[i].distance_squared(rpos[j]), j))
                .collect();
            nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(_, j) in nearest.iter().take(2) {
                let key = (i.min(j), i.max(j));
                if !linked.insert(key) {
                    continue;
                }
                let seed = (key.0 as u64) << 20 ^ key.1 as u64;
                let curve = edge_curve(rpos[i], rpos[j], 0.7, &curve_params, seed);
                add_tube(commands, meshes, &curve);
                wire(&mut graph, ridx[i], ridx[j], &curve, false);
            }
        }
    }

    // Pulse asset shared by all propagation pulses (spawned at runtime when neurons fire).
    let pulse_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(9.0, 5.0, 1.6),
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    commands.insert_resource(PulseAssets {
        mesh: sphere.clone(),
        material: pulse_material,
    });

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
        let radius = network.radii().max_element() * 1.3;
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
            let sz = 0.6 + lcg(&mut ms) * 1.2;
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

    commands.insert_resource(graph);

    let all: Vec<Vec3> = pos.values().copied().collect();
    let (center, radius) = bounds(&all);
    registry.galaxy_center = center;
    registry.galaxy_radius = radius;
    // Frame the Business network for the opening shot (Research is a distant cluster behind it).
    Some(registry.network_view(Network::Business))
}
