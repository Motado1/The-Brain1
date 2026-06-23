//! Level-of-detail anchor: turns the camera's distance into a single, cubic-smoothed `detail` value
//! that every cosmic-scaling decision (tier reveal, alpha fade, size-clamp dust, label style) reads
//! from. Kept tiny and pure so the math is unit-tested headless; the visual application lives in the
//! apply systems built on top of this.

use bevy::prelude::*;

use crate::components::{DendriteFine, LodReveal, OrbitCamera};
use crate::nav::NodeRegistry;
use crate::shaders::DendriteMaterial;
use crate::tuning::{LOD_GALACTIC_DIST, LOD_MICRO_DIST, TUBE_RIM_ALPHA};

/// Continuous detail factor in `[0,1]` from a camera→target distance: `1` at/under `micro` (deep
/// focus), `0` at/beyond `galactic` (zoomed out), **cubic-smoothstepped** between so tiers ease in
/// and out instead of popping. This is the single anchor all LOD reveal/scale/label logic reads.
pub(crate) fn detail_for_distance(dist: f32, micro: f32, galactic: f32) -> f32 {
    if galactic <= micro {
        return if dist <= micro { 1.0 } else { 0.0 };
    }
    let t = ((galactic - dist) / (galactic - micro)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t) // smoothstep (cubic ease) — no hard edge at the band boundaries
}

/// The three cosmic bands, derived from the global zoom detail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LodBand {
    /// Zoomed out: somas + primary trunks only; labels as initials.
    Galactic,
    /// Mid: child branches + fine twigs fading in.
    Planetary,
    /// Deep focus: finest twigs + terminal data nodes materialised.
    Micro,
}

/// Live LOD state, recomputed every frame from the camera.
#[derive(Resource, Default)]
pub(crate) struct LodState {
    /// Global band detail from the orbit *radius*: `0` = galactic, `1` = micro.
    pub(crate) zoom: f32,
    /// Nearest registry node to the camera *eye* (the one being dived into), if any.
    pub(crate) focus: Option<usize>,
    /// That focus node's own distance-based detail (drives its micro-anatomy reveal).
    pub(crate) focus_detail: f32,
}

impl LodState {
    pub(crate) fn band(&self) -> LodBand {
        if self.zoom < 0.34 {
            LodBand::Galactic
        } else if self.zoom < 0.67 {
            LodBand::Planetary
        } else {
            LodBand::Micro
        }
    }
}

