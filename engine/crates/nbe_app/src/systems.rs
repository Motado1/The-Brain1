use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::components::*;
use crate::geometry::*;
use crate::nav::*;
use crate::tuning::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn orbit_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut target: ResMut<CameraTarget>,
    registry: Res<NodeRegistry>,
    ui: Res<UiPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut OrbitCamera, &mut Transform, &Camera, &GlobalTransform)>,
) {
    let orbiting = mouse_buttons.pressed(MouseButton::Left);
    let panning =
        mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Middle);

    let mut drag = Vec2::ZERO;
    if orbiting || panning {
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

    // Pointer over a panel? Don't let sidebar scrolling/dragging move the camera.
    if ui.over {
        drag = Vec2::ZERO;
        scroll = 0.0;
    }

    // Esc unfocuses to the whole-scene view.
    if keys.just_pressed(KeyCode::Escape) {
        target.0 = Some((registry.galaxy_center, registry.galaxy_radius * 1.35));
    }
    // Manual rotate/pan cancels an active fly-to. (Scroll instead *drives* a smooth zoom below.)
    if drag != Vec2::ZERO {
        target.0 = None;
    }

    let cursor = windows.single().ok().and_then(|w| w.cursor_position());

    for (mut orbit, mut transform, camera, cam_global) in &mut query {
        if let Some((focus, radius)) = target.0 {
            let k = (time.delta_secs() * 3.5).min(1.0);
            orbit.focus = orbit.focus.lerp(focus, k);
            orbit.radius += (radius - orbit.radius) * k;
            if orbit.focus.distance(focus) < 0.5 && (orbit.radius - radius).abs() < 0.5 {
                target.0 = None;
            }
        }
        if orbiting {
            orbit.yaw -= drag.x * 0.005;
            orbit.pitch = (orbit.pitch + drag.y * 0.005).clamp(-1.4, 1.4);
        }

        let eye = orbit.eye();
        let radius = orbit.radius;
        let forward = (orbit.focus - eye).normalize_or_zero();
        if panning && drag != Vec2::ZERO {
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            orbit.focus += (-right * drag.x + up * drag.y) * (radius * 0.0015);
        }
        if scroll != 0.0 {
            // Zoom = pull the pivot onto whatever's under the cursor *and* shrink the orbit radius,
            // so after zooming in, rotation orbits the thing you zoomed into (not a stale pivot).
            // We set a fly-to target and let the lerp above ease it smoothly.
            let (cur_focus, cur_radius) = target.0.unwrap_or((orbit.focus, orbit.radius));
            let aim = cursor
                .and_then(|c| camera.viewport_to_world(cam_global, c).ok())
                .map(|ray| aim_point(&registry, ray.origin, ray.direction.as_vec3(), cur_radius))
                .unwrap_or(cur_focus + forward * cur_radius);
            let new_radius = (cur_radius * (1.0 - scroll * 0.18)).clamp(2.0, 6000.0);
            // Zooming in pulls the pivot toward the aim point; zooming out just widens the orbit.
            let new_focus = if scroll > 0.0 {
                cur_focus.lerp(aim, (scroll * 0.35).min(0.7))
            } else {
                cur_focus
            };
            target.0 = Some((new_focus, new_radius));
        }
        orbit.radius = orbit.radius.clamp(1.0, 6000.0);
        *transform = Transform::from_translation(orbit.eye()).looking_at(orbit.focus, Vec3::Y);
    }
}

/// Index of the node nearest the cursor ray, within a thin cone (closest along the ray wins).
fn pick_index(registry: &NodeRegistry, origin: Vec3, dir: Vec3) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (i, n) in registry.nodes.iter().enumerate() {
        let to = n.pos - origin;
        let along = to.dot(dir);
        if along <= 0.0 {
            continue;
        }
        let perp = (to - dir * along).length();
        let nearer = match best {
            Some((a, _)) => along < a,
            None => true,
        };
        if perp < along * 0.05 + 3.0 && nearer {
            best = Some((along, i));
        }
    }
    best.map(|(_, i)| i)
}

/// The node nearest the cursor ray, else a point along the ray at `fallback`. Lets zoom pull the
/// pivot onto whatever you're pointing at so rotation orbits it afterward.
fn aim_point(registry: &NodeRegistry, origin: Vec3, dir: Vec3, fallback: f32) -> Vec3 {
    pick_index(registry, origin, dir)
        .map(|i| registry.nodes[i].pos)
        .unwrap_or(origin + dir * fallback)
}

