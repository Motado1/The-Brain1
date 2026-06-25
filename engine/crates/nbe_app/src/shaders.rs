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

/// The connectors + dendrites wear the EXACT SAME glass cell-wall as the soma (soma.wgsl): a clear
/// translucent membrane with a glowing Fresnel rim, rendered `AlphaMode::Blend` so they read as the
/// same see-through glass as the cell body (additive can't be see-through). A travelling pulse floods
/// the tube with light where it passes. Per-tube instance so each carries its own wave uniform.
#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct DendriteMaterial {
    /// rgb = the soma's hue that bleeds into this tube near its soma end(s) + the pulse colour
    /// (network: amber CRM / indigo Research). The tube does NOT wear this at rest — it's colorless
    /// clear glass until lit by soma proximity or a pulse. a unused.
    #[uniform(0)]
    pub(crate) color: LinearRgba,
    /// x = rim power, y = rim/bleed intensity, z = rim alpha, w = pulse emissive (the travelling flood).
    #[uniform(1)]
    pub(crate) rest: Vec4,
    /// x = wave centre t (0..1), y = amplitude (0 = idle, 1 = active), z = width (sigma), w unused.
    #[uniform(2)]
    pub(crate) wave: Vec4,
    /// Soma-proximity glow ("light coming from the soma fills the tube"): x = strength at the u=0 end,
    /// y = strength at the u=1 end (dendrites light one end, connectors both), z = falloff length along
    /// u, w = the faint colorless glass-rim level shown everywhere at rest.
    #[uniform(3)]
    pub(crate) ends: Vec4,
}

impl Material for DendriteMaterial {
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
    app.add_plugins(MaterialPlugin::<DendriteMaterial>::default());
}
