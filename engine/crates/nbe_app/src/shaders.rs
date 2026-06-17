//! Custom render materials (WGSL) for the organic neural look. Shaders are embedded via
//! `load_internal_asset!` so there's no runtime asset-directory dependency (works packaged).

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::{Shader, ShaderRef};

const SOMA_SHADER: Handle<Shader> = uuid_handle!("b1c0e6a2-1f3d-4c8e-9a77-0a1b2c3d4e5f");
const FILAMENT_SHADER: Handle<Shader> = uuid_handle!("c2d1f7b3-2a4e-4d9f-8b66-1b2c3d4e5f60");

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

/// Glowing "glass tendril" material for the connection + dendrite tubes — the soma's Fresnel look,
/// plus a length-wise glow gradient and bands of light flowing along the strand.
#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct FilamentMaterial {
    /// rgb = glow colour, a = rim opacity.
    #[uniform(0)]
    pub(crate) tint: LinearRgba,
    /// x = rim power, y = base intensity, z = flow speed, w = flow strength.
    #[uniform(1)]
    pub(crate) params: Vec4,
    /// x = glow at root end (uv.x=0), y = glow at far end (uv.x=1), z = mid glow, w = gradient power.
    #[uniform(2)]
    pub(crate) grad: Vec4,
}

impl Material for FilamentMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(FILAMENT_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Embed the shaders + register the custom material plugins. Call after `DefaultPlugins`.
pub(crate) fn register(app: &mut App) {
    load_internal_asset!(app, SOMA_SHADER, "soma.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, FILAMENT_SHADER, "filament.wgsl", Shader::from_wgsl);
    app.add_plugins(MaterialPlugin::<SomaMaterial>::default());
    app.add_plugins(MaterialPlugin::<FilamentMaterial>::default());
}
