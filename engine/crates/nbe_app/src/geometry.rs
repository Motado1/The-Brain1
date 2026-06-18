use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
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
    (n.max(1) as f32).cbrt() * Vec3::new(45.0, 34.0, 45.0)
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
    /// Sweep a tube along `points` with a per-point `radii` profile (lets tubes taper / bulge). The
    /// length coordinate `uv.x` runs 0→1 evenly along the polyline.
    pub(crate) fn add(&mut self, points: &[Vec3], radii: &[f32], sides: usize) {
        let rings = points.len();
        let u: Vec<f32> = (0..rings)
            .map(|ri| ri as f32 / (rings - 1).max(1) as f32)
            .collect();
        self.add_with_u(points, radii, sides, &u);
    }

    /// Like `add`, but takes an explicit per-ring length coordinate `u` (written to `uv.x`). Lets a
    /// branching tree bake *distance-from-the-soma* into uv.x so one travelling wave sweeps the whole
    /// tree root→tip in spatial order, instead of each branch restarting 0→1.
    pub(crate) fn add_with_u(&mut self, points: &[Vec3], radii: &[f32], sides: usize, u: &[f32]) {
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
                    .push([u[ri.min(u.len() - 1)], s as f32 / sides as f32]);
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

    /// Rescale every `uv.x` by `max_u` so absolute root→tip distances become a normalised 0→1 length
    /// coordinate across the whole tree.
    pub(crate) fn normalize_u(&mut self, max_u: f32) {
        if max_u <= 0.0 {
            return;
        }
        for uv in &mut self.uvs {
            uv[0] /= max_u;
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

/// A dim, hair-thin tangle of background micro-fibers filling a network's volume — non-interactive
/// "noise" that simulates infinite biological depth and scale contrast. Many filaments are merged
/// into one mesh (one draw call).
pub(crate) fn background_fibers_mesh(center: Vec3, radii: Vec3, count: usize, seed: u64) -> Mesh {
    let mut s = seed | 1;
    let mut builder = TubeBuilder::default();
    let thin = 0.04; // genuinely hair-thin (absolute) — faint depth strands, not thick gray streaks
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

/// One branch segment of a dendrite tree, exposed so the scene can weave data anatomy directly onto
/// it: `points` is the polyline, `depth` its recursion level (0 = trunk), `leaf` true at the tips.
pub(crate) struct DendriteBranch {
    pub(crate) points: Vec<Vec3>,
    pub(crate) depth: u32,
    pub(crate) leaf: bool,
}

/// A dendrite tree: the merged tube `mesh` plus the `branches` it was built from (for embedding
/// session beads along segments and package/research twigs at the tips).
pub(crate) struct DendriteTree {
    pub(crate) mesh: Mesh,
    pub(crate) branches: Vec<DendriteBranch>,
}

/// Grow a small fractal tree of tapering filaments out of a neuron, returning both the merged mesh
/// and the branch polylines (so data anatomy can be embedded onto them — see `anatomy.rs`).
pub(crate) fn dendrite_tree(node: Vec3, count: usize, node_r: f32, seed: u64) -> DendriteTree {
    let mut s = seed | 1;
    let mut builder = TubeBuilder::default();
    let mut branches: Vec<DendriteBranch> = Vec::new();
    // Track the farthest reach (absolute arc-length from the soma) so uv.x can be normalised 0→1
    // across the whole tree afterwards — one wave then sweeps root→tip in spatial order.
    let mut max_u = 0.0f32;
    for _ in 0..count {
        let mut dir =
            Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5).normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            dir = Vec3::Y;
        }
        // The trunk begins just inside the soma surface (along its outgoing direction) so its wide
        // root fuses with the cell body, then tapers and branches out into a fractal tree.
        let start = node + dir * node_r * crate::tuning::DEND_EMBED;
        grow_dendrite(&mut builder, &mut branches, &mut s, start, dir, node_r * crate::tuning::DEND_ROOT_R, node_r, 0, 0.0, &mut max_u);
    }
    builder.normalize_u(max_u);
    DendriteTree { mesh: builder.build(), branches }
}

/// Recursively grow one tapering dendrite branch, then (until the depth budget runs out) split it
/// into 2–3 finer children that continue from its tip — the branching tree of the reference
/// neurons. Radii taper from `r0` to `r1`; children inherit `r1` so the tree stays continuous
/// (thick trunk → hair-thin twigs). Each segment's polyline is recorded into `branches`.
#[allow(clippy::too_many_arguments)]
fn grow_dendrite(
    builder: &mut TubeBuilder,
    branches: &mut Vec<DendriteBranch>,
    s: &mut u64,
    start: Vec3,
    mut dir: Vec3,
    r0: f32,
    node_r: f32,
    depth: u32,
    start_dist: f32,
    max_u: &mut f32,
) {
    let leaf = depth >= crate::tuning::DEND_BRANCH_DEPTH;
    let r1 = if leaf { 0.012 } else { r0 * crate::tuning::DEND_BRANCH_TAPER };
    // The trunk (depth 0) gets extra base rings + a concave taper so it flares smoothly out of the
    // soma like a limb off a trunk; deeper branches are short and taper linearly.
    let trunk = depth == 0;
    let segs = (if trunk { 4 } else { 2 }) + (lcg(s) * 2.0) as usize;
    let seg_len = node_r * (0.7 + lcg(s) * 0.7) / (1.0 + depth as f32 * 0.5);
    let mut p = start;
    let mut pts = vec![p];
    for i in 0..segs {
        let j = Vec3::new(lcg(s) - 0.5, lcg(s) - 0.5, lcg(s) - 0.5) * 0.5;
        dir = (dir + j).normalize_or_zero();
        // Short first steps on the trunk so the base flare has resolution; then full strides.
        let step = if trunk && i < 2 { seg_len * 0.35 } else { seg_len };
        p += dir * step;
        pts.push(p);
    }
    let n = pts.len();
    let pow = if trunk { crate::tuning::DEND_ROOT_TAPER_POW } else { 1.0 };
    let radii: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            // r1 + (r0 - r1) * (1-t)^pow — concave when pow>1: fat only at the soma, thin beyond.
            (r1 + (r0 - r1) * (1.0 - t).powf(pow)).max(0.01)
        })
        .collect();
    // Distance from the soma at each ring: this branch's start distance plus its own arc-length, so
    // uv.x grows continuously outward (children continue from their parent's tip).
    let mut dist = vec![start_dist; n];
    for i in 1..n {
        dist[i] = dist[i - 1] + pts[i].distance(pts[i - 1]);
    }
    *max_u = max_u.max(dist[n - 1]);
    // 8-sided so the tube is a smooth round cross-section (a glassy rod with a clear centre and a
    // rim outline), not a faceted prism that reads as flat under the Fresnel membrane shader.
    builder.add_with_u(&pts, &radii, 8, &dist);
    branches.push(DendriteBranch { points: pts.clone(), depth, leaf });
    if !leaf {
        let nchild = if lcg(s) > 0.7 { 3 } else { 2 };
        for _ in 0..nchild {
            let spread = Vec3::new(lcg(s) - 0.5, lcg(s) - 0.5, lcg(s) - 0.5) * 0.9;
            let mut cdir = (dir + spread).normalize_or_zero();
            if cdir.length_squared() < 1e-6 {
                cdir = dir;
            }
            grow_dendrite(builder, branches, s, p, cdir, r1, node_r, depth + 1, dist[n - 1], max_u);
        }
    }
}

