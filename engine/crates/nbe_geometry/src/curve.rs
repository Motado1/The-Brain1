//! Organic edge curves — the "straight-ish but alive" synaptic pathway.
//!
//! Each edge becomes a cubic Bézier from `a` to `b` whose control points are nudged
//! perpendicular to the segment (**bow**), drooped downward (**sag**), and lightly perturbed
//! (**jitter**). Endpoints stay exact. The returned sample points double as the positions for
//! a **dotted particle-trail** rendering of the edge (reference image 1).

use glam::Vec3;

use crate::rng::Rng;

/// Tunables for [`edge_curve`]. `bow`/`sag`/`jitter` are fractions of the edge length.
#[derive(Clone, Debug)]
pub struct CurveParams {
    pub samples: usize,
    pub bow: f32,
    pub sag: f32,
    pub jitter: f32,
    pub seed: u64,
}

impl Default for CurveParams {
    fn default() -> Self {
        // Reference-tuned: subtle organic bow, slight droop, faint jitter.
        Self {
            samples: 32,
            bow: 0.18,
            sag: 0.05,
            jitter: 0.01,
            seed: 0x00C0_FFEE,
        }
    }
}

/// Two stable axes perpendicular to `axis`.
fn perp_basis(axis: Vec3) -> (Vec3, Vec3) {
    let mut p = axis.cross(Vec3::Y);
    if p.length_squared() < 1e-6 {
        p = axis.cross(Vec3::X);
    }
    let p = p.normalize_or_zero();
    let q = axis.cross(p).normalize_or_zero();
    (p, q)
}

fn cubic_bezier(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// Sample an organic curve from `a` to `b`. `weight` (0..1) scales the bow so stronger edges
/// arc more. `seed` is mixed with `params.seed` for a per-edge but reproducible shape.
pub fn edge_curve(a: Vec3, b: Vec3, weight: f32, params: &CurveParams, seed: u64) -> Vec<Vec3> {
    let n = params.samples.max(2);
    let dir = b - a;
    let len = dir.length();
    let mut rng = Rng::seeded(params.seed, seed);

    let axis = if len > 1e-6 { dir / len } else { Vec3::X };
    let (perp, perp2) = perp_basis(axis);

    let wfac = 0.5 + 0.5 * weight.clamp(0.0, 1.0);
    let bow_dir = (perp * rng.signed() + perp2 * rng.signed()).normalize_or_zero();
    let bow = bow_dir * (params.bow * len * wfac);
    let sag = Vec3::NEG_Y * (params.sag * len);

    let c1 = a + dir * (1.0 / 3.0) + bow + sag;
    let c2 = a + dir * (2.0 / 3.0) + bow + sag;

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n as f32 - 1.0);
        let mut pt = cubic_bezier(a, c1, c2, b, t);
        // Organic jitter, enveloped to zero at the endpoints.
        let env = (t * std::f32::consts::PI).sin();
        let j = params.jitter * len * env;
        pt += (perp * rng.signed() + perp2 * rng.signed()) * j;
        out.push(pt);
    }

    // Guarantee exact endpoints regardless of jitter.
    out[0] = a;
    out[n - 1] = b;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max_perp_deviation(curve: &[Vec3], a: Vec3, b: Vec3) -> f32 {
        let axis = (b - a).normalize_or_zero();
        curve
            .iter()
            .map(|&p| {
                let v = p - a;
                (v - axis * v.dot(axis)).length()
            })
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn endpoints_are_exact_and_count_matches() {
        let a = Vec3::new(-3.0, 1.0, 0.5);
        let b = Vec3::new(4.0, -2.0, 1.0);
        let c = edge_curve(a, b, 0.7, &CurveParams::default(), 11);
        assert_eq!(c.len(), 32);
        assert_eq!(c[0], a);
        assert_eq!(c[c.len() - 1], b);
    }

    #[test]
    fn straight_when_zeroed() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(10.0, 0.0, 0.0);
        let p = CurveParams {
            bow: 0.0,
            sag: 0.0,
            jitter: 0.0,
            ..Default::default()
        };
        let c = edge_curve(a, b, 1.0, &p, 1);
        assert!(max_perp_deviation(&c, a, b) < 1e-4, "should be collinear");
    }

    #[test]
    fn bows_off_axis() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(10.0, 0.0, 0.0);
        let c = edge_curve(a, b, 1.0, &CurveParams::default(), 5);
        assert!(
            max_perp_deviation(&c, a, b) > 0.1,
            "a bowed curve must leave the straight line"
        );
    }

    #[test]
    fn is_deterministic() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(-4.0, 0.0, 2.0);
        let p = CurveParams::default();
        assert_eq!(edge_curve(a, b, 0.5, &p, 99), edge_curve(a, b, 0.5, &p, 99));
    }
}
