//! Legacy settings migration, ported from `config.py::_migrate_legacy_settings`.
//!
//! The only historical migration rule: files written before
//! `text_file_size_limit_enabled` existed only ever recorded `max_text_file_mb`. If a
//! user had changed that away from the default (5), infer that they meant the limit to
//! be enabled.

use serde_json::Value;

const DEFAULT_MAX_TEXT_FILE_MB: i64 = 5;

/// Mutates `data` in place, matching the legacy function's `dict` mutation.
pub fn migrate_legacy_settings(data: &mut serde_json::Map<String, Value>) {
    if data.contains_key("text_file_size_limit_enabled") {
        return;
    }
    let Some(max_text_file_mb) = data.get("max_text_file_mb") else {
        return;
    };
    let enabled = max_text_file_mb
        .as_i64()
        .map(|value| value != DEFAULT_MAX_TEXT_FILE_MB)
        .unwrap_or(false);
    data.insert(
        "text_file_size_limit_enabled".to_string(),
        Value::Bool(enabled),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_enabled_when_limit_was_customized() {
        let Value::Object(mut data) = json!({ "max_text_file_mb": 10 }) else {
            unreachable!()
        };
        migrate_legacy_settings(&mut data);
        assert_eq!(
            data.get("text_file_size_limit_enabled"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn infers_disabled_when_limit_matches_default() {
        let Value::Object(mut data) = json!({ "max_text_file_mb": 5 }) else {
            unreachable!()
        };
        migrate_legacy_settings(&mut data);
        assert_eq!(
            data.get("text_file_size_limit_enabled"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn leaves_explicit_flag_untouched() {
        let Value::Object(mut data) =
            json!({ "max_text_file_mb": 10, "text_file_size_limit_enabled": false })
        else {
            unreachable!()
        };
        migrate_legacy_settings(&mut data);
        assert_eq!(
            data.get("text_file_size_limit_enabled"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn no_op_when_max_text_file_mb_absent() {
        let Value::Object(mut data) = json!({ "theme": "dark" }) else {
            unreachable!()
        };
        migrate_legacy_settings(&mut data);
        assert!(!data.contains_key("text_file_size_limit_enabled"));
    }
}
