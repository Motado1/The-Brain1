// Glowing "glass tendril" material for the connection + dendrite tubes — the same Fresnel
// cell-wall language as the somas, so a strand reads as a translucent lit fibre rather than an
// opaque plastic pipe. Adds a length-wise glow gradient (bright at the roots, dim mid) and
// optional bands of light flowing along the strand. Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, globals}

@group(3) @binding(0) var<uniform> tint: vec4<f32>;   // rgb glow colour, a = rim opacity
// x = rim power (sharpness), y = base intensity, z = flow speed, w = flow strength.
@group(3) @binding(1) var<uniform> params: vec4<f32>;
// x = glow at uv.x=0 (root end), y = glow at uv.x=1 (other end), z = mid glow, w = gradient power.
@group(3) @binding(2) var<uniform> grad: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), params.x); // ~1 along a thin tube's grazing surface

    // Length-wise gradient: brightest toward the root end(s), dimmer in the middle.
    let t = in.uv.x;
    let to_end = abs(2.0 * t - 1.0);            // 1 at the ends, 0 at the middle
    let end_target = mix(grad.x, grad.y, t);    // the brightness of whichever end we're near
    let lengthwise = mix(grad.z, end_target, pow(to_end, grad.w));

    // Bands of light travelling along the strand (uv.x scrolled by time).
    let band = 0.5 + 0.5 * sin((t - globals.time * params.z) * 6.2831853 * 3.0);
    let flow = 1.0 + params.w * band;

    let intensity = params.y * lengthwise * flow;
    let rgb = tint.rgb * fresnel * intensity;
    let alpha = clamp(fresnel * tint.a + 0.02, 0.0, 1.0);
    return vec4<f32>(rgb, alpha);
}
