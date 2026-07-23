mod options;

pub use options::SecurityOptions;

use std::path::Path;

use crate::constants::{self, SAFE_EXPORT_MODES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyDecision {
    pub skip: bool,
    pub reason: String,
    pub severity: String,
}

impl Default for SafetyDecision {
    fn default() -> Self {
        Self {
            skip: false,
            reason: String::new(),
            severity: "medium".to_string(),
        }
    }
}

impl SafetyDecision {
    fn keep() -> Self {
        Self::default()
    }

    fn skip(reason: &str, severity: &str) -> Self {
        Self {
            skip: true,
            reason: reason.to_string(),
            severity: severity.to_string(),
        }
    }
}

pub fn normalise_mode(mode: &str) -> &str {
    if SAFE_EXPORT_MODES.contains(&mode) {
        mode
    } else {
        "safe"
    }
}

pub fn is_env_example(name: &str) -> bool {
    let lowered = name.to_lowercase();
    lowered.ends_with(".example")
        || lowered.ends_with(".sample")
        || lowered == ".env.example"
        || lowered == ".env.sample"
}

fn is_high_risk_env_file(name: &str) -> bool {
    constants::is_high_risk_filename(name) || (name.starts_with(".env") && !is_env_example(name))
}

fn file_name_and_suffix(path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let suffix = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    (name, suffix)
}

pub fn classify_sensitive_file(path: &Path) -> SafetyDecision {
    let (name, suffix) = file_name_and_suffix(path);

    if is_high_risk_env_file(&name) {
        return SafetyDecision::skip("high-risk credential filename", "critical");
    }
    if name.contains("secret") || name.contains("credential") || name.contains("private") {
        return SafetyDecision::skip("secret-like filename", "high");
    }
    if constants::is_safe_mode_excluded_suffix(&suffix) {
        return SafetyDecision::skip(&format!("sensitive/binary suffix .{suffix}"), "high");
    }
    SafetyDecision::keep()
}

pub fn should_skip_file_for_safety(relative_path: &Path, mode: &str) -> SafetyDecision {
    let mode = normalise_mode(mode);
    if mode == "full" {
        return SafetyDecision::keep();
    }

    let (name, suffix) = file_name_and_suffix(relative_path);

    if mode == "balanced" {
        if is_high_risk_env_file(&name) {
            return SafetyDecision::skip("high-risk credential filename", "critical");
        }
        if constants::is_balanced_mode_excluded_suffix(&suffix) {
            return SafetyDecision::skip(&format!("credential/key suffix .{suffix}"), "high");
        }
        return SafetyDecision::keep();
    }

    classify_sensitive_file(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_mode_falls_back_to_safe() {
        assert_eq!(normalise_mode("safe"), "safe");
        assert_eq!(normalise_mode("balanced"), "balanced");
        assert_eq!(normalise_mode("full"), "full");
        assert_eq!(normalise_mode("bogus"), "safe");
        assert_eq!(normalise_mode(""), "safe");
    }

    #[test]
    fn is_env_example_recognises_templates() {
        assert!(is_env_example(".env.example"));
        assert!(is_env_example(".env.sample"));
        assert!(is_env_example("config.EXAMPLE"));
        assert!(is_env_example("secrets.sample"));
        assert!(!is_env_example(".env"));
        assert!(!is_env_example(".env.local"));
    }

    #[test]
    fn safe_mode_skips_env_file() {
        let decision = should_skip_file_for_safety(Path::new(".env"), "safe");
        assert!(decision.skip);
        assert_eq!(decision.severity, "critical");
    }

    #[test]
    fn safe_mode_keeps_env_example() {
        let decision = should_skip_file_for_safety(Path::new(".env.example"), "safe");
        assert!(!decision.skip);
    }

    #[test]
    fn full_mode_keeps_sensitive_file() {
        let decision = should_skip_file_for_safety(Path::new("private.pem"), "full");
        assert!(!decision.skip);
    }

    #[test]
    fn safe_mode_flags_secret_like_filename() {
        let decision = should_skip_file_for_safety(Path::new("my_secret_notes.txt"), "safe");
        assert!(decision.skip);
        assert_eq!(decision.severity, "high");
        assert_eq!(decision.reason, "secret-like filename");
    }

    #[test]
    fn safe_mode_flags_excluded_suffix() {
        let decision = should_skip_file_for_safety(Path::new("data.sqlite3"), "safe");
        assert!(decision.skip);
        assert_eq!(decision.severity, "high");
    }

    #[test]
    fn balanced_mode_keeps_plain_db_but_skips_key() {
        let db = should_skip_file_for_safety(Path::new("data.sqlite3"), "balanced");
        assert!(!db.skip, "balanced mode does not exclude plain databases");

        let key = should_skip_file_for_safety(Path::new("service.key"), "balanced");
        assert!(key.skip);
        assert_eq!(key.severity, "high");
    }

    #[test]
    fn balanced_mode_skips_high_risk_filenames_not_in_sensitive_filenames() {
        let decision = should_skip_file_for_safety(Path::new(".npmrc"), "balanced");
        assert!(decision.skip);
        assert_eq!(decision.severity, "critical");
    }

    #[test]
    fn invalid_mode_falls_back_to_safe_behavior() {
        let decision = should_skip_file_for_safety(Path::new(".env"), "not-a-real-mode");
        assert!(decision.skip);
        assert_eq!(decision.severity, "critical");
    }

    #[test]
    fn classify_sensitive_file_precedence_order() {
        assert_eq!(
            classify_sensitive_file(Path::new("id_rsa")).reason,
            "high-risk credential filename"
        );
        assert_eq!(
            classify_sensitive_file(Path::new("my_credential_store.txt")).reason,
            "secret-like filename"
        );
        assert_eq!(
            classify_sensitive_file(Path::new("backup.bak")).reason,
            "sensitive/binary suffix .bak"
        );
        assert!(!classify_sensitive_file(Path::new("main.rs")).skip);
    }
}
