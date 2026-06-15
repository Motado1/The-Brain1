use bevy::prelude::*;

// ---- networks + node kinds -------------------------------------------------------------

/// A self-contained graph, rendered far from its sibling so each reads as its own constellation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Network {
    Business,
    Research,
}

impl Network {
    pub(crate) const ALL: [Network; 2] = [Network::Business, Network::Research];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Network::Business => "Business — Clients",
            Network::Research => "Research — Knowledge",
        }
    }

    /// World-space centre. Research sits far away so it hangs in the sky like a distant cluster.
    pub(crate) fn center(self) -> Vec3 {
        match self {
            Network::Business => Vec3::ZERO,
            Network::Research => Vec3::new(1100.0, 160.0, -480.0),
        }
    }
}

/// What a node *is* — drives colour and size. Kind determines which network it lives in.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    Client,
    Ledger,
    Knowledge,
}

impl Kind {
    pub(crate) fn network(self) -> Network {
        match self {
            Kind::Knowledge => Network::Research,
            Kind::Client | Kind::Ledger => Network::Business,
        }
    }

    /// Base emissive hue (pre activation/intensity). Business is warm amber; Research is a cool
    /// blue-purple so the two clusters are instantly distinguishable.
    pub(crate) fn base_color(self) -> (f32, f32, f32) {
        match self {
            Kind::Client => (1.0, 0.55, 0.13),    // honey amber
            Kind::Ledger => (1.0, 0.22, 0.08),    // deep red (folded out of overview)
            Kind::Knowledge => (0.5, 0.42, 1.0),  // blue-purple
        }
    }

    /// Base node radius. Client and Knowledge are identical so the two networks render the same;
    /// only the data (and a subtle hue) differs.
    pub(crate) fn base_size(self) -> f32 {
        match self {
            Kind::Client => 1.3,
            Kind::Ledger => 0.45,
            Kind::Knowledge => 1.3,
        }
    }

    /// How far out in the network's shell this kind tends to sit (0 = core, 1 = surface). Client and
    /// Knowledge match so both clusters fill their volume the same way.
    pub(crate) fn shell(self) -> f32 {
        match self {
            Kind::Client => 0.3,
            Kind::Ledger => 0.65,
            Kind::Knowledge => 0.3,
        }
    }
}
