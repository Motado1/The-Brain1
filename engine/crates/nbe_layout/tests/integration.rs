//! End-to-end: lay out a full seeded 500-node / 1,200-edge graph from the data engine.

use std::collections::BTreeMap;

use nbe_data::seed::{seed, SeedConfig};
use nbe_data::{snapshot, Db, Layer};
use nbe_layout::{layered_layout, Id, LayoutNode, LayoutParams};

#[test]
fn lays_out_full_seeded_graph() {
    let mut db = Db::open_in_memory().unwrap();
    seed(&mut db, &SeedConfig::default()).unwrap();
    let snap = snapshot::export(&db).unwrap();

    // Derive the number of hidden layers from the data, then map Layer -> column index.
    let mut max_hidden = 0u8;
    for l in &snap.layers {
        if let Some(Layer::Hidden(n)) = Layer::from_db(&l.layer) {
            max_hidden = max_hidden.max(n);
        }
    }
    let hidden_layers = max_hidden + 1;
    let column = |s: &str| -> u32 {
        match Layer::from_db(s).unwrap() {
            Layer::Input => 0,
            Layer::Hidden(n) => n as u32 + 1,
            Layer::Output => hidden_layers as u32 + 1,
        }
    };

    let nodes: Vec<LayoutNode> = snap
        .layers
        .iter()
        .map(|l| LayoutNode {
            id: l.entity_id.clone(),
            column: column(&l.layer),
        })
        .collect();
    let edges: Vec<(Id, Id)> = snap
        .edges
        .iter()
        .map(|e| (e.source_id.clone(), e.target_id.clone()))
        .collect();

    let initial = layered_layout(
        &nodes,
        &edges,
        &LayoutParams {
            sweeps: 0,
            ..Default::default()
        },
    );
    let solved = layered_layout(&nodes, &edges, &LayoutParams::default());

    // Every node is placed exactly once.
    assert_eq!(solved.nodes.len(), 500);

    // x increases strictly with column; a column shares one x.
    let mut col_x: BTreeMap<u32, f32> = BTreeMap::new();
    for p in &solved.nodes {
        let x = *col_x.entry(p.column).or_insert(p.x);
        assert_eq!(x, p.x, "same column must share x");
    }
    let xs: Vec<f32> = col_x.values().copied().collect();
    for w in xs.windows(2) {
        assert!(w[0] < w[1], "x must increase with column");
    }

    // The barycenter sweeps strictly reduce crossings on this tangled feed-forward graph.
    assert!(initial.crossings > 0, "seeded graph should start tangled");
    assert!(
        solved.crossings < initial.crossings,
        "expected fewer crossings: {} -> {}",
        initial.crossings,
        solved.crossings
    );
}
