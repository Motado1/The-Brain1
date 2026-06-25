//! Soma cell-body models. Six Blender-exported GLB variants (`assets/models/soma_01..06.glb`) —
//! distinct organic shapes (sphere / egg / oblate / oblong / blob / tall-oval), each a
//! bioluminescent glass membrane (Fresnel rim, dark translucent centre). They are loaded once at
//! startup and distributed across neurons by `pick_variant` so every network has visual variety,
//! while the glowing nucleus + halo billboards (spawned separately) read through the glass.

use bevy::asset::LoadState;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

/// The six loaded soma scenes, indexed by [`pick_variant`]. Inserted by [`setup_soma_assets`].
#[derive(Resource)]
pub(crate) struct SomaAssets {
    pub(crate) variants: [Handle<Scene>; 6],
}

/// Startup: load the six soma variant scenes from `assets/models/`. Registered to run **before**
/// `load_graph` so the handles exist when the scene is built. Loading is async — Bevy instantiates
/// each neuron's `SceneRoot` a frame or two later once its GLB finishes decoding; a missing file just
/// logs and leaves that variant empty (the neuron still has its nucleus + halo).
pub(crate) fn setup_soma_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("soma_assets: loading 6 GLB variants from assets/models/soma_01..06.glb");
    let variants = std::array::from_fn(|i| {
        let path = format!("models/soma_{:02}.glb", i + 1);
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))
    });
    commands.insert_resource(SomaAssets { variants });
}

/// Diagnostic: once every soma variant has reached a terminal load state, log each one's result
/// exactly once. If you see "SOMA GLB STATUS" with six `LOADED` lines, the GLB pipeline is live and
/// connected; `FAILED` points at a path/asset-root problem; if the block never appears at all, the
/// running binary is built from older code (pull the latest commit and rebuild).
pub(crate) fn report_soma_load(
    asset_server: Res<AssetServer>,
    soma: Option<Res<SomaAssets>>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }
    let Some(soma) = soma else { return };
    let mut lines: Vec<String> = Vec::with_capacity(soma.variants.len());
    for (i, h) in soma.variants.iter().enumerate() {
        match asset_server.get_load_state(h.id()) {
            Some(LoadState::Loaded) => lines.push(format!("soma_{:02}.glb: LOADED", i + 1)),
            Some(LoadState::Failed(e)) => lines.push(format!("soma_{:02}.glb: FAILED — {e}", i + 1)),
            _ => return, // still loading — wait until all six are terminal, then report together
        }
    }
    info!("=== SOMA GLB STATUS ({} variants) ===", lines.len());
    for l in &lines {
        info!("  {l}");
    }
    *reported = true;
}

/// Pick one of the six soma variants for a node from its id hash — a cheap, stable spread so the same
/// node always gets the same shape across reloads (and the choice is independent of spawn order).
pub(crate) fn pick_variant(seed: u64) -> usize {
    (seed % 6) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_index_always_in_range() {
        for seed in [0u64, 1, 5, 6, 7, 42, u64::MAX, 0xDEAD_BEEF] {
            assert!(pick_variant(seed) < 6, "variant index must index the 6-element array");
        }
        // Stable: same seed → same variant; and the six residues map onto all six slots.
        assert_eq!(pick_variant(13), pick_variant(13));
        let hit: std::collections::HashSet<usize> = (0u64..6).map(pick_variant).collect();
        assert_eq!(hit.len(), 6, "the six residues should cover all six variants");
    }
}
