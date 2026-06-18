use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use std::collections::HashMap;

use crate::components::*;
use crate::geometry::*;
use crate::interaction::Dissolving;
use crate::lod::{LodBand, LodState};
use crate::nav::*;
use crate::shaders::PulseWaveMaterial;
use crate::tuning::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn orbit_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut target: ResMut<CameraTarget>,
    mut glide: ResMut<CameraGlide>,
    omni: Res<OmniSearch>,
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

    // Esc unfocuses to the whole-scene view — unless the omni-search overlay is open, where Esc
    // closes the overlay instead (handled in the egui pass) and shouldn't also move the camera.
    if keys.just_pressed(KeyCode::Escape) && !omni.open {
        glide.to(registry.galaxy_center, registry.galaxy_radius * 1.35);
    }
    // Manual rotate/pan/scroll cancels any in-flight cinematic glide or zoom target — the user has
    // taken back control. (Scroll then *drives* the smooth zoom below.)
    if drag != Vec2::ZERO || scroll != 0.0 {
        glide.request = None;
        glide.flight = None;
    }
    if drag != Vec2::ZERO {
        target.0 = None;
    }

    let cursor = windows.single().ok().and_then(|w| w.cursor_position());

    for (mut orbit, mut transform, camera, cam_global) in &mut query {
        // Begin a cinematic glide if one was requested (capture the current camera as the start).
        if let Some((end_focus, end_radius)) = glide.request.take() {
            glide.flight = Some(Flight {
                start_focus: orbit.focus,
                start_radius: orbit.radius,
                end_focus,
                end_radius,
                t: 0.0,
                dur: GLIDE_SECS,
            });
            target.0 = None; // a glide supersedes the zoom target
        }
        if let Some(f) = glide.flight.as_mut() {
            // Fixed-duration cubic ease-in/out — accelerate out, decelerate into frame.
            f.t = (f.t + time.delta_secs() / f.dur).min(1.0);
            let e = ease_in_out_cubic(f.t);
            orbit.focus = f.start_focus.lerp(f.end_focus, e);
            orbit.radius = f.start_radius + (f.end_radius - f.start_radius) * e;
            if f.t >= 1.0 {
                glide.flight = None;
            }
        } else if let Some((focus, radius)) = target.0 {
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
    picker.hovered_entity = picker.hovered.map(|i| registry.nodes[i].entity);

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
                picker.selected_entity = picker.hovered_entity;
                picker.detail = match picker.selected {
                    Some(i) => fetch_detail(&db_path.0, &registry.nodes[i].id),
                    None => String::new(),
                };
            }
        }
    }
}

