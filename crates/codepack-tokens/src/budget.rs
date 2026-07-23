//! Greedy selection under a token budget (BLUEPRINT §B.3/§E.5).
//!
//! This is a wholly new capability with no legacy precedent: the legacy-reference
//! module (`.ai/project/14-legacy-reference.md`) and a direct grep of the archived
//! codebase confirm no "Fit to budget" implementation exists there at all.
//! `importance` is always a caller-supplied value on [`BudgetCandidate`] — a future
//! `codepack-reports` job (`16_key_files_report`/`22_project_health_report`) computes
//! it; this module never does.

use std::cmp::Ordering;

/// One candidate competing for a slice of the token budget.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetCandidate {
    pub id: String,
    pub importance: f64,
    pub tokens: u64,
}

/// Why a candidate did not make it into the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExclusionReason {
    /// The candidate's own token cost exceeded whatever budget remained when greedy
    /// selection, in descending-rank order, reached it.
    OverBudget {
        rank: f64,
        tokens: u64,
        remaining_budget: u64,
    },
}

/// The result of [`fit_to_budget`].
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetSelection {
    pub included: Vec<String>,
    pub excluded: Vec<(String, ExclusionReason)>,
    pub total_tokens: u64,
}

/// Greedy density-ranked selection (BLUEPRINT §E.5): the exact "fit as much
/// importance as possible into a token budget" problem is a 0/1 knapsack, which is
/// NP-hard in general. The practical approximation ranks each candidate by value
/// density, `rank_i = importance_i / tokens_i`, and takes candidates in descending
/// rank order while the running total stays within `budget_tokens`. This trades
/// optimality for explainability: every excluded candidate can point at the exact
/// rank and remaining budget that excluded it.
///
/// Tie-breaking rule (deterministic, tested explicitly): candidates with exactly
/// equal rank keep their relative order from the input slice. `sort_by` is a stable
/// sort in Rust, so ranking is computed once per input index and then stably sorted
/// by rank alone (descending) — two same-rank candidates never swap position between
/// runs, and the one appearing earlier in `candidates` is always considered first.
///
/// A candidate with `tokens == 0` always ranks above every candidate with a positive
/// token cost (its density is treated as infinite) and always fits, regardless of
/// `importance`'s sign, since it never consumes any of the remaining budget.
pub fn fit_to_budget(candidates: &[BudgetCandidate], budget_tokens: u64) -> BudgetSelection {
    let mut ranked: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let rank = if candidate.tokens == 0 {
                f64::INFINITY
            } else {
                candidate.importance / candidate.tokens as f64
            };
            (index, rank)
        })
        .collect();

    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    let mut included = Vec::with_capacity(candidates.len());
    let mut excluded = Vec::new();
    let mut spent: u64 = 0;

    for (index, rank) in ranked {
        let candidate = &candidates[index];
        let remaining_budget = budget_tokens - spent;
        if candidate.tokens <= remaining_budget {
            included.push(candidate.id.clone());
            spent += candidate.tokens;
        } else {
            excluded.push((
                candidate.id.clone(),
                ExclusionReason::OverBudget {
                    rank,
                    tokens: candidate.tokens,
                    remaining_budget,
                },
            ));
        }
    }

    BudgetSelection {
        included,
        excluded,
        total_tokens: spent,
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetCandidate, ExclusionReason, fit_to_budget};

    fn candidate(id: &str, importance: f64, tokens: u64) -> BudgetCandidate {
        BudgetCandidate {
            id: id.to_string(),
            importance,
            tokens,
        }
    }

    #[test]
    fn determinism_repeat_runs_produce_identical_output() {
        let candidates = vec![
            candidate("a", 10.0, 50),
            candidate("b", 4.0, 80),
            candidate("c", 1.0, 30),
        ];

        let first = fit_to_budget(&candidates, 100);
        let second = fit_to_budget(&candidates, 100);
        assert_eq!(first, second);
    }

    #[test]
    fn explains_why_a_lower_ranked_candidate_was_excluded() {
        let candidates = vec![candidate("a", 10.0, 50), candidate("b", 4.0, 80)];

        let selection = fit_to_budget(&candidates, 100);

        assert_eq!(selection.included, vec!["a".to_string()]);
        assert_eq!(selection.total_tokens, 50);

        let expected_rank = 4.0_f64 / 80.0_f64;
        assert_eq!(
            selection.excluded,
            vec![(
                "b".to_string(),
                ExclusionReason::OverBudget {
                    rank: expected_rank,
                    tokens: 80,
                    remaining_budget: 50
                }
            )]
        );
    }

    #[test]
    fn ties_break_by_preserving_input_order() {
        // Both candidates rank at exactly importance/tokens = 0.1: 10/100 and 5/50 are
        // the same exact rational value, so IEEE-754 division yields bit-identical
        // f64 results for both — this is a genuine tie, not an approximate one.
        let candidates = vec![candidate("first", 10.0, 100), candidate("second", 5.0, 50)];

        let selection = fit_to_budget(&candidates, 100);

        assert_eq!(selection.included, vec!["first".to_string()]);
        assert_eq!(
            selection.excluded,
            vec![(
                "second".to_string(),
                ExclusionReason::OverBudget {
                    rank: 0.1,
                    tokens: 50,
                    remaining_budget: 0
                }
            )]
        );
    }

    #[test]
    fn zero_token_candidate_always_fits() {
        let candidates = vec![candidate("free", 0.0, 0), candidate("costly", 1.0, 10)];

        let selection = fit_to_budget(&candidates, 5);

        assert!(selection.included.contains(&"free".to_string()));
        assert_eq!(selection.total_tokens, 0);
    }
}