/// Anchor system: read the camera once and populate [`LodState`]. The global band comes from the
/// orbit *radius* (how zoomed in we are); the focus node is the nearest registry node to the camera
/// *eye*, carrying its own distance-based detail for local micro-anatomy reveal.
pub(crate) fn compute_lod(
    cam: Query<&OrbitCamera>,
    registry: Res<NodeRegistry>,
    mut lod: ResMut<LodState>,
) {
    let Ok(cam) = cam.single() else {
        return;
    };
    lod.zoom = detail_for_distance(cam.radius, LOD_MICRO_DIST, LOD_GALACTIC_DIST);

    let eye = cam.eye();
    let mut best: Option<(usize, f32)> = None;
    for (i, n) in registry.nodes.iter().enumerate() {
        let d = eye.distance(n.pos);
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    match best {
        Some((i, d)) => {
            lod.focus = Some(i);
            lod.focus_detail = detail_for_distance(d, LOD_MICRO_DIST, LOD_GALACTIC_DIST);
        }
        None => {
            lod.focus = None;
            lod.focus_detail = 0.0;
        }
    }
}

/// Reveal data-anatomy elements by scaling them `0 → base_scale` (cubic-smoothed) as the global zoom
/// detail crosses each element's `[start, full]` window — the finest data nodes materialise on deep
/// zoom and shrink away (to a shimmering speck, then nothing) when pulled back. Hidden entirely once
/// imperceptibly small, to skip their draw cost in the galactic view.
pub(crate) fn apply_lod_reveal(
    lod: Res<LodState>,
    mut q: Query<(&LodReveal, &mut Transform, &mut Visibility)>,
) {
    for (reveal, mut tf, mut vis) in &mut q {
        let t = if reveal.full <= reveal.start {
            if lod.zoom >= reveal.full { 1.0 } else { 0.0 }
        } else {
            let u = ((lod.zoom - reveal.start) / (reveal.full - reveal.start)).clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u)
        };
        tf.scale = Vec3::splat(reveal.base_scale * t);
        *vis = if t < 0.01 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// Cubic-smoothed reveal weight in `[0,1]` for a `[start, full]` window on the zoom detail — `0` at/
/// below `start`, `1` at/above `full`, eased between. Shared by the dendrite + (conceptually) the
/// anatomy reveals so the macro→micro transition is one consistent ease.
pub(crate) fn reveal_weight(zoom: f32, start: f32, full: f32) -> f32 {
    if full <= start {
        return if zoom >= full { 1.0 } else { 0.0 };
    }
    let u = ((zoom - start) / (full - start)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Reveal the finer-dendrite tier by global zoom: fade its membrane rim alpha `0 → full` across each
/// branch's `[start, full]` window and fully hide it below `start`, so the galactic view is clean
/// trunks-only and the fractal thicket blooms back in on approach. The trunk tier is a separate,
/// always-drawn mesh and is untouched. Pairs with `drive_dendrite_waves` (which writes the firing
/// surge into `wave`) — this only touches `rest.z`, so the two never fight.
pub(crate) fn apply_dendrite_lod(
    lod: Res<LodState>,
    mut mats: ResMut<Assets<DendriteMaterial>>,
    mut q: Query<(&DendriteFine, &mut Visibility)>,
) {
    for (fine, mut vis) in &mut q {
        let t = reveal_weight(lod.zoom, fine.start, fine.full);
        if let Some(m) = mats.get_mut(&fine.mat) {
            m.rest.z = TUBE_RIM_ALPHA * t;
        }
        // Cull entirely once imperceptible — skips its draw cost galactic and beats the shader's small
        // baseline-alpha floor (and any in-flight firing flash) so the thicket truly vanishes.
        let want = if t < 0.02 { Visibility::Hidden } else { Visibility::Inherited };
        if *vis != want {
            *vis = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_endpoints_and_monotonicity() {
        let (micro, gal) = (60.0, 900.0);
        assert_eq!(detail_for_distance(10.0, micro, gal), 1.0, "inside micro = full detail");
        assert_eq!(detail_for_distance(2000.0, micro, gal), 0.0, "beyond galactic = none");
        assert_eq!(detail_for_distance(micro, micro, gal), 1.0);
        assert_eq!(detail_for_distance(gal, micro, gal), 0.0);
        // Smoothstep midpoint is exactly 0.5, and detail strictly decreases with distance.
        let mid = (micro + gal) / 2.0;
        assert!((detail_for_distance(mid, micro, gal) - 0.5).abs() < 1e-5);
        let mut prev = 1.1;
        for k in 0..=20 {
            let d = micro + (gal - micro) * (k as f32 / 20.0);
            let v = detail_for_distance(d, micro, gal);
            assert!(v <= prev + 1e-6, "detail must not increase with distance");
            prev = v;
        }
    }

    #[test]
    fn fine_dendrites_reveal_across_their_window() {
        // Below `start` → fully culled (0); at/above `full` → fully present (1); strictly rising and
        // smooth (midpoint = 0.5) between, so the thicket eases in on approach instead of popping.
        let (start, full) = (0.30, 0.62);
        assert_eq!(reveal_weight(0.0, start, full), 0.0, "galactic = no fine branches");
        assert_eq!(reveal_weight(start, start, full), 0.0);
        assert_eq!(reveal_weight(0.2, start, full), 0.0, "below start stays culled");
        assert_eq!(reveal_weight(1.0, start, full), 1.0, "micro = full thicket");
        assert_eq!(reveal_weight(full, start, full), 1.0);
        assert!((reveal_weight((start + full) / 2.0, start, full) - 0.5).abs() < 1e-5);
        let mut prev = -0.1;
        for k in 0..=20 {
            let z = k as f32 / 20.0;
            let v = reveal_weight(z, start, full);
            assert!(v + 1e-6 >= prev, "reveal must rise with zoom-in");
            prev = v;
        }
    }

    #[test]
    fn bands_follow_zoom() {
        let s = |zoom| LodState { zoom, ..Default::default() }.band();
        assert_eq!(s(0.0), LodBand::Galactic);
        assert_eq!(s(0.5), LodBand::Planetary);
        assert_eq!(s(1.0), LodBand::Micro);
    }
}
