use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::domain::*;

pub(crate) fn hash_u64(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Deterministic value in [-1, 1] from an id + salt.
pub(crate) fn rand_unit(id: &str, salt: u64) -> f32 {
    let mut h = hash_u64(id) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 32;
    (h >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0
}

pub(crate) fn rand01(id: &str, salt: u64) -> f32 {
    rand_unit(id, salt) * 0.5 + 0.5
}

/// Evenly distributed direction on the unit sphere (Fibonacci).
pub(crate) fn fib_dir(i: usize, n: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * ((i as f32 + 0.5) / n as f32);
    let r = (1.0 - y * y).max(0.0).sqrt();
    let th = i as f32 * golden;
    Vec3::new(r * th.cos(), y, r * th.sin())
}

/// Ellipsoid extents for a network of `n` nodes. Radius ∝ n^(1/3) keeps node spacing — and thus
/// the woven-mesh density — constant no matter how many nodes a network has, so a 35-node cluster
/// and a 350-node cluster read identically (just different overall size).
pub(crate) fn density_radii(n: usize) -> Vec3 {
    (n.max(1) as f32).cbrt() * Vec3::new(32.6, 24.8, 32.6)
}

/// Place a node inside its network's ellipsoid, biased toward its kind's shell radius.
pub(crate) fn net_pos(center: Vec3, radii: Vec3, kind: Kind, i: usize, n: usize, id: &str) -> Vec3 {
    let dir = fib_dir(i, n);
    // jitter proportional to size so the scatter looks the same organic amount at any scale.
    let jitter = Vec3::new(rand_unit(id, 12), rand_unit(id, 13), rand_unit(id, 14)) * (radii.x * 0.1);
    let shell = kind.shell();
    let rr = shell + (1.0 - shell) * rand01(id, 11);
    center + (dir * rr) * radii + jitter
}

pub(crate) fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

pub(crate) fn sample_path(points: &[Vec3], t: f32) -> Vec3 {
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

/// Emissive (HDR) for a node: a *subtle* cool glow at rest, brightening toward a white-cyan
/// hotspot as it activates. Kept dim so most neurons sit quiet and only the active few stand out.
pub(crate) fn node_emissive(kind: Kind, activation: f32) -> LinearRgba {
    let (br, bg, bb) = kind.base_color();
    let (hr, hg, hb) = (1.0, 1.0, 1.0); // white-hot core, applied gently so the hue is preserved
    let t = activation.clamp(0.0, 1.0);
    // Only a little desaturation toward white as it activates — keeps the hue.
    let mix = |b: f32, h: f32| b + (h - b) * t * 0.25;
    // Warm hues carry two bright channels (R+G), so they bloom to white when too intense; cool
    // purple is single-channel-dominant and stays saturated. Cap the warm cores lower so amber
    // blooms a soft *orange* instead of a solid white blob, while purple keeps its current peak.
    let peak = match kind {
        Kind::Knowledge => 2.0,
        Kind::Client | Kind::Ledger => 1.2,
    };
    let intensity = 0.25 + activation * peak;
    LinearRgba::rgb(
        mix(br, hr) * intensity,
        mix(bg, hg) * intensity,
        mix(bb, hb) * intensity,
    )
}

pub(crate) fn bounds(points: &[Vec3]) -> (Vec3, f32) {
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

/// Accumulating mesh buffers so many tube strips can share one mesh (one draw call per neuron's
/// whole dendrite tree, say).
#[derive(Default)]
pub(crate) struct TubeBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl TubeBuilder {
    /// Sweep a tube along `points` with a per-point `radii` profile (lets tubes taper / bulge).
    pub(crate) fn add(&mut self, points: &[Vec3], radii: &[f32], sides: usize) {
        if points.len() < 2 {
            return;
        }
        let rings = points.len();
        let base = self.positions.len() as u32;
        for (ri, &p) in points.iter().enumerate() {
            let tangent = if ri == 0 {
                points[1] - points[0]
            } else if ri == rings - 1 {
                points[ri] - points[ri - 1]
            } else {
                points[ri + 1] - points[ri - 1]
            }
            .normalize_or_zero();
            let t = if tangent.length_squared() < 1e-6 {
                Vec3::Z
            } else {
                tangent
            };
            let up = if t.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            let n0 = t.cross(up).normalize();
            let b0 = t.cross(n0).normalize();
            let r = radii[ri.min(radii.len() - 1)];
            for s in 0..sides {
                let a = s as f32 / sides as f32 * std::f32::consts::TAU;
                let dir = n0 * a.cos() + b0 * a.sin();
                self.positions.push((p + dir * r).to_array());
                self.normals.push(dir.to_array());
                self.uvs
                    .push([ri as f32 / (rings - 1) as f32, s as f32 / sides as f32]);
            }
        }
        for ri in 0..rings - 1 {
            for s in 0..sides {
                let s2 = (s + 1) % sides;
                let a = base + (ri * sides + s) as u32;
                let b = base + (ri * sides + s2) as u32;
                let c = base + ((ri + 1) * sides + s) as u32;
                let d = base + ((ri + 1) * sides + s2) as u32;
                self.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }

    pub(crate) fn build(self) -> Mesh {
        let usages = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
        Mesh::new(PrimitiveTopology::TriangleList, usages)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Single tapered tube as its own mesh.
pub(crate) fn tube_mesh(points: &[Vec3], radii: &[f32], sides: usize) -> Mesh {
    let mut b = TubeBuilder::default();
    b.add(points, radii, sides);
    b.build()
}

/// Connection profile: fat at both endpoints (where it meets the somas), pinched in the middle —
/// the organic dendrite-junction look rather than an even pipe.
/// Axon profile: flares thick where it meets each soma (so the connection looks like it grows out
/// of the cell body), pinching to a thin waist along its length. Ends may differ (each soma's size).
pub(crate) fn axon_radii(n: usize, r_start: f32, r_end: f32, waist: f32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            let end_r = if t < 0.5 { r_start } else { r_end };
            let k = (2.0 * t - 1.0).abs().powf(1.6); // ~1 at the ends, ~0 mid — flare hugs the soma
            waist + (end_r - waist) * k
        })
        .collect()
}

/// Dendrite profile: thick at the root, tapering smoothly to a hair-thin tip.
pub(crate) fn dendrite_radii(n: usize, base: f32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            base * (1.0 - t).powf(1.5) + 0.012
        })
        .collect()
}

/// A dim, hair-thin tangle of background micro-fibers filling a network's volume — non-interactive
/// "noise" that simulates infinite biological depth and scale contrast. Many filaments are merged
/// into one mesh (one draw call).
pub(crate) fn background_fibers_mesh(center: Vec3, radii: Vec3, count: usize, seed: u64) -> Mesh {
    let mut s = seed | 1;
    let mut builder = TubeBuilder::default();
    let thin = radii.x * 0.0016; // hair-thin relative to the cluster
    for _ in 0..count {
        let spread = 0.3 + lcg(&mut s) * 1.1;
        let offset = Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5) * 2.0;
        let mut p = center + offset * radii * spread;
        let segs = 6 + (lcg(&mut s) * 6.0) as usize;
        let seg_len = radii.x * (0.05 + lcg(&mut s) * 0.12);
        let mut dir =
            Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5).normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            dir = Vec3::Y;
        }
        let mut pts = vec![p];
        for _ in 0..segs {
            let j = Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5) * 0.5;
            dir = (dir + j).normalize_or_zero();
            p += dir * seg_len;
            pts.push(p);
        }
        let prof = vec![thin; pts.len()];
        builder.add(&pts, &prof, 4);
    }
    builder.build()
}

