# Renderer assets (`nbe_app`)

Drop Blender exports here. This folder is the Bevy asset root when you run the renderer with
`cargo run -p nbe_app` (Bevy resolves `assets/` relative to this crate's `CARGO_MANIFEST_DIR`).
For a directly-launched release binary, put an `assets/` folder next to the `.exe` instead.

## `soma.glb` — the neuron cell body

If a file named **`soma.glb`** is present here, the engine imports it and uses it for every neuron's
cell body (the membrane shell), instead of the procedural lumpy icosphere. **If it's absent, the
procedural soma is used** — so the app always runs; the model just "appears" once you add the file
and re-run (no recompile).

The engine keeps the glowing nucleus + halo billboards and the firing/breathing animation, so your
model only needs to be the **body/shell** — it's placed at each neuron, scaled by activation, and
gently breathes.

### Export requirements (must match for it to drop in cleanly)
- **Format:** glTF 2.0 binary `.glb`, textures embedded.
- **Scale:** model within a **unit sphere — radius 1.0 (diameter ~2.0), centred on the origin
  (0,0,0)**. The engine multiplies each instance by the neuron's size (~1.3–1.75).
- **Apply all transforms** before export (Blender: Ctrl+A → All Transforms) so scale = 1, rotation = 0.
- **+Y up** on export (glTF default).
- **Material:** Principled BSDF only (glTF exports Base Color, Metallic, Roughness, Emission
  Color + Strength, Alpha, Normal, Transmission). Author it **neutral/near-white** so the engine can
  tint per region later; put the glow on **Emission** (strength > 1 for HDR bloom). Bake any
  procedural noise/Geometry-Nodes to mesh + image textures first.
- **Geometry:** organic, lumpy, roughly radially symmetric (connectors attach from any direction);
  ~3k–10k triangles; clean outward normals.

### Variants (optional, later)
Multiple cell-body variants (`soma_01.glb` … `soma_06.glb`) can be wired up so the engine picks one
per neuron by hash. The current build loads a single `soma.glb`; ask to enable the variant set.
