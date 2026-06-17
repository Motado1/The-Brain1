//! Spatial-UI interaction backend (no visual layer yet): hover→action state, a request event bus
//! bridged to the tested `ops::*` mutations, the apoptosis (`Dissolving`) lifecycle, and the
//! financial→visual-scale aggregator. The decision logic is **pure functions** (unit-tested
//! headlessly below); the Bevy systems are thin adapters around them.

use bevy::prelude::*;

use crate::components::{DbPath, SceneControl};
use crate::domain::Kind;
use crate::nav::{NodeRegistry, Picker};

/// Configurable hover linger before the action buttons appear.
pub(crate) const HOVER_THRESHOLD: f32 = 0.45;
/// Apoptosis fade duration before the entity is despawned.
pub(crate) const DISSOLVE_SECS: f32 = 0.8;

// ---- 1. hover → action state ------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NodeAction {
    Sprout,
    Link,
    Edit,
    Dissolve,
}

/// The actions offered for a node of the given kind.
pub(crate) fn actions_for_kind(kind: Kind) -> Vec<NodeAction> {
    match kind {
        Kind::Client | Kind::Knowledge => vec![
            NodeAction::Sprout,
            NodeAction::Link,
            NodeAction::Edit,
            NodeAction::Dissolve,
        ],
        Kind::Ledger => vec![NodeAction::Edit, NodeAction::Dissolve],
    }
}

/// Hover/linger state that drives the spatial action buttons.
#[derive(Resource, Default)]
pub(crate) struct InteractionState {
    pub(crate) hovered_node: Option<usize>,
    pub(crate) hover_duration: f32,
    pub(crate) visible_actions: Vec<NodeAction>,
}

/// Advance the hover state: reset on a new (or no) target, otherwise accumulate time and reveal the
/// available actions once the linger threshold is crossed. Pure — unit-tested.
pub(crate) fn advance_hover(
    state: &mut InteractionState,
    hovered: Option<usize>,
    kind: Option<Kind>,
    dt: f32,
) {
    if hovered != state.hovered_node {
        // New (or no) target — restart the linger timer and hide the previous actions.
        state.hovered_node = hovered;
        state.hover_duration = 0.0;
        state.visible_actions.clear();
    }
    let (Some(_), Some(kind)) = (hovered, kind) else {
        return;
    };
    state.hover_duration += dt;
    if state.hover_duration >= HOVER_THRESHOLD && state.visible_actions.is_empty() {
        state.visible_actions = actions_for_kind(kind);
    }
}

pub(crate) fn update_interaction(
    time: Res<Time>,
    picker: Res<Picker>,
    registry: Res<NodeRegistry>,
    mut state: ResMut<InteractionState>,
) {
    let kind = picker.hovered.and_then(|i| registry.nodes.get(i)).map(|n| n.kind);
    advance_hover(&mut state, picker.hovered, kind, time.delta_secs());
}

// ---- 2. request event bus → ops ---------------------------------------------------------

#[derive(Message)]
pub(crate) struct UiRequestSprout {
    pub(crate) parent: usize,
    pub(crate) title: String,
}

#[derive(Message)]
pub(crate) struct UiRequestLink {
    pub(crate) a: usize,
    pub(crate) b: usize,
}

#[derive(Message)]
pub(crate) struct UiRequestEdit {
    pub(crate) node: usize,
}

#[derive(Message)]
pub(crate) struct UiRequestDissolve {
    pub(crate) node: usize,
}

/// Open the DB, run a mutation, and format the outcome for the status line (best-effort).
fn run_db<E: std::fmt::Display>(
    path: &str,
    f: impl FnOnce(&mut nbe_data::Db) -> Result<String, E>,
) -> String {
    match nbe_data::Db::open(path, None) {
        Ok(mut db) => f(&mut db).unwrap_or_else(|e| format!("failed: {e}")),
        Err(e) => format!("db: {e}"),
    }
}

pub(crate) fn on_sprout(
    mut reader: MessageReader<UiRequestSprout>,
    registry: Res<NodeRegistry>,
    db_path: Res<DbPath>,
    mut control: ResMut<SceneControl>,
) {
    for ev in reader.read() {
        let Some(node) = registry.nodes.get(ev.parent) else {
            continue;
        };
        let parent = node.id.clone();
        control.status =
            run_db(&db_path.0, |db| nbe_cli::ops::sprout(db, &parent, &ev.title, crate::now_unix()));
        control.reload = true;
    }
}

pub(crate) fn on_link(
    mut reader: MessageReader<UiRequestLink>,
    registry: Res<NodeRegistry>,
    db_path: Res<DbPath>,
    mut control: ResMut<SceneControl>,
) {
    for ev in reader.read() {
        let (Some(a), Some(b)) = (registry.nodes.get(ev.a), registry.nodes.get(ev.b)) else {
            continue;
        };
        let (a, b) = (a.id.clone(), b.id.clone());
        control.status = run_db(&db_path.0, |db| nbe_cli::ops::link(db, &a, &b, "ref", 1.0));
        control.reload = true;
    }
}

pub(crate) fn on_edit(
    mut reader: MessageReader<UiRequestEdit>,
    registry: Res<NodeRegistry>,
    db_path: Res<DbPath>,
    mut picker: ResMut<Picker>,
) {
    for ev in reader.read() {
        let Some(node) = registry.nodes.get(ev.node) else {
            continue;
        };
        picker.selected = Some(ev.node);
        picker.selected_entity = Some(node.entity);
        picker.detail = crate::systems::fetch_detail(&db_path.0, &node.id);
    }
}

