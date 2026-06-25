# Renderer assets (`nbe_app`)

Blender exports live here. This folder is the Bevy asset root when you run the renderer with
`cargo run -p nbe_app` (Bevy resolves `assets/` relative to this crate's `CARGO_MANIFEST_DIR`).
For a directly-launched release binary, put an `assets/` folder next to the `.exe` instead.

## `models/soma_01.glb` … `models/soma_06.glb` — the neuron cell bodies

The engine loads **six** soma variants at startup (`soma_assets::setup_soma_assets`) and gives each
neuron one of them, picked by a hash of its id (`pick_variant`), so every network has organic variety.
Each is spawned as a `SceneRoot` at the neuron, scaled by activation, and breathes. The glowing
nucleus + halo billboards and the firing/breathing animation are separate, so your model only needs to
be the **body/shell** — a bioluminescent glass membrane (Fresnel rim, dark translucent centre).

These six files are required for the current build (there is no procedural fallback — a missing file
just leaves that variant's neurons with only their nucleus + halo glow).

### Export requirements (must match for it to drop in cleanly)
- **Format:** glTF 2.0 binary `.glb`, textures embedded, first scene = the model (loaded as `Scene0`).
- **Scale:** model within a **unit sphere — radius 1.0 (diameter ~2.0), centred on the origin
  (0,0,0)**. The engine multiplies each instance by the neuron's size (~1.3–1.75).
- **Apply all transforms** before export (Blender: Ctrl+A → All Transforms) so scale = 1, rotation = 0.
- **+Y up** on export (glTF default).
- **Material:** Principled BSDF only (glTF exports Base Color, Metallic, Roughness, Emission
  Color + Strength, Alpha, Normal, Transmission). Put the glow on **Emission** (strength > 1 for HDR
  bloom). Bake any procedural noise / Geometry-Nodes to mesh + image textures first.
- **Geometry:** organic, lumpy, roughly radially symmetric; ~3k–10k triangles; clean outward normals.

### Replacing / re-exporting
Drop a new `soma_0N.glb` over the old one and re-run — no recompile. To change how many variants
there are, edit the array length in `src/soma_assets.rs` (`SomaAssets.variants` + `pick_variant`'s
modulus) and the loader loop.
