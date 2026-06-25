//! Data model for the Neural Business Engine.
//!
//! Each [`Entity`] is one neuron. Its *role* — client, financial line-item, or knowledge
//! node — is emergent from which **facets** it carries (CRM / ledger / knowledge) and which
//! [`Layer`] it sits in, never a hard-coded type. This is the Entity-Component model from the
//! architecture doc, projected onto SQLite facet tables.

use serde::{Deserialize, Serialize};

/// Stable identifier (UUID, hyphenated string) — portable across exports and JSON snapshots.
pub type Id = String;

/// The neuron. The single thing every facet hangs off of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: Id,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Position of a neuron in the ANN topology. Drives the left→center→right layout (P2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    Input,
    Hidden(u8),
    Output,
}

impl Layer {
    /// Encode for the `layer_assignment.layer` TEXT column.
    pub fn to_db(&self) -> String {
        match self {
            Layer::Input => "input".to_string(),
            Layer::Hidden(n) => format!("hidden:{n}"),
            Layer::Output => "output".to_string(),
        }
    }

    /// Decode from the stored TEXT value.
    pub fn from_db(s: &str) -> Option<Layer> {
        match s {
            "input" => Some(Layer::Input),
            "output" => Some(Layer::Output),
            _ => s
                .strip_prefix("hidden:")
                .and_then(|n| n.parse::<u8>().ok())
                .map(Layer::Hidden),
        }
    }

    /// Column index used to order layers left→right (Input = 0, Hidden(n) = n+1, Output = last).
    pub fn column(&self, hidden_layers: u8) -> u32 {
        match self {
            Layer::Input => 0,
            Layer::Hidden(n) => *n as u32 + 1,
            Layer::Output => hidden_layers as u32 + 1,
        }
    }
}

/// "Behaves as a client" — CRM & scheduling facet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrmFacet {
    pub entity_id: Id,
    pub contact: Option<String>,
    /// lead | active | renewal | churned
    pub lifecycle_stage: String,
    pub session_schedule: Option<String>,
    pub renewal_date: Option<i64>,
}

/// "Behaves as a financial line-item" — ledger facet. Money is stored in integer cents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerFacet {
    pub entity_id: Id,
    pub amount_cents: i64,
    /// draft | sent | paid | overdue
    pub invoice_status: String,
    pub is_expense: bool,
    pub tax_bucket: Option<String>,
    pub pacing_target_cents: Option<i64>,
}

/// "Behaves as a knowledge node" — backlinked knowledge facet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeFacet {
    pub entity_id: Id,
    pub body_md: String,
    pub template_type: Option<String>,
    /// draft | reviewed | archived
    pub review_status: String,
}

/// "Client profile" — free-text personal-training detail for a client (the data behind the orbiting
/// "planet" nodes). All fields optional: an owner fills these in over time, and an empty profile
/// simply renders dim/absent planets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFacet {
    pub entity_id: Id,
    pub fitness_goals: Option<String>,
    pub dietary_needs: Option<String>,
    pub injury_history: Option<String>,
}

/// A straight, weighted, (usually) directed synaptic pathway between two neurons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: Id,
    pub source_id: Id,
    pub target_id: Id,
    /// hierarchy | dependency | reference | flow
    pub edge_type: String,
    pub weight: f64,
    pub directed: bool,
}

/// Real-time activation state — drives node scale + luminance + bloom in the renderer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activation {
    pub entity_id: Id,
    pub value: f64,
    pub threshold: f64,
    pub last_fired_at: Option<i64>,
}

/// An entity together with whatever facets it currently carries — the polymorphic view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityWithFacets {
    pub entity: Option<Entity>,
    pub crm: Option<CrmFacet>,
    pub ledger: Option<LedgerFacet>,
    pub knowledge: Option<KnowledgeFacet>,
    pub profile: Option<ProfileFacet>,
    pub activation: Option<Activation>,
    pub layer: Option<Layer>,
}

/// Aggregated financial roll-up that feeds the Output layer. Invariant: `paid + outstanding == total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRollup {
    pub total_cents: i64,
    pub paid_cents: i64,
    pub outstanding_cents: i64,
    pub income_cents: i64,
    pub expense_cents: i64,
}

/// A session package a client buys (PT10/PT20/PT30), paid in full up front. Each purchase or
/// renewal is its own row, so revenue history and per-package progress are preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub id: Id,
    pub client_id: Id,
    /// e.g. "PT10" / "PT20" / "PT30".
    pub kind: String,
    pub total_sessions: i64,
    /// Amount received up front, in cents.
    pub price_cents: i64,
    pub purchased_at: i64,
    pub active: bool,
}

/// A single training session, logged as it occurs ("James Bywater 9/10").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: Id,
    pub client_id: Id,
    pub package_id: Option<Id>,
    pub occurred_at: i64,
    /// completed | no_show | cancelled
    pub status: String,
    /// manual | gcal
    pub source: String,
    /// Calendar event UID — makes sync idempotent.
    pub external_id: Option<String>,
    pub note: Option<String>,
}

/// A recurring weekly time slot for a client (the fixed schedule). `cadence` is sessions per
/// week for this slot (1.0 weekly, 0.5 biweekly, ...), so a client's frequency is the sum of
/// their slot cadences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub id: Id,
    pub client_id: Id,
    /// 0 = Monday .. 6 = Sunday.
    pub weekday: i64,
    /// Minutes from midnight.
    pub start_min: i64,
    pub duration_min: i64,
    pub cadence: f64,
}
