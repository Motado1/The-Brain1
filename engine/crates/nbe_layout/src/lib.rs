//! `nbe_layout` — Sugiyama-style layered layout for the ANN topology.
//!
//! Layer assignment is an *input* here (the data engine already places each neuron in the
//! Input / Hidden(n) / Output layer), so this crate implements the two remaining Sugiyama
//! stages:
//!
//! 1. **Crossing reduction** — order nodes within each column by the iterated barycenter
//!    heuristic (alternating down/up sweeps), keeping the best ordering seen.
//! 2. **Coordinate assignment** — `x` from the column (left→right), `y` from the row order.
//!
//! Long edges (spanning more than one column) and intra-column edges don't participate in
//! the inter-layer ordering; the seeded graph is feed-forward (column → column+1) plus a few
//! intra-column reference links, which this handles directly without dummy nodes.
//!
//! The crate is pure (`std` only) and deterministic: identical input ⇒ identical [`Layout`].

use std::collections::HashMap;

pub type Id = String;

/// A node and the column (layer) it belongs to. Input = 0, Hidden(n) = n+1, Output = last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutNode {
    pub id: Id,
    pub column: u32,
}

/// Tunables for [`layered_layout`].
#[derive(Clone, Debug)]
pub struct LayoutParams {
    pub x_gap: f32,
    pub y_gap: f32,
    pub sweeps: usize,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            x_gap: 6.0,
            y_gap: 1.5,
            sweeps: 8,
        }
    }
}

/// Final placement of one node.
#[derive(Clone, Debug, PartialEq)]
pub struct NodePos {
    pub id: Id,
    pub column: u32,
    pub row: usize,
    pub x: f32,
    pub y: f32,
}

/// Result of a layout pass: placed nodes (ordered by column, then row) and the residual
/// inter-layer crossing count of the chosen ordering.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub nodes: Vec<NodePos>,
    pub crossings: usize,
}

/// Compute a layered layout. `edges` are directed `(source, target)` id pairs; direction is
/// ignored for ordering. Nodes are placed at their column's `x` and a centered `y` by row.
pub fn layered_layout(nodes: &[LayoutNode], edges: &[(Id, Id)], params: &LayoutParams) -> Layout {
    if nodes.is_empty() {
        return Layout {
            nodes: Vec::new(),
            crossings: 0,
        };
    }

    // Distinct columns, ascending -> contiguous slot indices 0..n_slots.
    let mut cols: Vec<u32> = nodes.iter().map(|n| n.column).collect();
    cols.sort_unstable();
    cols.dedup();
    let slot_of_col: HashMap<u32, usize> = cols.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let n_slots = cols.len();

    // Initial within-column order preserves input order (which the caller keeps deterministic).
    let mut order: Vec<Vec<Id>> = vec![Vec::new(); n_slots];
    let mut id_slot: HashMap<Id, usize> = HashMap::new();
    for n in nodes {
        let s = slot_of_col[&n.column];
        order[s].push(n.id.clone());
        id_slot.insert(n.id.clone(), s);
    }

    // Edges grouped by the adjacent slot pair they connect: pair `p` joins slot p and p+1,
    // stored as (upper_id in slot p, lower_id in slot p+1).
    let mut pair_edges: Vec<Vec<(Id, Id)>> = vec![Vec::new(); n_slots.saturating_sub(1)];
    for (a, b) in edges {
        let (Some(&sa), Some(&sb)) = (id_slot.get(a), id_slot.get(b)) else {
            continue;
        };
        if sa + 1 == sb {
            pair_edges[sa].push((a.clone(), b.clone()));
        } else if sb + 1 == sa {
            pair_edges[sb].push((b.clone(), a.clone()));
        }
        // intra-slot or long edges: not used for inter-layer ordering
    }

    let total = |order: &[Vec<Id>]| -> usize {
        let pos = positions(order);
        let mut sum = 0;
        for (p, pe) in pair_edges.iter().enumerate() {
            let list: Vec<(usize, usize)> =
                pe.iter().map(|(u, v)| (pos[p][u], pos[p + 1][v])).collect();
            sum += pair_crossings(&list);
        }
        sum
    };

    let mut best = order.clone();
    let mut best_c = total(&order);

    for _ in 0..params.sweeps {
        // Down sweep: fix slot s-1, reorder slot s.
        for s in 1..n_slots {
            reorder_by_neighbors(&mut order, s, s - 1, &pair_edges[s - 1], true);
        }
        // Up sweep: fix slot s+1, reorder slot s.
        for s in (0..n_slots.saturating_sub(1)).rev() {
            reorder_by_neighbors(&mut order, s, s + 1, &pair_edges[s], false);
        }
        let c = total(&order);
        if c < best_c {
            best_c = c;
            best = order.clone();
        }
    }

    // Coordinate assignment from the best ordering.
    let mut out = Vec::with_capacity(nodes.len());
    for (s, ids) in best.iter().enumerate() {
        let col = cols[s];
        let len = ids.len();
        for (row, id) in ids.iter().enumerate() {
            let x = col as f32 * params.x_gap;
            let y = (row as f32 - (len as f32 - 1.0) / 2.0) * params.y_gap;
            out.push(NodePos {
                id: id.clone(),
                column: col,
                row,
                x,
                y,
            });
        }
    }
    out.sort_by(|a, b| a.column.cmp(&b.column).then(a.row.cmp(&b.row)));

    Layout {
        nodes: out,
        crossings: best_c,
    }
}

/// Per-slot map of id -> current row index.
fn positions(order: &[Vec<Id>]) -> Vec<HashMap<Id, usize>> {
    order
        .iter()
        .map(|c| {
            c.iter()
                .enumerate()
                .map(|(i, id)| (id.clone(), i))
                .collect()
        })
        .collect()
}

