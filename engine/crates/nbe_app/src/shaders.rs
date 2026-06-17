//! Custom render materials (WGSL) for the organic neural look. Shaders are embedded via
//! `load_internal_asset!` so there's no runtime asset-directory dependency (works packaged).

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::{Shader, ShaderRef};

const SOMA_SHADER: Handle<Shader> = uuid_handle!("b1c0e6a2-1f3d-4c8e-9a77-0a1b2c3d4e5f");
const PULSE_WAVE_SHADER: Handle<Shader> = uuid_handle!("d3e2a8c4-3b5f-4ea0-9c55-2c3d4e5f6071");

/// Fresnel "cell-wall" material for the soma sphere: glowing at the silhouette rim, clear in the
/// centre — so the bright nucleus reads as light within a translucent membrane.
#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct SomaMaterial {
    #[uniform(0)]
    pub(crate) rim_color: LinearRgba,
    /// x = rim power (sharpness), y = rim intensity, z = rim alpha, w = unused.
    #[uniform(1)]
    pub(crate) params: Vec4,
}

impl Material for SomaMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SOMA_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Unified tube material: glassy Fresnel rest state + a travelling Gaussian energy wave (replaces
/// the discrete dot pulses). Per-tube instance so each carries its own wave timeline uniform.
#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct PulseWaveMaterial {
    /// rgb = tube colour (network hue), a unused.
    #[uniform(0)]
    pub(crate) color: LinearRgba,
    /// x = rim power, y = rim intensity, z = rim alpha, w unused (the resting glassy outline).
    #[uniform(1)]
    pub(crate) rest: Vec4,
    /// x = wave centre t (0..1), y = amplitude (0 = idle), z = width (sigma), w unused.
    #[uniform(2)]
    pub(crate) wave: Vec4,
}

impl Material for PulseWaveMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(PULSE_WAVE_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Embed the shaders + register the custom material plugins. Call after `DefaultPlugins`.
pub(crate) fn register(app: &mut App) {
    load_internal_asset!(app, SOMA_SHADER, "soma.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, PULSE_WAVE_SHADER, "pulse_wave.wgsl", Shader::from_wgsl);
    app.add_plugins(MaterialPlugin::<SomaMaterial>::default());
    app.add_plugins(MaterialPlugin::<PulseWaveMaterial>::default());
}
