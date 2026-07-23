use std::collections::HashSet;
use std::sync::LazyLock;

pub const SENSITIVE_FILENAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials.json",
    "service-account.json",
    "service_account.json",
    "firebase-adminsdk.json",
];

pub const SENSITIVE_SUFFIXES: &[&str] =
    &["key", "pem", "p12", "pfx", "crt", "cer", "keystore", "jks"];

pub const HIGH_RISK_FILENAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    ".npmrc",
    ".pypirc",
    "credentials.json",
    "secrets.json",
    "secret.json",
    "service-account.json",
    "service_account.json",
    "firebase-adminsdk.json",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
];

pub const SAFE_MODE_EXCLUDED_SUFFIXES: &[&str] = &[
    "key", "pem", "p12", "pfx", "crt", "cer", "keystore", "jks", "db", "sqlite", "sqlite3", "bak",
    "dump", "backup", "7z", "rar", "tar", "gz", "xz", "zip",
];

pub const BALANCED_MODE_EXCLUDED_SUFFIXES: &[&str] = &[
    "key", "pem", "p12", "pfx", "crt", "cer", "keystore", "jks", "bak", "dump", "backup",
];

pub const SAFE_EXPORT_MODES: &[&str] = &["safe", "balanced", "full"];

static SENSITIVE_FILENAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| SENSITIVE_FILENAMES.iter().copied().collect());

static SENSITIVE_SUFFIXES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| SENSITIVE_SUFFIXES.iter().copied().collect());

static HIGH_RISK_FILENAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HIGH_RISK_FILENAMES.iter().copied().collect());

static SAFE_MODE_EXCLUDED_SUFFIXES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| SAFE_MODE_EXCLUDED_SUFFIXES.iter().copied().collect());

static BALANCED_MODE_EXCLUDED_SUFFIXES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| BALANCED_MODE_EXCLUDED_SUFFIXES.iter().copied().collect());

pub(crate) fn is_sensitive_filename(name: &str) -> bool {
    SENSITIVE_FILENAMES_SET.contains(name)
}

pub(crate) fn is_sensitive_suffix(suffix: &str) -> bool {
    SENSITIVE_SUFFIXES_SET.contains(suffix)
}

pub(crate) fn is_high_risk_filename(name: &str) -> bool {
    HIGH_RISK_FILENAMES_SET.contains(name)
}

pub(crate) fn is_safe_mode_excluded_suffix(suffix: &str) -> bool {
    SAFE_MODE_EXCLUDED_SUFFIXES_SET.contains(suffix)
}

pub(crate) fn is_balanced_mode_excluded_suffix(suffix: &str) -> bool {
    BALANCED_MODE_EXCLUDED_SUFFIXES_SET.contains(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sizes_match_legacy_counts() {
        assert_eq!(SENSITIVE_FILENAMES.len(), 13);
        assert_eq!(SENSITIVE_SUFFIXES.len(), 8);
        assert_eq!(HIGH_RISK_FILENAMES.len(), 17);
        assert_eq!(SAFE_MODE_EXCLUDED_SUFFIXES.len(), 20);
        assert_eq!(BALANCED_MODE_EXCLUDED_SUFFIXES.len(), 11);
    }

    #[test]
    fn no_duplicate_entries() {
        assert_eq!(SENSITIVE_FILENAMES_SET.len(), SENSITIVE_FILENAMES.len());
        assert_eq!(SENSITIVE_SUFFIXES_SET.len(), SENSITIVE_SUFFIXES.len());
        assert_eq!(HIGH_RISK_FILENAMES_SET.len(), HIGH_RISK_FILENAMES.len());
        assert_eq!(
            SAFE_MODE_EXCLUDED_SUFFIXES_SET.len(),
            SAFE_MODE_EXCLUDED_SUFFIXES.len()
        );
        assert_eq!(
            BALANCED_MODE_EXCLUDED_SUFFIXES_SET.len(),
            BALANCED_MODE_EXCLUDED_SUFFIXES.len()
        );
    }

    #[test]
    fn mode_specific_sets_are_sensitive_suffixes_plus_extras() {
        for suffix in SENSITIVE_SUFFIXES {
            assert!(is_safe_mode_excluded_suffix(suffix));
            assert!(is_balanced_mode_excluded_suffix(suffix));
        }
    }

    #[test]
    fn high_risk_filenames_and_sensitive_filenames_are_distinct_sets() {
        assert!(is_high_risk_filename(".npmrc"));
        assert!(!is_sensitive_filename(".npmrc"));
        assert!(is_sensitive_filename("id_rsa"));
        assert!(is_high_risk_filename("id_rsa"));
    }

    #[test]
    fn spot_check_known_members() {
        assert!(is_sensitive_suffix("pem"));
        assert!(!is_sensitive_suffix("txt"));
        assert!(is_safe_mode_excluded_suffix("db"));
        assert!(!is_balanced_mode_excluded_suffix("db"));
    }
}
