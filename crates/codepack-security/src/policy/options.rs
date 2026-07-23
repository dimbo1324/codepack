#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityOptions {
    pub safe_export_mode: String,
    pub redact_secrets: bool,
    pub max_bytes_per_file: Option<u64>,
}

impl From<&codepack_core::config::Config> for SecurityOptions {
    fn from(config: &codepack_core::config::Config) -> Self {
        let max_bytes_per_file = if config.text_file_size_limit_enabled {
            Some(u64::from(config.max_text_file_mb) * 1024 * 1024)
        } else {
            None
        };
        Self {
            safe_export_mode: config.normalized_safe_export_mode().to_string(),
            redact_secrets: config.redact_secrets,
            max_bytes_per_file,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::config::Config;

    #[test]
    fn from_config_derives_max_bytes_from_mb_limit() {
        let config = Config {
            text_file_size_limit_enabled: true,
            max_text_file_mb: 2,
            safe_export_mode: "balanced".to_string(),
            ..Config::default()
        };

        let options = SecurityOptions::from(&config);
        assert_eq!(options.safe_export_mode, "balanced");
        assert_eq!(options.max_bytes_per_file, Some(2 * 1024 * 1024));
    }

    #[test]
    fn from_config_no_limit_when_disabled() {
        let config = Config {
            text_file_size_limit_enabled: false,
            ..Config::default()
        };

        let options = SecurityOptions::from(&config);
        assert_eq!(options.max_bytes_per_file, None);
    }

    #[test]
    fn from_config_falls_back_invalid_mode_to_safe() {
        let config = Config {
            safe_export_mode: "not-a-mode".to_string(),
            ..Config::default()
        };

        let options = SecurityOptions::from(&config);
        assert_eq!(options.safe_export_mode, "safe");
    }
}
