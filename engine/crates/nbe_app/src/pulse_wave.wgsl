// DendriteMaterial: the connectors + dendrites wear the EXACT SAME glass cell-wall as the soma
// (soma.wgsl) so they look identical to the cell body — a clear, see-through centre with a glowing
// Fresnel rim outlining the silhouette, rendered with AlphaMode::Blend (real translucency, NOT
// additive — additive adds light and can never read as see-through glass). A travelling Gaussian
// pulse then floods the tube with light where it passes (alpha → 1 + a massive emissive surge), so
// the firing/data pulse still flows through. Color-agnostic: base_color adapts to CRM (amber) or
// Research (indigo). Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb = base_color (network hue), a unused.
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = rim power, y = rim intensity, z = rim alpha (all matched to the soma's RIM_*), w = pulse emissive.
@group(3) @binding(2) var<uniform> wave: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle, 1 = active), z = width (sigma).

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    let fresnel = pow(1.0 - facing, rest.x); // ~0 facing the camera (clear), ~1 at the grazing rim

    // Resting membrane: the SAME formula and constants as the soma (soma.wgsl) — transparent
    // see-through centre, glowing translucent rim — so the tubes are the same glass as the cell body.
    let membrane_a = clamp(fresnel * rest.z + 0.015, 0.0, 1.0);

    // Travelling Gaussian pulse along the length (uv.x). amplitude (wave.y) is 0 idle, up to 1 active.
    let w = max(wave.z, 0.001);
    let d = in.uv.x - wave.x;
    let g = exp(-(d * d) / (2.0 * w * w)) * wave.y;

    // The pulse OVERRIDES the membrane where it passes — the tube fills (alpha → 1) and floods with
    // intense emissive; at rest only the rim glows, exactly like the soma.
    let alpha = max(membrane_a, g);
    let emissive = color.rgb * (rest.y * fresnel + rest.w * g);

    return vec4<f32>(emissive, clamp(alpha, 0.0, 1.0));
}
