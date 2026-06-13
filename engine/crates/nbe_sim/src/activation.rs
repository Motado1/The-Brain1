//! Activation thresholds — turn a neuron's business state into a `[0, 1]` activation value.
//!
//! Each signal contributes a candidate value and the strongest wins (`max`), so a single
//! urgent trigger (an overdue invoice, a renewal due tomorrow) lights the neuron up regardless
//! of the others. The renderer maps the result to node scale + emissive intensity.

use nbe_data::{CrmFacet, KnowledgeFacet, LedgerFacet};

const DAY: i64 = 86_400;
const RENEWAL_WINDOW_DAYS: f64 = 30.0;
const RECALL_WINDOW_DAYS: f64 = 7.0;

/// Everything the activation rules look at for one entity at time `now` (unix seconds).
#[derive(Debug, Default, Clone, Copy)]
pub struct ActivationInputs<'a> {
    pub crm: Option<&'a CrmFacet>,
    pub ledger: Option<&'a LedgerFacet>,
    pub knowledge: Option<&'a KnowledgeFacet>,
    /// Last time this neuron fired / was recalled — drives the recency signal.
    pub last_fired_at: Option<i64>,
    pub now: i64,
}

/// Compute the activation value in `[0, 1]`.
pub fn activation_value(inp: &ActivationInputs) -> f64 {
    let mut v: f64 = 0.0;

    // Upcoming contract renewal: closer = hotter, past-due = maxed out.
    if let Some(rd) = inp.crm.and_then(|c| c.renewal_date) {
        let days = (rd - inp.now) as f64 / DAY as f64;
        let c = if days <= 0.0 {
            1.0
        } else if days >= RENEWAL_WINDOW_DAYS {
            0.0
        } else {
            1.0 - days / RENEWAL_WINDOW_DAYS
        };
        v = v.max(c);
    }

    // CRM lifecycle emphasis.
    if inp.crm.map(|c| c.lifecycle_stage.as_str()) == Some("renewal") {
        v = v.max(0.6);
    }

    // Financial urgency.
    if let Some(l) = inp.ledger {
        let c = match l.invoice_status.as_str() {
            "overdue" => 0.85,
            "sent" => 0.4,
            _ => 0.0,
        };
        v = v.max(c);
    }

    // Recent knowledge-base recall: decays linearly over the recall window.
    if inp.knowledge.is_some() {
        if let Some(t) = inp.last_fired_at {
            let days = (inp.now - t) as f64 / DAY as f64;
            if (0.0..RECALL_WINDOW_DAYS).contains(&days) {
                v = v.max(1.0 - days / RECALL_WINDOW_DAYS);
            }
        }
    }

    v.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crm(stage: &str, renewal: Option<i64>) -> CrmFacet {
        CrmFacet {
            entity_id: "e".into(),
            contact: None,
            lifecycle_stage: stage.into(),
            session_schedule: None,
            renewal_date: renewal,
        }
    }

    fn ledger(status: &str) -> LedgerFacet {
        LedgerFacet {
            entity_id: "e".into(),
            amount_cents: 1000,
            invoice_status: status.into(),
            is_expense: false,
            tax_bucket: None,
            pacing_target_cents: None,
        }
    }

    #[test]
    fn renewal_proximity_scales_up() {
        let now = 1_000_000_000;
        let at = |days: i64| {
            let c = crm("active", Some(now + days * DAY));
            activation_value(&ActivationInputs {
                crm: Some(&c),
                now,
                ..Default::default()
            })
        };
        assert!((at(30) - 0.0).abs() < 1e-9, "30 days out = cold");
        assert!((at(15) - 0.5).abs() < 1e-9, "halfway = 0.5");
        assert!((at(0) - 1.0).abs() < 1e-9, "due now = hot");
        assert!((at(-5) - 1.0).abs() < 1e-9, "past due = maxed");
    }

    #[test]
    fn overdue_invoice_lights_up() {
        let l = ledger("overdue");
        let v = activation_value(&ActivationInputs {
            ledger: Some(&l),
            now: 0,
            ..Default::default()
        });
        assert!((v - 0.85).abs() < 1e-9);
    }

    #[test]
    fn strongest_signal_wins() {
        // cold renewal but overdue invoice -> overdue dominates
        let c = crm("active", Some(100 * DAY));
        let l = ledger("overdue");
        let v = activation_value(&ActivationInputs {
            crm: Some(&c),
            ledger: Some(&l),
            now: 0,
            ..Default::default()
        });
        assert!((v - 0.85).abs() < 1e-9);
    }

    #[test]
    fn recall_recency_decays() {
        let now = 10 * DAY;
        let k = KnowledgeFacet {
            entity_id: "e".into(),
            body_md: "x".into(),
            template_type: None,
            review_status: "reviewed".into(),
        };
        let recall = |ago_days: i64| {
            activation_value(&ActivationInputs {
                knowledge: Some(&k),
                last_fired_at: Some(now - ago_days * DAY),
                now,
                ..Default::default()
            })
        };
        assert!(recall(0) > recall(3), "more recent = hotter");
        assert!((recall(7) - 0.0).abs() < 1e-9, "outside window = cold");
    }

    #[test]
    fn empty_inputs_are_cold() {
        assert_eq!(activation_value(&ActivationInputs::default()), 0.0);
    }
}