pub(crate) fn on_dissolve(
    mut reader: MessageReader<UiRequestDissolve>,
    registry: Res<NodeRegistry>,
    db_path: Res<DbPath>,
    mut commands: Commands,
    mut control: ResMut<SceneControl>,
) {
    for ev in reader.read() {
        let Some(node) = registry.nodes.get(ev.node) else {
            continue;
        };
        let (id, entity) = (node.id.clone(), node.entity);
        control.status = run_db(&db_path.0, |db| nbe_cli::ops::delete(db, &id));
        // Start the fade; the purge system despawns it after the window (no reload, so it plays).
        commands.entity(entity).insert(Dissolving::new());
    }
}

// ---- 3. apoptosis lifecycle -------------------------------------------------------------

/// A node mid-deletion: fading out before it's despawned. Sim systems ignore `With<Dissolving>`.
#[derive(Component)]
pub(crate) struct Dissolving {
    pub(crate) remaining: f32,
}

impl Dissolving {
    pub(crate) fn new() -> Self {
        Self { remaining: DISSOLVE_SECS }
    }
    /// Advance the fade; returns true once finished (ready to despawn).
    pub(crate) fn tick(&mut self, dt: f32) -> bool {
        self.remaining -= dt;
        self.remaining <= 0.0
    }
    /// 1.0 → 0.0 fade factor (for the visual layer to read later).
    #[allow(dead_code)] // consumed by the visual dissolve layer (next phase) + tests
    pub(crate) fn fade(&self) -> f32 {
        (self.remaining / DISSOLVE_SECS).clamp(0.0, 1.0)
    }
}

pub(crate) fn purge_dissolving(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Dissolving)>,
) {
    let dt = time.delta_secs();
    for (e, mut d) in &mut q {
        if d.tick(dt) {
            commands.entity(e).despawn();
        }
    }
}

// ---- 4. financial scale aggregator ------------------------------------------------------

/// Revenue (cents) a client has earned → a target visual weight for its neuron. Diminishing returns,
/// capped, so a whale doesn't dwarf the whole brain. Pure — unit-tested.
pub(crate) fn revenue_to_scale(cents: i64) -> f32 {
    let dollars = (cents.max(0) as f32) / 100.0;
    (1.0 + (dollars / 2000.0).sqrt()).min(2.5)
}

#[derive(Component)]
pub(crate) struct ClientRevenue(pub(crate) i64);

#[derive(Component, Default)]
pub(crate) struct TargetVisualScale(pub(crate) f32);

pub(crate) fn update_financial_scale(mut q: Query<(&ClientRevenue, &mut TargetVisualScale)>) {
    for (rev, mut scale) in &mut q {
        scale.0 = revenue_to_scale(rev.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_depend_on_kind() {
        assert!(actions_for_kind(Kind::Client).contains(&NodeAction::Sprout));
        assert!(actions_for_kind(Kind::Knowledge).contains(&NodeAction::Link));
        let ledger = actions_for_kind(Kind::Ledger);
        assert!(!ledger.contains(&NodeAction::Sprout));
        assert!(ledger.contains(&NodeAction::Dissolve));
    }

    #[test]
    fn hover_reveals_actions_only_after_threshold() {
        let mut s = InteractionState::default();
        // below threshold → tracked, but no actions yet.
        advance_hover(&mut s, Some(3), Some(Kind::Client), 0.2);
        assert_eq!(s.hovered_node, Some(3));
        assert!(s.visible_actions.is_empty());
        // crossing the threshold → actions appear.
        advance_hover(&mut s, Some(3), Some(Kind::Client), 0.3);
        assert!(s.hover_duration >= HOVER_THRESHOLD);
        assert_eq!(s.visible_actions, actions_for_kind(Kind::Client));
    }

    #[test]
    fn changing_or_clearing_hover_resets() {
        let mut s = InteractionState::default();
        advance_hover(&mut s, Some(1), Some(Kind::Client), 1.0);
        assert!(!s.visible_actions.is_empty());
        // switching target restarts the linger (timer back under threshold, actions hidden).
        advance_hover(&mut s, Some(2), Some(Kind::Knowledge), 0.1);
        assert_eq!(s.hovered_node, Some(2));
        assert!(s.hover_duration < HOVER_THRESHOLD);
        assert!(s.visible_actions.is_empty());
        // clearing the hover resets to none.
        advance_hover(&mut s, None, None, 0.1);
        assert_eq!(s.hovered_node, None);
    }

    #[test]
    fn dissolving_finishes_after_its_window() {
        let mut d = Dissolving::new();
        assert!(d.fade() > 0.99);
        assert!(!d.tick(DISSOLVE_SECS * 0.5));
        assert!((d.fade() - 0.5).abs() < 0.01);
        assert!(d.tick(DISSOLVE_SECS)); // total elapsed past the window → done.
        assert_eq!(d.fade(), 0.0);
    }

    #[test]
    fn revenue_scale_is_monotonic_and_capped() {
        assert_eq!(revenue_to_scale(0), 1.0);
        assert_eq!(revenue_to_scale(-500), 1.0); // never below base.
        let mid = revenue_to_scale(200_000); // $2000 → 1 + sqrt(1) = 2.0
        assert!((mid - 2.0).abs() < 1e-4, "{mid}");
        assert!(revenue_to_scale(50_000) < mid); // less revenue → smaller weight.
        assert!(revenue_to_scale(i64::MAX / 2) <= 2.5); // capped.
    }
}
