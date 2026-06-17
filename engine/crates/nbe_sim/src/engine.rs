//! The cohesive simulation step. `Sim` owns the live activation/pulse state and advances the
//! whole brain one `tick(dt)` at a time, composing the [`crate::pulse`] primitives:
//! fire eligible neurons → move action potentials → bump arrivals → cool everything.
//!
//! It is the CPU model the GPU compute-shader spark system is an optimization of: deterministic,
//! GPU-free, and fully unit-testable. The renderer drives one `tick` per frame and uses the
//! returned [`Tick`] (what fired / what arrived) to spawn and retire visual sparks.

use crate::pulse::{advance_pulses, apply_arrivals, decay, fire, ready_to_fire, Pulse, SimEdge};

/// Tunable dynamics for the simulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimParams {
    /// Base pulse travel speed (progress/sec before the edge-weight multiplier).
    pub base_speed: f32,
    /// Activation at which a neuron fires.
    pub threshold: f64,
    /// Activation added to a target when a pulse arrives.
    pub bump: f64,
    /// Activation lost per second (the graph cools between firings).
    pub decay_rate: f64,
    /// Seconds a neuron must wait after firing before it can fire again.
    pub refractory: f64,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            base_speed: 0.5,
            threshold: 0.8,
            bump: 0.9,
            decay_rate: 0.15,
            refractory: 1.0,
        }
    }
}

/// What happened during one [`Sim::tick`] — for the renderer to drive visuals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tick {
    /// Node indices that fired this tick (spawn outgoing sparks).
    pub fired: Vec<usize>,
    /// Node indices a pulse arrived at this tick (flash the target).
    pub arrived: Vec<usize>,
}

/// The live simulation state for a graph in index space.
pub struct Sim {
    pub activations: Vec<f64>,
    pub edges: Vec<SimEdge>,
    pub pulses: Vec<Pulse>,
    pub params: SimParams,
    /// Remaining refractory time per node (0 = ready to fire).
    cooldown: Vec<f64>,
}

impl Sim {
    pub fn new(activations: Vec<f64>, edges: Vec<SimEdge>, params: SimParams) -> Self {
        let n = activations.len();
        Self {
            activations,
            edges,
            pulses: Vec::new(),
            params,
            cooldown: vec![0.0; n],
        }
    }

    /// Manually excite a node (e.g. a business event lights up its neuron), clamped at 1.0.
    pub fn stimulate(&mut self, node: usize, value: f64) {
        if let Some(a) = self.activations.get_mut(node) {
            *a = (*a + value).min(1.0);
        }
    }

    /// Advance the brain by `dt` seconds. Returns what fired and what arrived this tick.
    pub fn tick(&mut self, dt: f64) -> Tick {
        // 1. Fire neurons that are hot and out of their refractory period.
        let mut fired = Vec::new();
        for n in ready_to_fire(&self.activations, self.params.threshold) {
            if self.cooldown[n] <= 0.0 {
                self.pulses.extend(fire(&self.edges, n));
                self.cooldown[n] = self.params.refractory;
                fired.push(n);
            }
        }

        // 2. Move action potentials; bump the neurons they reach.
        let arrived = advance_pulses(
            &self.edges,
            &mut self.pulses,
            dt as f32,
            self.params.base_speed,
        );
        apply_arrivals(&mut self.activations, &arrived, self.params.bump);

        // 3. Cool the whole graph and tick down refractory timers.
        decay(&mut self.activations, self.params.decay_rate, dt);
        for c in &mut self.cooldown {
            *c = (*c - dt).max(0.0);
        }

        Tick { fired, arrived }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_edges() -> Vec<SimEdge> {
        vec![
            SimEdge { source: 0, target: 1, weight: 1.0 },
            SimEdge { source: 1, target: 2, weight: 1.0 },
        ]
    }

    #[test]
    fn activation_propagates_down_a_chain() {
        // Only node 0 starts hot; it should drive 1 then 2.
        let params = SimParams {
            decay_rate: 0.0, // isolate propagation from cooling
            ..Default::default()
        };
        let mut sim = Sim::new(vec![1.0, 0.0, 0.0], chain_edges(), params);

        let mut fired_all = std::collections::HashSet::new();
        for _ in 0..30 {
            for n in sim.tick(0.34).fired {
                fired_all.insert(n);
            }
        }
        assert!(fired_all.contains(&0) && fired_all.contains(&1) && fired_all.contains(&2));
        assert!(sim.activations[2] >= params.threshold, "reached the end of the chain");
    }

    #[test]
    fn refractory_blocks_immediate_refire() {
        let params = SimParams {
            decay_rate: 0.0,
            refractory: 1.0,
            ..Default::default()
        };
        let mut sim = Sim::new(vec![1.0, 0.0, 0.0], chain_edges(), params);

        // Stays hot (no decay), so without a refractory it would fire every tick.
        assert_eq!(sim.tick(0.1).fired, vec![0], "fires once");
        assert!(sim.tick(0.1).fired.is_empty(), "blocked during refractory");
        // After the refractory window elapses it may fire again.
        let mut refired = false;
        for _ in 0..20 {
            if sim.tick(0.1).fired.contains(&0) {
                refired = true;
                break;
            }
        }
        assert!(refired, "fires again once refractory clears");
    }

    #[test]
    fn decay_eventually_silences_the_graph() {
        let mut sim = Sim::new(vec![1.0], Vec::new(), SimParams::default());
        for _ in 0..200 {
            sim.tick(0.1);
        }
        assert!(sim.activations[0] < SimParams::default().threshold, "cooled below firing");
    }
}
