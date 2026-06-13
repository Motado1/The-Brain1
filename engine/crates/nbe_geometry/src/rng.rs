//! Tiny deterministic PRNG (splitmix64). No external dependency, stable across platforms —
//! so geometry is byte-reproducible from a seed (the determinism the renderer relies on for
//! stable visuals frame-to-frame and run-to-run).

use glam::Vec3;

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Mix two seeds into a fresh stream (used to derive per-edge / per-tendril streams).
    pub fn seeded(a: u64, b: u64) -> Self {
        Self::new(a ^ b.wrapping_mul(GAMMA))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// f32 in `[0, 1)` with 24 bits of precision.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// f32 in `[-1, 1)`.
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// A uniformly distributed unit vector on the sphere.
    pub fn unit_vec3(&mut self) -> Vec3 {
        let z = self.signed();
        let t = self.unit() * std::f32::consts::TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * t.cos(), r * t.sin(), z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        // a different seed diverges
        let mut c = Rng::new(43);
        assert_ne!(Rng::new(42).next_u64(), c.next_u64());
    }

    #[test]
    fn ranges_hold() {
        let mut r = Rng::new(7);
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for _ in 0..10_000 {
            let u = r.unit();
            assert!((0.0..1.0).contains(&u));
            let s = r.signed();
            lo = lo.min(s);
            hi = hi.max(s);
            let v = r.unit_vec3();
            assert!((v.length() - 1.0).abs() < 1e-3, "unit vec length ~1");
        }
        assert!(lo < -0.9 && hi > 0.9, "signed() should span [-1,1]");
    }
}