/// Cursor picking: track the node under the pointer, and on a stationary left-click (not an
/// orbit-drag) select it and cache its detail text for the panel.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pick_node(
    windows: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    registry: Res<NodeRegistry>,
    ui: Res<UiPointer>,
    db_path: Res<DbPath>,
    mut picker: ResMut<Picker>,
) {
    let (Ok(window), Ok((camera, cam_global))) = (windows.single(), cam_q.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        picker.hovered = None;
        return;
    };
    picker.hovered = camera
        .viewport_to_world(cam_global, cursor)
        .ok()
        .and_then(|ray| pick_index(&registry, ray.origin, ray.direction.as_vec3()));

    if ui.over {
        picker.press = None;
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        picker.press = Some(cursor);
    }
    if buttons.just_released(MouseButton::Left) {
        if let Some(press) = picker.press.take() {
            // Only a click (not a drag) selects — drags are camera orbit.
            if press.distance(cursor) < 5.0 {
                picker.selected = picker.hovered;
                picker.detail = match picker.selected {
                    Some(i) => fetch_detail(&db_path.0, &registry.nodes[i].id),
                    None => String::new(),
                };
            }
        }
    }
}

/// Open the DB and render the full detail view for an entity (cached on selection).
fn fetch_detail(path: &str, id: &str) -> String {
    match nbe_data::Db::open(path, None) {
        Ok(db) => {
            nbe_cli::ops::show(&db, id, crate::now_unix()).unwrap_or_else(|e| format!("(error: {e})"))
        }
        Err(e) => format!("(cannot open '{path}': {e})"),
    }
}

