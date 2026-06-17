// Fresnel "cell-wall" material for the soma spheres: bright at the silhouette rim, clear in the
// centre — so the bright nucleus billboard inside reads as light glowing within a translucent
// membrane (the reference neuron look). Embedded via load_internal_asset! (no assets dir needed).

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> rim_color: vec4<f32>;
// x = rim power (sharpness), y = rim intensity, z = rim alpha, w = unused.
@group(3) @binding(1) var<uniform> params: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    let fresnel = pow(1.0 - facing, params.x);            // ~0 facing the camera, ~1 at the rim
    let rgb = rim_color.rgb * fresnel * params.y;         // glowing cell wall
    let alpha = clamp(fresnel * params.z + 0.015, 0.0, 1.0); // transparent centre, opaque rim
    return vec4<f32>(rgb, alpha);
}
