// DendriteMaterial: one color-agnostic ADDITIVE glowing-filament material for connectors + dendrites.
// At REST the strand reads like the SOMA MEMBRANE — a clear see-through centre with a glowing Fresnel
// rim outlining the silhouette (the same cell-wall look as soma.wgsl), brightness modulated along its
// length. A travelling Gaussian pulse then FLOODS the whole strand with light as it passes. Rendered
// with AlphaMode::Add (see shaders.rs): the clear centre adds nothing (shows the black void), only the
// rim glows, and additive blending never occludes or stacks to opaque — so even a fatter trunk stays
// a clear-cored glowing outline rather than a solid cylinder. Color-agnostic: base_color adapts to
// CRM (amber) or Research (indigo). Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb = base_color (network hue).
// a = profile flag: >=0.5 → dendrite (root-bright→tip-dim), <0.5 → connector (bright at both ends).
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = rim power (sharpness of the membrane outline), y = resting rim glow, z = length-profile floor,
// w = pulse emissive (massive, the travelling flood).
@group(3) @binding(2) var<uniform> wave: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle, 1 = active), z = width (sigma).

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    // Fresnel rim: ~0 facing the camera (clear centre), ~1 at the grazing silhouette (glowing wall) —
    // the SAME cell-wall formula as the soma membrane (soma.wgsl), so the tubes read as the same glass.
    let fresnel = pow(1.0 - facing, max(rest.x, 0.001));

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

    // Emissive (HDR, drives bloom): resting rim glow modulated by length + the travelling flood.
    let emissive = color.rgb * (rest.y * fresnel * profile + rest.w * g);
    // Additive alpha: the rim outline at rest (clear centre adds nothing), lifted to a full bright fill
    // where the pulse passes. With AlphaMode::Add the framebuffer gets emissive*alpha.
    let alpha = clamp(fresnel * profile + g, 0.0, 1.0);

    return vec4<f32>(emissive, alpha);
}
