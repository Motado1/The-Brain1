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
