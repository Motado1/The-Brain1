// Unified tube material: the soma's translucent Fresnel "glass" at rest, plus a continuous Gaussian
// energy wave that surges along the tube's length (uv.x 0→1) — replacing the old discrete dot
// pulses with a liquid, bioluminescent flow. Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb tube colour (a unused)
// x = rim power, y = rim intensity, z = rim alpha, w = unused — the resting glassy outline.
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle), z = width (sigma), w = unused.
@group(3) @binding(2) var<uniform> wave: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let fres = pow(1.0 - max(dot(n, v), 0.0), rest.x); // grazing rim → glassy outline

    var rgb = color.rgb * fres * rest.y;
    var a = clamp(fres * rest.z + 0.015, 0.0, 1.0);

    // Travelling Gaussian surge: brightest where uv.x == wave centre, falling off over `width`.
    let w = max(wave.z, 0.001);
    let d = in.uv.x - wave.x;
    let g = exp(-(d * d) / (2.0 * w * w)) * wave.y;
    rgb += color.rgb * g;            // HDR emissive crest → bloom blurs it into a bleeding wave
    a = clamp(a + g * 0.5, 0.0, 1.0); // and the tube reads as more solid where the energy is

    return vec4<f32>(rgb, a);
}
