//! Model context-window limits (BLUEPRINT §B.2; legacy
//! `utils/text_utils.py::MODEL_CONTEXT_LIMITS`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Named model context limits, expressed in tokens.
///
/// Backed by a `BTreeMap` rather than a `Vec<(String, u64)>`: it serializes to the
/// same plain string-keyed JSON object shape as the legacy Python `dict[str, int]`
/// (via `#[serde(transparent)]`), and `BTreeMap`'s key-ordered iteration keeps
/// serialized output deterministic across runs, which matters for stable JSON
/// snapshots and diffs.
///
/// Loading an override file (or resolving a config path) to extend or replace this
/// table with fresher model limits is a documented future extension point — no such
/// file-loading, override, or path-resolution logic is implemented in this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelContextLimits(BTreeMap<String, u64>);

impl ModelContextLimits {
    /// The token limit registered for `model_name`, if any.
    pub fn get(&self, model_name: &str) -> Option<u64> {
        self.0.get(model_name).copied()
    }

    /// All entries, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0.iter().map(|(name, limit)| (name.as_str(), *limit))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for ModelContextLimits {
    /// The four legacy entries, verbatim in both name and token value
    /// (`utils/text_utils.py::MODEL_CONTEXT_LIMITS` in the archived Python
    /// implementation).
    fn default() -> Self {
        let mut limits = BTreeMap::new();
        limits.insert("Claude (200K)".to_string(), 200_000);
        limits.insert("GPT-4o (128K)".to_string(), 128_000);
        limits.insert("GPT-4 Turbo (128K)".to_string(), 128_000);
        limits.insert("Gemini 1.5 Pro (1M)".to_string(), 1_000_000);
        Self(limits)
    }
}

#[cfg(test)]
mod tests {
    use super::ModelContextLimits;

    #[test]
    fn default_matches_the_four_legacy_entries() {
        let limits = ModelContextLimits::default();
        assert_eq!(limits.len(), 4);
        assert_eq!(limits.get("Claude (200K)"), Some(200_000));
        assert_eq!(limits.get("GPT-4o (128K)"), Some(128_000));
        assert_eq!(limits.get("GPT-4 Turbo (128K)"), Some(128_000));
        assert_eq!(limits.get("Gemini 1.5 Pro (1M)"), Some(1_000_000));
        assert_eq!(limits.get("unknown model"), None);
    }

    #[test]
    fn json_round_trips() {
        let limits = ModelContextLimits::default();
        let json = serde_json::to_string(&limits).expect("in-memory serialization cannot fail");
        let round_tripped: ModelContextLimits =
            serde_json::from_str(&json).expect("round-tripping just-serialized JSON cannot fail");
        assert_eq!(limits, round_tripped);
    }

    #[test]
    fn json_shape_is_a_plain_string_keyed_object() {
        let limits = ModelContextLimits::default();
        let json = serde_json::to_string(&limits).expect("in-memory serialization cannot fail");
        assert!(json.starts_with('{'));
        assert!(json.contains("\"Claude (200K)\":200000"));
    }
}
