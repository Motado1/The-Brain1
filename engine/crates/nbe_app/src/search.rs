//! Offline fuzzy search — the engine behind the Ctrl+P / Cmd+K omni-search overlay. Pure, allocation-
//! light, and unit-tested so the ranking is verifiable headless; the egui overlay (in `ui.rs`) just
//! feeds it the loaded node corpus and flies the camera to the chosen result.

/// Score how well `query` fuzzy-matches `candidate` (case-insensitive subsequence). Returns `None`
/// when not every query character appears, in order, somewhere in the candidate. Higher is better:
/// consecutive runs, matches at word boundaries, and matches earlier in the string all score more.
/// An empty query matches everything with score 0 (so the overlay can list the whole corpus).
pub(crate) fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let q: Vec<char> = query
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| !c.is_whitespace())
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let cand: Vec<char> = candidate.chars().collect();
    let mut qi = 0usize;
    let mut score = 0i32;
    let mut prev_match: Option<usize> = None;
    for (ci, &ch) in cand.iter().enumerate() {
        if qi >= q.len() {
            break;
        }
        let lc = ch.to_lowercase().next().unwrap_or(ch);
        if lc == q[qi] {
            score += 1;
            // Word-boundary bonus: first char, or preceded by a non-alphanumeric separator.
            let at_boundary = ci == 0
                || cand
                    .get(ci - 1)
                    .map(|p| !p.is_alphanumeric())
                    .unwrap_or(false);
            if at_boundary {
                score += 10;
            }
            // Consecutive-match bonus: this char directly follows the previous matched char.
            if prev_match == Some(ci.wrapping_sub(1)) {
                score += 5;
            }
            // Earliness: a small penalty the further along the *first* match sits.
            if prev_match.is_none() {
                score -= ci as i32;
            }
            prev_match = Some(ci);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(score)
}

/// Rank `candidates` against `query`, returning their indices best-first. Non-matches are dropped
/// (unless `query` is empty, in which case all indices are returned in input order). Ties keep input
/// order (stable sort) so the result is deterministic.
pub(crate) fn fuzzy_rank(query: &str, candidates: &[&str]) -> Vec<usize> {
    let mut scored: Vec<(usize, i32)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, c)| fuzzy_score(query, c).map(|s| (i, s)))
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1)); // stable: equal scores keep enumeration order
    scored.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_in_order_subsequence() {
        assert!(fuzzy_score("jms", "James").is_some()); // J-m-s appear in order
        assert!(fuzzy_score("xyz", "James").is_none()); // no x
        assert!(fuzzy_score("sj", "James").is_none()); // wrong order (s before j)
    }

    #[test]
    fn empty_query_matches_all_in_order() {
        let cands = ["beta", "alpha", "gamma"];
        assert_eq!(fuzzy_rank("", &cands), vec![0, 1, 2]);
        assert_eq!(fuzzy_rank("   ", &cands), vec![0, 1, 2]); // whitespace-only too
    }

    #[test]
    fn word_start_and_earliness_score_higher() {
        // A match at the very start (word boundary, earliest position) outscores the same character
        // buried mid-word later in the string.
        let start = fuzzy_score("r", "Rehab").unwrap();
        let mid = fuzzy_score("r", "Stressor").unwrap();
        assert!(start > mid, "{start} !> {mid}");
    }

    #[test]
    fn ranks_by_relevance_and_drops_nonmatches() {
        let cands = ["James Bywater", "Jacob", "Shoulder Rehab"];
        let ranked = fuzzy_rank("jb", &cands);
        // "James Bywater" (J + B both at word starts) ranks above "Jacob" (J start, b mid-word);
        // "Shoulder Rehab" has no 'j' and is dropped entirely.
        assert_eq!(ranked, vec![0, 1]);
    }

    #[test]
    fn is_case_insensitive() {
        assert!(fuzzy_score("JAMES", "james bywater").is_some());
        assert!(fuzzy_score("james", "JAMES BYWATER").is_some());
    }
}
