//! Action potentials — pulses that travel an edge from source to target and, on arrival,
//! bump the destination neuron's activation. This is the CPU side of the spark system; the GPU
//! renders the travelling particles from this same `progress` state.

/// An edge in index space (positions resolved by the caller). `weight` scales pulse speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimEdge {
    pub source: usize,
    pub target: usize,
    pub weight: f32,
}

/// A single action potential travelling along `edge`, `progress` in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pulse {
    pub edge: usize,
    pub progress: f32,
}

/// Advance every pulse by `dt` (speed = `base_speed * edge.weight`), remove the ones that
/// reached the end, and return the target node index of each arrival (in stable order).
pub fn advance_pulses(
    edges: &[SimEdge],
    pulses: &mut Vec<Pulse>,
    dt: f32,
    base_speed: f32,
) -> Vec<usize> {
    for p in pulses.iter_mut() {
        let w = edges[p.edge].weight.max(0.05);
        p.progress += dt * base_speed * w;
    }
    let arrivals: Vec<usize> = pulses
        .iter()
        .filter(|p| p.progress >= 1.0)
        .map(|p| edges[p.edge].target)
        .collect();
    pulses.retain(|p| p.progress < 1.0);
    arrivals
}

/// Apply an activation `bump` to each arrived target, clamped at 1.0.
pub fn apply_arrivals(activations: &mut [f64], arrivals: &[usize], bump: f64) {
    for &t in arrivals {
        activations[t] = (activations[t] + bump).min(1.0);
    }
}

/// Linear activation decay toward 0 (so the graph cools between firings).
pub fn decay(activations: &mut [f64], rate: f64, dt: f64) {
    for a in activations.iter_mut() {
        *a = (*a - rate * dt).max(0.0);
    }
}

/// Nodes whose activation has reached the firing threshold.
pub fn ready_to_fire(activations: &[f64], threshold: f64) -> Vec<usize> {
    activations
        .iter()
        .enumerate()
        .filter(|(_, &a)| a >= threshold)
        .map(|(i, _)| i)
        .collect()
}

/// Spawn a pulse on every outgoing edge of `node`.
pub fn fire(edges: &[SimEdge], node: usize) -> Vec<Pulse> {
    edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.source == node)
        .map(|(i, _)| Pulse {
            edge: i,
            progress: 0.0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn weight_scales_speed_and_reports_arrival() {
        let edges = [
            SimEdge { source: 0, target: 1, weight: 1.0 },
            SimEdge { source: 0, target: 2, weight: 0.5 },
        ];
        let mut pulses = vec![
            Pulse { edge: 0, progress: 0.0 },
            Pulse { edge: 1, progress: 0.0 },
        ];
        // dt*base_speed = 0.6 -> edge0 (w1) hits 0.6, edge1 (w0.5) hits 0.3
        let a = advance_pulses(&edges, &mut pulses, 0.6, 1.0);
        assert!(a.is_empty());
        // next tick: edge0 -> 1.2 (arrives at node 1), edge1 -> 0.6 (still travelling)
        let a = advance_pulses(&edges, &mut pulses, 0.6, 1.0);
        assert_eq!(a, vec![1]);
        assert_eq!(pulses.len(), 1, "slower pulse still in flight");
    }

    #[test]
    fn arrivals_bump_and_clamp() {
        let mut act = vec![0.0, 0.5, 0.95];
        apply_arrivals(&mut act, &[1, 2], 0.3);
        assert!((act[1] - 0.8).abs() < 1e-9);
        assert!((act[2] - 1.0).abs() < 1e-9, "clamped at 1.0");
    }

    #[test]
    fn decay_floors_at_zero() {
        let mut act = vec![0.1, 1.0];
        decay(&mut act, 0.5, 1.0);
        assert!((act[0] - 0.0).abs() < 1e-9);
        assert!((act[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn propagates_along_a_chain() {
        // 0 -> 1 -> 2; only node 0 starts hot.
        let edges = [
            SimEdge { source: 0, target: 1, weight: 1.0 },
            SimEdge { source: 1, target: 2, weight: 1.0 },
        ];
        let mut act = vec![1.0, 0.0, 0.0];
        let mut pulses: Vec<Pulse> = Vec::new();
        let mut fired: HashSet<usize> = HashSet::new();
        let (thr, bump) = (0.8, 0.9);

        for _ in 0..20 {
            for n in ready_to_fire(&act, thr) {
                if fired.insert(n) {
                    pulses.extend(fire(&edges, n));
                }
            }
            let arrivals = advance_pulses(&edges, &mut pulses, 0.5, 1.0);
            apply_arrivals(&mut act, &arrivals, bump);
        }

        assert!(act[1] >= thr, "node 1 activated by the pulse from 0");
        assert!(act[2] >= thr, "activation propagated through to node 2");
    }
}
