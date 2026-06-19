// DendriteMaterial: one color-agnostic ADDITIVE glowing-filament material for connectors + dendrites.
// The connections in the reference images are not glass tubes you see *through* — they are thin,
// bright, glowing light strands against black, brightest near the cell bodies, with the data/firing
// pulse flooding light along them. So this material is purely EMISSIVE and rendered with
// AlphaMode::Add (see shaders.rs): the strand's camera-FACING front is the bright core and the
// grazing silhouette feathers off (the opposite of a Fresnel rim), its length is modulated by a
// brightness profile, and a travelling Gaussian crest adds light on top. Additive blending never
// occludes or accumulates to opaque, so even a fatter strand stays an airy glow rather than a solid
// cylinder. Color-agnostic: base_color adapts to CRM (amber) or Research (indigo).
// Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb = base_color (network hue).
// a = profile flag: >=0.5 → dendrite (root-bright→tip-dim), <0.5 → connector (bright at both ends).
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = core power (camera-facing softness), y = resting glow, z = length-profile floor,
// w = pulse emissive (massive, the travelling flood).
@group(3) @binding(2) var<uniform> wave: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle, 1 = active), z = width (sigma).

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    // Camera-facing front is the bright core of the strand; the grazing silhouette feathers off
    // (inverse of the soma's Fresnel rim) — so a finite-radius tube reads as a soft round glowing line.
    let core = pow(facing, max(rest.x, 0.001));

    // Length brightness profile along uv.x (0 = root, 1 = tip/far end). rest.z is the profile's floor.
    let u = clamp(in.uv.x, 0.0, 1.0);
    var profile: f32;
    if (color.a >= 0.5) {
        profile = mix(1.0, rest.z, u);                 // dendrite: bright at the soma root, dim at the tip
    } else {
        profile = mix(rest.z, 1.0, abs(u - 0.5) * 2.0); // connector: bright at both soma ends, dim mid-span
    }

    // Travelling Gaussian pulse along the length (uv.x). amplitude (wave.y) is 0 idle, up to 1 active.
    let w = max(wave.z, 0.001);
    let d = u - wave.x;
    let g = exp(-(d * d) / (2.0 * w * w)) * wave.y;

    // Emissive (HDR, drives bloom): resting strand glow modulated by length + the travelling flood.
    let emissive = color.rgb * (rest.y * profile + rest.w * g);
    // Additive alpha is a shape mask: the soft core cross-section, lifted to full where the pulse passes
    // so the segment visibly fills with light. With AlphaMode::Add the framebuffer gets emissive*alpha.
    let alpha = clamp(core + g, 0.0, 1.0);

    return vec4<f32>(emissive, alpha);
}
