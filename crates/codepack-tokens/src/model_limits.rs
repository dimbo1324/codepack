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
/// [`ModelContextLimits::load_or_default`] reads an override file so limits can be
/// refreshed without rebuilding the application (BLUEPRINT §B.2). Resolving *where*
/// that file lives is the caller's business — this crate takes a path and stays free
/// of any dependency on `codepack-core`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelContextLimits(BTreeMap<String, u64>);

impl ModelContextLimits {
    /// Merges an override file over the built-in table, so a user can add or refresh a
    /// model without rebuilding (BLUEPRINT §B.2). A missing file yields the built-in
    /// table unchanged — that is the normal case, not an error.
    ///
    /// The file is merged, never substituted wholesale: an override listing one model
    /// should add that model, not silently delete the other three. Entries in the file
    /// win over built-ins with the same name.
    ///
    /// A file that exists but is unreadable or malformed returns an error rather than
    /// being quietly ignored: a user who wrote an override meant it to take effect, and
    /// silently falling back to stale limits is exactly the failure they would not
    /// notice.
    pub fn load_or_default(path: &std::path::Path) -> Result<Self, LoadError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(|source| LoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let overrides: BTreeMap<String, u64> =
            serde_json::from_str(&text).map_err(|source| LoadError::Parse {
                path: path.to_path_buf(),
                source,
            })?;

        let mut merged = Self::default();
        for (name, limit) in overrides {
            merged.0.insert(name, limit);
        }
        Ok(merged)
    }

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

/// Why an override file could not be applied. Distinct from "no override file", which
/// is not an error at all.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("could not read model-limits override at {path}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("model-limits override at {path} is not a JSON object of name -> token count")]
    Parse {
        path: std::path::PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod override_tests {
    use super::*;

    #[test]
    fn a_missing_file_yields_the_builtin_table() {
        let dir = tempfile::tempdir().unwrap();
        let limits = ModelContextLimits::load_or_default(&dir.path().join("absent.json")).unwrap();
        assert_eq!(limits, ModelContextLimits::default());
    }

    #[test]
    fn an_override_merges_over_the_builtin_table_without_dropping_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("model_limits.json");
        std::fs::write(&file, r#"{"Brand New 5": 2000000}"#).unwrap();

        let limits = ModelContextLimits::load_or_default(&file).unwrap();
        assert_eq!(limits.get("Brand New 5"), Some(2_000_000));
        assert_eq!(
            limits.len(),
            ModelContextLimits::default().len() + 1,
            "an override adds to the built-ins, it does not replace them"
        );
    }

    #[test]
    fn an_override_wins_for_a_name_that_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("model_limits.json");
        let existing = ModelContextLimits::default()
            .iter()
            .next()
            .map(|(name, _)| name.to_string())
            .unwrap();
        std::fs::write(&file, format!(r#"{{"{existing}": 42}}"#)).unwrap();

        let limits = ModelContextLimits::load_or_default(&file).unwrap();
        assert_eq!(limits.get(&existing), Some(42));
    }

    #[test]
    fn a_malformed_override_is_an_error_not_a_silent_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("model_limits.json");
        std::fs::write(&file, "not json at all").unwrap();
        assert!(ModelContextLimits::load_or_default(&file).is_err());
    }
}