/// Smooth multi-octave pseudo-noise in [-1, 1] from a direction on the sphere — cheap, deterministic
/// (sine "value noise"), enough for organic lumps. `seed` shifts the pattern per variant.
fn soma_noise(d: Vec3, seed: f32) -> f32 {
    let p = d * 3.3 + Vec3::splat(seed);
    let o1 = (p.x).sin() * (p.y).cos() * (p.z).sin();
    let o2 = (p.x * 2.1 + 1.7).sin() * (p.y * 2.3).cos() * (p.z * 1.9 + 0.5).sin();
    let o3 = (p.x * 4.2).sin() * (p.y * 3.7 + 2.0).cos() * (p.z * 4.5).sin();
    (o1 * 0.6 + o2 * 0.3 + o3 * 0.1).clamp(-1.0, 1.0)
}

/// Granular soma mesh (Target 1, "the mass"): an icosphere whose vertices are pushed in/out along
/// their radius by `soma_noise`, giving a lumpy "rough gemstone / packed cluster" silhouette that
/// catches light unevenly instead of a smooth ball. Normals are recomputed so the bumps shade
/// correctly (the Fresnel rim then ripples across the contours). `subdivisions` sets resolution,
/// `amp` is bump depth as a fraction of radius, `seed` varies the lump pattern between variants.
pub(crate) fn displaced_sphere(subdivisions: u8, amp: f32, seed: u64) -> Mesh {
    let mut mesh = Sphere::new(1.0)
        .mesh()
        .ico(u32::from(subdivisions))
        .expect("icosphere subdivisions in range");
    let so = seed as f32 * 0.618_034;
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions.iter_mut() {
            let dir = Vec3::from_array(*p).normalize_or_zero(); // unit sphere → position == normal
            let r = 1.0 + amp * soma_noise(dir, so);
            *p = (dir * r).to_array();
        }
    }
    mesh.compute_normals();
    mesh
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

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex_count(mesh: &Mesh) -> usize {
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => p.len(),
            _ => 0,
        }
    }

    #[test]
    fn dendrites_branch_into_a_tree() {
        // A branching tree yields many branch polylines (trunks + recursive children) and far more
        // vertices than the trunk count alone; every position is finite. Deterministic per seed.
        let tree = dendrite_tree(Vec3::ZERO, 5, 1.0, 0xABCD);
        assert!(tree.branches.len() > 5, "trunks should split into children: {}", tree.branches.len());
        assert!(tree.branches.iter().any(|b| b.leaf), "tree has leaf tips");
        let n = vertex_count(&tree.mesh);
        assert!(n > 5 * 10, "expected a dense branched tree, got {n} verts");
        if let Some(VertexAttributeValues::Float32x3(p)) = tree.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert!(p.iter().flatten().all(|f| f.is_finite()), "all positions finite");
        }
        // Same seed → identical geometry (stable across reloads).
        assert_eq!(n, vertex_count(&dendrite_tree(Vec3::ZERO, 5, 1.0, 0xABCD).mesh));
    }

    #[test]
    fn dendrite_uv_runs_root_to_tip() {
        // uv.x must encode normalised distance-from-the-soma across the whole tree: ~0 at the roots,
        // reaching ~1 at the farthest tip — so a single travelling wave sweeps the tree in order
        // (not each branch restarting at 0). Verticies near the soma centre have the smallest u.
        let node = Vec3::ZERO;
        let tree = dendrite_tree(node, 5, 1.0, 0x51CE);
        let (Some(VertexAttributeValues::Float32x3(pos)), Some(VertexAttributeValues::Float32x2(uv))) = (
            tree.mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            tree.mesh.attribute(Mesh::ATTRIBUTE_UV_0),
        ) else {
            panic!("expected position + uv attributes");
        };
        let umin = uv.iter().map(|u| u[0]).fold(f32::INFINITY, f32::min);
        let umax = uv.iter().map(|u| u[0]).fold(f32::NEG_INFINITY, f32::max);
        assert!(umin >= 0.0, "u never negative, got {umin}");
        assert!(umin < 0.1, "roots start near u=0, got {umin}");
        assert!((umax - 1.0).abs() < 1e-3, "farthest tip normalises to u≈1, got {umax}");
        // The vertex with the smallest u is close to the soma; the largest is far from it.
        let near = pos[uv.iter().enumerate().min_by(|a, b| a.1[0].total_cmp(&b.1[0])).unwrap().0];
        let far = pos[uv.iter().enumerate().max_by(|a, b| a.1[0].total_cmp(&b.1[0])).unwrap().0];
        assert!(
            Vec3::from_array(near).distance(node) < Vec3::from_array(far).distance(node),
            "small-u vertex sits nearer the soma than the large-u vertex"
        );
    }

    #[test]
    fn displaced_sphere_stays_within_bump_bounds() {
        let amp = 0.18;
        let mesh = displaced_sphere(3, amp, 1);
        let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("expected Float32x3 positions");
        };
        assert!(!pos.is_empty(), "mesh has vertices");
        // Every vertex sits between the inward and outward extremes of the noise displacement —
        // it's a bumpy ball, never collapsed or blown out.
        for p in pos {
            let r = Vec3::from_array(*p).length();
            assert!(
                (1.0 - amp - 1e-3..=1.0 + amp + 1e-3).contains(&r),
                "vertex radius {r} outside bump bounds"
            );
        }
    }

    #[test]
    fn displaced_sphere_variants_differ() {
        // Different seeds must produce different lump patterns (so the pool isn't all clones).
        let a = displaced_sphere(3, 0.18, 1);
        let b = displaced_sphere(3, 0.18, 4);
        let (Some(VertexAttributeValues::Float32x3(pa)), Some(VertexAttributeValues::Float32x3(pb))) =
            (a.attribute(Mesh::ATTRIBUTE_POSITION), b.attribute(Mesh::ATTRIBUTE_POSITION))
        else {
            panic!("expected Float32x3 positions");
        };
        assert_ne!(pa, pb, "seeds should yield distinct geometry");
    }
}