pub(crate) fn animate_breath(time: Res<Time>, mut query: Query<(&Breath, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (b, mut transform) in &mut query {
        transform.scale = b.base * (1.0 + 0.1 * (t * b.speed + b.phase).sin());
    }
}

/// Spawn one travelling pulse along directed edge `edge_idx`, coloured for its network. A soft,
/// slow blob of light drifting down the filament.
fn spawn_pulse(commands: &mut Commands, assets: &PulseAssets, graph: &BrainGraph, edge_idx: usize, seed: u64) {
    let edge = &graph.edges[edge_idx];
    // Intra-network edge, so the source and target share a network → colour by the target's.
    let net = graph.nodes[edge.target].network;
    let Some(mat) = assets.material.get(&net) else {
        return;
    };
    let mut s = seed | 1;
    let speed = 0.12 + lcg(&mut s) * 0.12;
    commands.spawn((
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(edge.path[0]),
        Pulse { edge: edge_idx, t: 0.0, speed, target: edge.target, energy: PULSE_ENERGY },
        Billboard,
        SceneItem,
    ));
}

/// Integrate-and-fire: each neuron charges from its activation; when it crosses threshold it flares
/// and tries to emit a pulse down each outgoing edge. Traffic control (one pulse per connection,
/// directional mutex, capped queue) keeps the flow clean — see `EdgeTraffic`.
pub(crate) fn fire_scheduler(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut traffic: ResMut<EdgeTraffic>,
    pulse: Option<Res<PulseAssets>>,
    mut commands: Commands,
    mut q: Query<(&Neuron, &mut Firing)>,
) {
    let dt = time.delta_secs();
    for (neuron, mut firing) in &mut q {
        let Some(node) = graph.nodes.get(neuron.0) else {
            continue;
        };
        firing.intensity *= (-dt * FIRE_DECAY).exp();
        // base ambient liveliness for everyone, plus more the more a neuron actually needs.
        firing.accumulator += (FIRE_BASE + node.activation.max(0.0) * FIRE_NEED) * dt;
        if firing.accumulator >= node.threshold {
            firing.accumulator = 0.0;
            firing.intensity = 1.0;
            if let Some(assets) = &pulse {
                let mut seed = neuron.0 as u64 ^ time.elapsed().as_nanos() as u64;
                for &e in node.out.iter().take(MAX_PULSES_PER_FIRE) {
                    let c = graph.edges[e].channel;
                    let Some(ch) = traffic.channels.get_mut(c) else {
                        continue;
                    };
                    if !ch.busy {
                        // claim the connection and send the pulse.
                        ch.busy = true;
                        spawn_pulse(&mut commands, assets, &graph, e, seed);
                    } else if ch.queue.len() < QUEUE_CAP && !ch.queue.contains(&e) {
                        // connection in use — queue this direction behind the current pulse.
                        ch.queue.push_back(e);
                    }
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                }
            }
        }
    }
}

/// Render firing: flare the neuron's emissive + swell its halo, plus a constant gentle twinkle.
pub(crate) fn fire_render(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<(&Firing, &NodeViz)>,
    mut transforms: Query<&mut Transform>,
) {
    let t = time.elapsed_secs();
    for (firing, viz) in &q {
        let twinkle = 1.0 + (t * 1.7 + viz.phase).sin() * viz.twinkle;
        let mul = twinkle + firing.intensity * FLARE_GAIN;
        if let Some(m) = materials.get_mut(&viz.mat) {
            m.emissive = LinearRgba::rgb(
                viz.base_emissive.red * mul,
                viz.base_emissive.green * mul,
                viz.base_emissive.blue * mul,
            );
        }
        if let Ok(mut tr) = transforms.get_mut(viz.halo) {
            tr.scale = Vec3::splat(viz.base_radius * 3.0 * (1.0 + firing.intensity * HALO_SWELL));
        }
    }
}

/// Move propagation pulses along their edge; on arrival, deposit energy into the target neuron
/// (which may tip it over threshold → it fires → the cascade continues), then despawn.
pub(crate) fn advance_pulses(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut traffic: ResMut<EdgeTraffic>,
    assets: Option<Res<PulseAssets>>,
    mut commands: Commands,
    mut pulses: Query<(Entity, &mut Pulse, &mut Transform)>,
    mut fire: Query<&mut Firing>,
) {
    let dt = time.delta_secs();
    for (ent, mut pulse, mut tr) in &mut pulses {
        let Some(edge) = graph.edges.get(pulse.edge) else {
            commands.entity(ent).despawn();
            continue;
        };
        pulse.t += pulse.speed * dt;
        let tt = pulse.t.min(1.0);
        // Soft round glow that fades in/out along the path (rotation is handled by face_camera, so
        // it always faces us as a shapeless blob — no hard streak).
        tr.translation = sample_path(&edge.path, tt);
        let env = (tt * std::f32::consts::PI).sin().max(0.0);
        tr.scale = Vec3::splat(2.4 * (0.35 + 0.65 * env));
        if pulse.t >= 1.0 {
            if let Some(node) = graph.nodes.get(pulse.target) {
                if let Ok(mut f) = fire.get_mut(node.entity) {
                    f.accumulator += pulse.energy;
                }
            }
            // Release the connection: hand it to the next queued pulse, or free it.
            let channel = edge.channel;
            if let Some(ch) = traffic.channels.get_mut(channel) {
                if let Some(next) = ch.queue.pop_front() {
                    if let Some(assets) = &assets {
                        spawn_pulse(&mut commands, assets, &graph, next, ent.to_bits());
                    }
                } else {
                    ch.busy = false;
                }
            }
            commands.entity(ent).despawn();
        }
    }
}

/// Drift the ambient dust motes slowly, steering them back when they leave their network volume.
pub(crate) fn drift_motes(time: Res<Time>, mut q: Query<(&mut Mote, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut mote, mut tr) in &mut q {
        tr.translation += mote.vel * dt;
        let off = tr.translation - mote.center;
        if off.length() > mote.radius {
            let speed = mote.vel.length().max(0.5);
            mote.vel = -off.normalize_or_zero() * speed;
        }
    }
}

/// Turn the glow-halo quads to face the camera each frame.
pub(crate) fn face_camera(
    cam: Query<&GlobalTransform, With<Camera>>,
    mut halos: Query<&mut Transform, With<Billboard>>,
) {
    let Ok(cam) = cam.single() else {
        return;
    };
    let cam_pos = cam.translation();
    for mut t in &mut halos {
        let dir = (cam_pos - t.translation).normalize_or_zero();
        if dir.length_squared() > 1e-6 {
            t.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
        }
    }
}

/// Keep depth-of-field focused at the current orbit distance so the looked-at cluster stays crisp.
pub(crate) fn update_dof(time: Res<Time>, mut query: Query<(&OrbitCamera, &mut DepthOfField)>) {
    // Ease the focus plane toward the current orbit distance ("focus pulling") so zooming/flying
    // softens foreground & background into bokeh smoothly rather than snapping.
    let k = (time.delta_secs() * 3.0).min(1.0);
    for (orbit, mut dof) in &mut query {
        dof.focal_distance += (orbit.radius - dof.focal_distance) * k;
    }
}

/// Two directional lights from opposite sides — they don't brighten the scene much (nodes are
/// emissive) but give the glass tubes a specular streak so they read as round 3D tubes.
pub(crate) fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.82, 0.55),
            illuminance: 4000.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.2, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.5, 0.28),
            illuminance: 1800.0,
            ..default()
        },
        Transform::from_xyz(-1.0, -0.5, -0.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub(crate) fn setup_hud(mut commands: Commands) {
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

pub(crate) fn update_hud(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<HudText>>) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    for mut text in &mut query {
        text.0 = format!("{fps:5.0} FPS");
    }
}
