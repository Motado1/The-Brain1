// Tube material for connectors + dendrites: the SAME clear Fresnel "cell-wall" membrane as the soma
// (bright at the silhouette rim, transparent through the centre — see soma.wgsl), plus a continuous
// Gaussian energy wave that surges along the tube's length (uv.x 0→1) as a pulse flows through.
// Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb tube colour (a unused)
// x = rim power, y = rim intensity, z = rim alpha, w = unused — identical to the soma membrane.
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle), z = width (sigma), w = unused.
@group(3) @binding(2) var<uniform> wave: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    let fresnel = pow(1.0 - facing, rest.x);              // clear-membrane Fresnel, exactly like soma
    var rgb = rest.y * color.rgb * fresnel;               // glowing cell wall
    var a = clamp(fresnel * rest.z + 0.015, 0.0, 1.0);    // transparent centre, lit rim

    // Travelling Gaussian surge: brightest where uv.x == wave centre, falling off over `width`.
    let w = max(wave.z, 0.001);
    let d = in.uv.x - wave.x;
    let g = exp(-(d * d) / (2.0 * w * w)) * wave.y;
    rgb += color.rgb * g;            // HDR emissive crest → bloom blurs it into a bleeding wave
    a = clamp(a + g * 0.5, 0.0, 1.0); // and the tube reads as more solid where the energy is

    return vec4<f32>(rgb, a);
}