/// Reorder `slot` by the barycenter of each node's neighbors in the fixed adjacent slot.
/// Nodes with no neighbor keep their current index. Ties break by current index (stable,
/// hence deterministic). `node_is_lower` selects which endpoint of the stored pair edge is in
/// `slot` (lower = slot p+1, upper = slot p).
fn reorder_by_neighbors(
    order: &mut [Vec<Id>],
    slot: usize,
    fixed: usize,
    edges: &[(Id, Id)],
    node_is_lower: bool,
) {
    // Maps keyed by owned ids so no borrow into `order` outlives the reassignment below.
    let fixed_pos: HashMap<Id, usize> = order[fixed]
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    let mut neigh: HashMap<Id, Vec<usize>> = HashMap::new();
    for (u, v) in edges {
        let (node, other) = if node_is_lower { (v, u) } else { (u, v) };
        if let Some(&pos) = fixed_pos.get(other) {
            neigh.entry(node.clone()).or_default().push(pos);
        }
    }

    let cur: HashMap<Id, usize> = order[slot]
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    let mut keyed: Vec<(f64, usize, Id)> = order[slot]
        .iter()
        .map(|id| {
            let c = cur[id];
            let bary = match neigh.get(id) {
                Some(ps) if !ps.is_empty() => {
                    ps.iter().map(|&p| p as f64).sum::<f64>() / ps.len() as f64
                }
                _ => c as f64,
            };
            (bary, c, id.clone())
        })
        .collect();

    keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    order[slot] = keyed.into_iter().map(|(_, _, id)| id).collect();
}

/// Crossings between two layers given each edge as `(upper_pos, lower_pos)`: the number of
/// edge pairs whose endpoints are in opposite order — i.e. inversions of the lower positions
/// after sorting by upper position.
fn pair_crossings(list: &[(usize, usize)]) -> usize {
    if list.len() < 2 {
        return 0;
    }
    let mut e = list.to_vec();
    e.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let vs: Vec<usize> = e.iter().map(|x| x.1).collect();
    inversions(&vs)
}

/// Count inversions (strictly out-of-order pairs) with a Fenwick tree, O(n log n).
fn inversions(seq: &[usize]) -> usize {
    let Some(&max) = seq.iter().max() else {
        return 0;
    };
    let mut bit = vec![0usize; max + 2];
    let mut inv = 0;
    for (inserted, &v) in seq.iter().enumerate() {
        let leq = fenwick_prefix(&bit, v + 1); // already-inserted values <= v
        inv += inserted - leq;
        fenwick_add(&mut bit, v + 1);
    }
    inv
}

fn fenwick_add(bit: &mut [usize], mut i: usize) {
    while i < bit.len() {
        bit[i] += 1;
        i += i & i.wrapping_neg();
    }
}

fn fenwick_prefix(bit: &[usize], mut i: usize) -> usize {
    let mut s = 0;
    while i > 0 {
        s += bit[i];
        i -= i & i.wrapping_neg();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(spec: &[(&str, u32)]) -> Vec<LayoutNode> {
        spec.iter()
            .map(|(id, c)| LayoutNode {
                id: (*id).into(),
                column: *c,
            })
            .collect()
    }

    fn edges(spec: &[(&str, &str)]) -> Vec<(Id, Id)> {
        spec.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect()
    }

    #[test]
    fn counts_a_single_crossing() {
        // (upper,lower) = (0,1),(1,0) -> exactly one crossing.
        assert_eq!(pair_crossings(&[(0, 1), (1, 0)]), 1);
        // nested, no crossing
        assert_eq!(pair_crossings(&[(0, 0), (1, 1)]), 0);
        // shared endpoints never cross
        assert_eq!(pair_crossings(&[(0, 5), (0, 2)]), 0);
    }

    #[test]
    fn reduces_crossings() {
        let n = nodes(&[("a0", 0), ("a1", 0), ("b0", 1), ("b1", 1)]);
        let e = edges(&[("a0", "b1"), ("a1", "b0")]);

        let initial = layered_layout(&n, &e, &LayoutParams { sweeps: 0, ..Default::default() });
        let solved = layered_layout(&n, &e, &LayoutParams { sweeps: 8, ..Default::default() });

        assert_eq!(initial.crossings, 1, "crossed initial ordering");
        assert_eq!(solved.crossings, 0, "barycenter should untangle it");
        assert!(solved.crossings < initial.crossings);
    }

    #[test]
    fn x_is_monotonic_by_column() {
        let n = nodes(&[("i", 0), ("h0", 1), ("h1", 1), ("o", 2)]);
        let layout = layered_layout(&n, &[], &LayoutParams::default());

        let x_of = |id: &str| layout.nodes.iter().find(|p| p.id == id).unwrap().x;
        assert!(x_of("i") < x_of("h0"));
        assert!(x_of("h0") < x_of("o"));
        assert_eq!(x_of("h0"), x_of("h1"), "same column shares x");
    }

    #[test]
    fn places_every_node_once() {
        let n = nodes(&[("a", 0), ("b", 1), ("c", 1), ("d", 2)]);
        let layout = layered_layout(&n, &edges(&[("a", "b"), ("a", "c"), ("b", "d")]), &Default::default());
        assert_eq!(layout.nodes.len(), 4);
        let mut ids: Vec<_> = layout.nodes.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn is_deterministic() {
        let n = nodes(&[("a0", 0), ("a1", 0), ("b0", 1), ("b1", 1), ("c0", 2)]);
        let e = edges(&[("a0", "b1"), ("a1", "b0"), ("b0", "c0"), ("b1", "c0")]);
        let p = LayoutParams::default();
        assert_eq!(layered_layout(&n, &e, &p), layered_layout(&n, &e, &p));
    }
}