/// Open the DB and render the full detail view for an entity (cached on selection).
pub(crate) fn fetch_detail(path: &str, id: &str) -> String {
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

/// Spawn one travelling pulse along directed edge `edge_idx`. The pulse is now a *logical* timeline
/// (no visible mesh) — `drive_pulse_waves` reads its `t` to surge the Gaussian wave along the tube.
fn spawn_pulse(commands: &mut Commands, graph: &BrainGraph, edge_idx: usize, seed: u64) {
    let Some(edge) = graph.edges.get(edge_idx) else {
        return;
    };
    let mut s = seed | 1;
    let speed = PULSE_SPEED * (0.85 + lcg(&mut s) * 0.3);
    commands.spawn((
        Pulse {
            edge: edge_idx,
            t: 0.0,
            speed,
            target: edge.target,
            energy: PULSE_ENERGY,
            arrived: false,
            fade: 0.0,
        },
        SceneItem,
    ));
}

/// Cubic smoothstep on a clamped 0..1 input — eases the pulse's arrival fade.
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Count down each connection's rest period; once a channel has rested and is free, release the next
/// queued pulse (typically the reply heading back the other way). This is what gives the
/// absorb → pause → reply-back rhythm instead of pulses ping-ponging instantly.
pub(crate) fn tick_channels(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut traffic: ResMut<EdgeTraffic>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let mut seed = time.elapsed().as_nanos() as u64;
    for c in 0..traffic.channels.len() {
        let ch = &mut traffic.channels[c];
        if ch.cooldown > 0.0 {
            ch.cooldown -= dt;
            continue; // still resting
        }
        if !ch.busy {
            if let Some(next) = ch.queue.pop_front() {
                ch.busy = true;
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                spawn_pulse(&mut commands, &graph, next, seed);
            }
        }
    }
}

/// Drive the continuous Gaussian energy wave along each connection from its active pulse timeline.
/// Only materials that are active (or just went idle) are touched, so idle tubes aren't re-uploaded.
pub(crate) fn drive_pulse_waves(
    graph: Res<BrainGraph>,
    pulses: Query<&Pulse>,
    conns: Query<&ConnectionWave>,
    mut mats: ResMut<Assets<PulseWaveMaterial>>,
    mut active: ResMut<WaveActive>,
) {
    let by_channel: HashMap<usize, &ConnectionWave> = conns.iter().map(|c| (c.channel, c)).collect();
    // Channel → (wave centre, amplitude) this frame (centre oriented to the tube's uv via fwd edge).
    let mut now: HashMap<usize, (f32, f32)> = HashMap::new();
    for p in &pulses {
        let Some(edge) = graph.edges.get(p.edge) else {
            continue;
        };
        let Some(cw) = by_channel.get(&edge.channel) else {
            continue;
        };
        let tt = p.t.clamp(0.0, 1.0);
        let t_center = if p.edge == cw.fwd_edge { tt } else { 1.0 - tt };
        // Once it has arrived the crest sits on the node end and its glow eases out (no hard cutoff).
        let amp = PULSE_WAVE_AMP * if p.arrived { 1.0 - smoothstep(p.fade) } else { 1.0 };
        now.insert(edge.channel, (t_center, amp));
    }
    for (&ch, &(t_center, amp)) in &now {
        if let Some(cw) = by_channel.get(&ch) {
            if let Some(m) = mats.get_mut(&cw.mat) {
                m.wave = Vec4::new(t_center, amp, PULSE_WIDTH, 0.0);
            }
        }
    }
    // Reset connections that were waving last frame but aren't now.
    for ch in active.0.iter() {
        if !now.contains_key(ch) {
            if let Some(cw) = by_channel.get(ch) {
                if let Some(m) = mats.get_mut(&cw.mat) {
                    m.wave = Vec4::new(0.0, 0.0, PULSE_WIDTH, 0.0);
                }
            }
        }
    }
    active.0 = now.keys().copied().collect();
}

/// Integrate-and-fire: each neuron charges from its activation; when it crosses threshold it flares
/// and tries to emit a pulse down each outgoing edge. Traffic control (one pulse per connection,
/// directional mutex, capped queue) keeps the flow clean — see `EdgeTraffic`.
pub(crate) fn fire_scheduler(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut traffic: ResMut<EdgeTraffic>,
    mut commands: Commands,
    mut q: Query<(&Neuron, &mut Firing), Without<Dissolving>>,
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
            let mut seed = neuron.0 as u64 ^ time.elapsed().as_nanos() as u64;
            for &e in node.out.iter().take(MAX_PULSES_PER_FIRE) {
                let c = graph.edges[e].channel;
                let Some(ch) = traffic.channels.get_mut(c) else {
                    continue;
                };
                if !ch.busy && ch.cooldown <= 0.0 {
                    // claim the rested connection and send the pulse.
                    ch.busy = true;
                    spawn_pulse(&mut commands, &graph, e, seed);
                } else if ch.queue.len() < QUEUE_CAP && !ch.queue.contains(&e) {
                    // connection in use or resting — queue this direction; it releases after cooldown.
                    ch.queue.push_back(e);
                }
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            }
        }
    }
}

/// Drive each dendrite tree's surge: when its soma fires (a rising edge in `Firing.intensity`) the
/// wave restarts at the root and travels out past the tips, then idles. Only active trees are
/// touched, so idle dendrite materials aren't re-uploaded.
pub(crate) fn drive_dendrite_waves(
    time: Res<Time>,
    firings: Query<&Firing>,
    mut dends: Query<&mut DendriteWave>,
    mut mats: ResMut<Assets<PulseWaveMaterial>>,
) {
    let dt = time.delta_secs();
    // Let the crest fully clear the farthest tip (u=1) plus a few sigma before going idle.
    let end = 1.0 + PULSE_WIDTH * 3.0;
    for mut dw in &mut dends {
        let intensity = firings.get(dw.soma).map(|f| f.intensity).unwrap_or(0.0);
        // Rising edge → the soma just fired → (re)start the surge from the root.
        if intensity - dw.prev_intensity > 0.3 {
            dw.t = 0.0;
        }
        dw.prev_intensity = intensity;
        if dw.t <= end {
            dw.t += DEND_WAVE_SPEED * dt;
            let amp = if dw.t <= end { DEND_WAVE_AMP } else { 0.0 };
            if let Some(m) = mats.get_mut(&dw.mat) {
                m.wave = Vec4::new(dw.t, amp, PULSE_WIDTH, 0.0);
            }
        }
    }
}