/// Grow a small tree of wandering, tapering filaments out of a neuron — the dendrites that make it
/// read as a living cell instead of a dot. Returns one merged mesh.
pub(crate) fn dendrite_mesh(node: Vec3, count: usize, node_r: f32, seed: u64) -> Mesh {
    let mut s = seed | 1;
    let mut builder = TubeBuilder::default();
    for _ in 0..count {
        let segs = 5 + (lcg(&mut s) * 3.0) as usize;
        let seg_len = node_r * (1.4 + lcg(&mut s) * 1.2);
        let mut dir =
            Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5).normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            dir = Vec3::Y;
        }
        let mut p = node;
        let mut pts = vec![p];
        for _ in 0..segs {
            let jitter =
                Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5) * 0.55;
            dir = (dir + jitter).normalize_or_zero();
            p += dir * seg_len;
            pts.push(p);
        }
        let radii = dendrite_radii(pts.len(), node_r * 0.12);
        builder.add(&pts, &radii, 5);
    }
    builder.build()
}

/// A soft round radial-gradient texture (white core fading to transparent) for the glow halos.
pub(crate) fn glow_texture() -> Image {
    const N: usize = 64;
    let mut data = vec![0u8; N * N * 4];
    let c = (N as f32 - 1.0) * 0.5;
    for y in 0..N {
        for x in 0..N {
            let d = (((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt()) / c;
            // Smooth falloff, squared for a tighter hot core.
            let a = (1.0 - d).clamp(0.0, 1.0).powf(2.2);
            let i = (y * N + x) * 4;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}
