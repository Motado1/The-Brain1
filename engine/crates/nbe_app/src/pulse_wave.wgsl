// DendriteMaterial: the connectors + dendrites are COLORLESS clear-glass tubes. Unlike the soma they
// do NOT generate their own hue at rest — an idle tube is a faint, see-through glass cell-wall with a
// neutral (white) Fresnel rim, rendered with AlphaMode::Blend. Two things light a tube, both borrowing
// the SOMA's colour (never its own): (1) proximity to a soma — the cell body's light bleeds into the
// tube near its soma end(s) and fades along the length (uv.x); (2) a travelling Gaussian pulse that
// floods the tube where it passes. So colour lives at the cell bodies + in the pulses; the wiring
// between them reads as clear glass. Embedded via load_internal_asset!.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0) var<uniform> color: vec4<f32>; // rgb = soma-bleed + pulse hue (network), a unused.
@group(3) @binding(1) var<uniform> rest: vec4<f32>;
// x = rim power, y = rim/bleed intensity, z = rim alpha, w = pulse emissive.
@group(3) @binding(2) var<uniform> wave: vec4<f32>;
// x = wave centre t (0..1 along the path), y = amplitude (0 = idle, 1 = active), z = width (sigma).
@group(3) @binding(3) var<uniform> ends: vec4<f32>;
// x = soma-glow at u=0 end, y = soma-glow at u=1 end, z = falloff length (u), w = clear-glass rim level.

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    let fresnel = pow(1.0 - facing, rest.x); // ~0 facing the camera (clear), ~1 at the grazing rim

    // Resting tube: faint COLORLESS glass — a clear see-through centre with a neutral white rim, so the
    // wiring is visible as glass but carries no hue of its own.
    let glass_a = clamp(fresnel * rest.z + 0.015, 0.0, 1.0);
    let glass_rim = vec3<f32>(ends.w * fresnel);

    // Soma bleed: the tube is "filled with the light coming from the soma" near its soma end(s),
    // fading out along the length. A little flat fill (0.30) on top of the rim so it reads as the tube
    // glowing from within near the cell body, not just an outline.
    let fall = max(ends.z, 0.001);
    let prox = ends.x * (1.0 - smoothstep(0.0, fall, in.uv.x))     // peaks at the u=0 end
             + ends.y * smoothstep(1.0 - fall, 1.0, in.uv.x);      // peaks at the u=1 end
    let prox_c = clamp(prox, 0.0, 1.0);
    let soma_bleed = color.rgb * (rest.y * prox_c * (fresnel * 0.70 + 0.30));

    // Travelling Gaussian pulse along the length (uv.x). amplitude (wave.y) is 0 idle, up to 1 active.
    let w = max(wave.z, 0.001);
    let d = in.uv.x - wave.x;
    let g = exp(-(d * d) / (2.0 * w * w)) * wave.y;
    let pulse = color.rgb * (rest.w * g);

    // Colour comes only from the soma bleed + the pulse; the glass rim is colorless. The pulse fills
    // the tube (alpha -> 1) where it passes; soma proximity raises alpha so the lit base reads solid.
    let emissive = glass_rim + soma_bleed + pulse;
    let alpha = max(glass_a, max(prox_c * rest.z, g));

    return vec4<f32>(emissive, clamp(alpha, 0.0, 1.0));
}