/// Render firing: flare the neuron's emissive + swell its halo, plus a constant gentle twinkle.
/// The hovered/selected neuron gets an extra steady boost so you can see what you're pointing at.
pub(crate) fn fire_render(
    time: Res<Time>,
    picker: Res<Picker>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<(Entity, &Firing, &NodeViz), Without<Dissolving>>,
    mut transforms: Query<&mut Transform>,
) {
    let t = time.elapsed_secs();
    for (entity, firing, viz) in &q {
        let (glow_boost, halo_boost) = if picker.selected_entity == Some(entity) {
            (2.4, 1.6)
        } else if picker.hovered_entity == Some(entity) {
            (1.5, 1.25)
        } else {
            (1.0, 1.0)
        };
        let twinkle = 1.0 + (t * 1.7 + viz.phase).sin() * viz.twinkle;
        let mul = (twinkle + firing.intensity * FLARE_GAIN) * glow_boost;
        // The nucleus is an additive (unlit) billboard, so its brightness is its base_color.
        if let Some(m) = materials.get_mut(&viz.mat) {
            m.base_color = Color::LinearRgba(LinearRgba::new(
                viz.base_emissive.red * mul,
                viz.base_emissive.green * mul,
                viz.base_emissive.blue * mul,
                1.0,
            ));
        }
        if let Ok(mut tr) = transforms.get_mut(viz.halo) {
            let swell = viz.base_radius * 3.0 * (1.0 + firing.intensity * HALO_SWELL) * halo_boost;
            tr.scale = Vec3::splat(swell);
        }
    }
}

/// Move propagation pulses along their edge; on arrival, deposit energy into the target neuron
/// (which may tip it over threshold → it fires → the cascade continues), then despawn.
pub(crate) fn advance_pulses(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut traffic: ResMut<EdgeTraffic>,
    mut commands: Commands,
    mut pulses: Query<(Entity, &mut Pulse)>,
    mut fire: Query<&mut Firing>,
) {
    let dt = time.delta_secs();
    for (ent, mut pulse) in &mut pulses {
        let Some(edge) = graph.edges.get(pulse.edge) else {
            commands.entity(ent).despawn();
            continue;
        };
        if pulse.arrived {
            // Crest has reached the node — ease its glow out (drive_pulse_waves), then despawn.
            pulse.fade += dt / PULSE_FADE_TIME;
            if pulse.fade >= 1.0 {
                commands.entity(ent).despawn();
            }
            continue;
        }
        pulse.t += pulse.speed * dt;
        if pulse.t >= 1.0 {
            pulse.t = 1.0;
            pulse.arrived = true;
            // Deposit the energy into the receiving node (done once, on contact).
            if let Some(node) = graph.nodes.get(pulse.target) {
                if let Ok(mut f) = fire.get_mut(node.entity) {
                    f.accumulator += pulse.energy;
                }
            }
            // Rest the connection for CHANNEL_COOLDOWN before any queued (reverse) pulse is released
            // (see tick_channels) — no ping-pong. The fade-out runs inside this rest window, so the
            // connection isn't reclaimed mid-fade (PULSE_FADE_TIME < CHANNEL_COOLDOWN).
            if let Some(ch) = traffic.channels.get_mut(edge.channel) {
                ch.busy = false;
                ch.cooldown = CHANNEL_COOLDOWN;
            }
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
        // Neutral key/fill so the tissue's colour comes from the cores it's lit by, not the light.
        DirectionalLight {
            color: Color::srgb(0.92, 0.95, 1.0),
            illuminance: 4000.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.2, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.78, 0.83, 0.95),
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

pub(crate) fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    lod: Res<LodState>,
    registry: Res<NodeRegistry>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let band = match lod.band() {
        LodBand::Galactic => "Galactic",
        LodBand::Planetary => "Planetary",
        LodBand::Micro => "Micro",
    };
    // Live LOD readout so the camera-distance anchor is verifiable at a glance while flying.
    let focus = lod
        .focus
        .and_then(|i| registry.nodes.get(i))
        .map(|n| format!("  →  {} ({:.2})", n.name, lod.focus_detail))
        .unwrap_or_default();
    for mut text in &mut query {
        text.0 = format!("{fps:5.0} FPS   LOD {band} {:.2}{focus}", lod.zoom);
    }
}
