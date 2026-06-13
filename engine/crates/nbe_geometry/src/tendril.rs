//! Mycelial tendrils — fine, tapering, branching filaments that give the network its
//! biological, fungal feel (reference image 2). Each tendril is a polyline plus a per-point
//! width that tapers to a thread at the tip; tendrils recursively branch down to `max_depth`.

use glam::Vec3;

use crate::rng::Rng;

/// Tunables for tendril growth.
#[derive(Clone, Debug)]
pub struct TendrilParams {
    pub count: usize,
    pub segments: usize,
    pub length: f32,
    pub branch_prob: f32,
    pub max_depth: u32,
    /// Direction wander per segment (larger = more curl).
    pub wander: f32,
    pub seed: u64,
}

impl Default for TendrilParams {
    fn default() -> Self {
        // Reference-tuned: several fine, curling, branching filaments.
        Self {
            count: 4,
            segments: 6,
            length: 1.0,
            branch_prob: 0.3,
            max_depth: 3,
            wander: 0.5,
            seed: 0x7A1D_5EED,
        }
    }
}

/// One filament: a polyline with a tapering width per point and its branch depth.
#[derive(Clone, Debug, PartialEq)]
pub struct Tendril {
    pub points: Vec<Vec3>,
    pub widths: Vec<f32>,
    pub depth: u32,
}

/// Width at sample `i` of a `segs`-segment tendril at branch `depth`: thinner per generation,
/// tapering to ~0 at the tip.
fn taper(i: usize, segs: usize, depth: u32) -> f32 {
    let base = 1.0 / (1.0 + depth as f32);
    let t = i as f32 / segs as f32;
    base * (1.0 - t)
}

#[allow(clippy::too_many_arguments)]
fn grow_one(
    start: Vec3,
    dir: Vec3,
    length: f32,
    depth: u32,
    params: &TendrilParams,
    rng: &mut Rng,
    out: &mut Vec<Tendril>,
) {
    let segs = params.segments.max(2);
    let step = length / segs as f32;
    let mut points = Vec::with_capacity(segs + 1);
    let mut widths = Vec::with_capacity(segs + 1);

    let mut pos = start;
    let mut d = dir.normalize_or(Vec3::Y);
    points.push(pos);
    widths.push(taper(0, segs, depth));

    for i in 1..=segs {
        d = (d + rng.unit_vec3() * (params.wander * 0.5)).normalize_or(d);
        pos += d * step;
        points.push(pos);
        widths.push(taper(i, segs, depth));

        if depth < params.max_depth && rng.unit() < params.branch_prob {
            let bdir = (d + rng.unit_vec3() * params.wander).normalize_or(d);
            grow_one(pos, bdir, length * 0.6, depth + 1, params, rng, out);
        }
    }

    out.push(Tendril {
        points,
        widths,
        depth,
    });
}

/// Grow `params.count` root tendrils from `origin`, spreading around `direction`.
pub fn grow_tendrils(
    origin: Vec3,
    direction: Vec3,
    params: &TendrilParams,
    seed: u64,
) -> Vec<Tendril> {
    let mut rng = Rng::seeded(params.seed, seed);
    let dir0 = direction.normalize_or(Vec3::Y);
    let mut out = Vec::new();
    for _ in 0..params.count {
        let d = (dir0 + rng.unit_vec3() * params.wander).normalize_or(dir0);
        grow_one(origin, d, params.length, 0, params, &mut rng, &mut out);
    }
    out
}

/// Grow fine offshoots perpendicular to an edge curve — the mycelial threading along a
/// synapse. Picks `params.count` points spread along `curve` and sprouts a short tendril at
/// each. Every root tendril's first point lies exactly on the supplied curve.
pub fn sprout_along(curve: &[Vec3], params: &TendrilParams, seed: u64) -> Vec<Tendril> {
    if curve.len() < 2 || params.count == 0 {
        return Vec::new();
    }
    let mut rng = Rng::seeded(params.seed ^ 0xA5A5_5A5A, seed);
    let mut out = Vec::new();
    let interior = curve.len().saturating_sub(2).max(1);

    let child = TendrilParams {
        count: 1,
        length: params.length * 0.5,
        ..params.clone()
    };

    for k in 0..params.count {
        let idx = (1 + k * interior / params.count).min(curve.len() - 1);
        let origin = curve[idx];
        let tangent = (curve[idx] - curve[idx - 1]).normalize_or(Vec3::X);

        let mut perp = tangent.cross(Vec3::Y);
        if perp.length_squared() < 1e-6 {
            perp = tangent.cross(Vec3::X);
        }
        perp = perp.normalize_or_zero();
        let perp2 = tangent.cross(perp).normalize_or_zero();
        let dir = (perp * rng.signed() + perp2 * rng.signed()).normalize_or(perp);

        let mut grown = grow_tendrils(origin, dir, &child, seed ^ (k as u64).wrapping_mul(0x9E37_79B9));
        out.append(&mut grown);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_finite(t: &Tendril) -> bool {
        t.points.iter().all(|p| p.is_finite()) && t.widths.iter().all(|w| w.is_finite())
    }

    #[test]
    fn root_count_and_all_finite() {
        let ts = grow_tendrils(Vec3::ZERO, Vec3::Y, &TendrilParams::default(), 1);
        let roots = ts.iter().filter(|t| t.depth == 0).count();
        assert_eq!(roots, 4, "one root per `count`");
        assert!(ts.iter().all(all_finite), "no NaN/inf coordinates");
    }

    #[test]
    fn widths_taper_along_each_tendril() {
        let ts = grow_tendrils(Vec3::ZERO, Vec3::Y, &TendrilParams::default(), 2);
        for t in &ts {
            for w in t.widths.windows(2) {
                assert!(w[1] <= w[0] + 1e-6, "width must not increase toward the tip");
            }
        }
    }

    #[test]
    fn depth_is_bounded() {
        let p = TendrilParams {
            max_depth: 2,
            branch_prob: 0.9, // force lots of branching
            ..Default::default()
        };
        let ts = grow_tendrils(Vec3::ZERO, Vec3::Y, &p, 3);
        assert!(ts.iter().all(|t| t.depth <= 2));
        assert!(ts.iter().any(|t| t.depth > 0), "high branch_prob should branch");
    }

    #[test]
    fn is_deterministic() {
        let p = TendrilParams::default();
        assert_eq!(
            grow_tendrils(Vec3::ZERO, Vec3::Y, &p, 9),
            grow_tendrils(Vec3::ZERO, Vec3::Y, &p, 9)
        );
    }

    #[test]
    fn sprouts_originate_on_curve() {
        let curve: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let ts = sprout_along(&curve, &TendrilParams::default(), 4);
        assert!(!ts.is_empty());
        for t in ts.iter().filter(|t| t.depth == 0) {
            assert!(
                curve.contains(&t.points[0]),
                "each offshoot root must start on the curve"
            );
        }
    }
}
