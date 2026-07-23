use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

const APP_NAME: &str = "codepack";
const LEGACY_SETTINGS_FILE_NAME: &str = ".project_exporter_desktop.json";
const LEGACY_HISTORY_FILE_NAME: &str = ".project_exporter_history.json";
const SETTINGS_FILE_NAME: &str = "settings.json";
const DB_FILE_NAME: &str = "codepack.db";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
    Windows,
    Mac,
    Linux,
}

fn current_os() -> Os {
    if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Mac
    } else {
        Os::Linux
    }
}

fn layout(os: Os, home_dir: &Path, config_dir: &Path, data_local_dir: &Path) -> (PathBuf, PathBuf) {
    let settings_dir = config_dir.join(APP_NAME);
    let log_dir = match os {
        Os::Windows => data_local_dir.join(APP_NAME).join("logs"),
        Os::Mac => home_dir.join("Library").join("Logs").join(APP_NAME),
        Os::Linux => home_dir.join(".local").join("state").join(APP_NAME),
    };
    (settings_dir, log_dir)
}

pub(crate) fn home_dir_from_env() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn resolve_base_dirs() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let home_dir = home_dir_from_env().ok_or(CoreError::NoAppDirectories)?;
    let (config_dir, data_local_dir) = match current_os() {
        Os::Windows => {
            let roaming = std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or(CoreError::NoAppDirectories)?;
            let local = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or(CoreError::NoAppDirectories)?;
            (roaming, local)
        }
        Os::Mac => (
            home_dir.join("Library").join("Application Support"),
            PathBuf::new(),
        ),
        Os::Linux => {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home_dir.join(".config"));
            (config, PathBuf::new())
        }
    };
    Ok((home_dir, config_dir, data_local_dir))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    home_dir: PathBuf,
    settings_dir: PathBuf,
    log_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let (home_dir, config_dir, data_local_dir) = resolve_base_dirs()?;
        let (settings_dir, log_dir) = layout(current_os(), &home_dir, &config_dir, &data_local_dir);
        Ok(Self {
            home_dir,
            settings_dir,
            log_dir,
        })
    }

    pub fn for_root(root: &Path) -> Self {
        let home_dir = root.join("home");
        let config_dir = root.join("config");
        let data_local_dir = root.join("data_local");
        let (settings_dir, log_dir) = layout(current_os(), &home_dir, &config_dir, &data_local_dir);
        Self {
            home_dir,
            settings_dir,
            log_dir,
        }
    }

    pub fn settings_dir(&self) -> &Path {
        &self.settings_dir
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn settings_file(&self) -> PathBuf {
        self.settings_dir.join(SETTINGS_FILE_NAME)
    }

    pub fn legacy_settings_file(&self) -> PathBuf {
        self.home_dir.join(LEGACY_SETTINGS_FILE_NAME)
    }

    /// Mirrors legacy's `HISTORY_FILE = SETTINGS_FILE.with_name(".project_exporter_history.json")`
    /// (`export_history.py`): a flat JSON file sitting next to the legacy settings
    /// file in the user's home directory, read-only from this crate's perspective —
    /// `codepack-storage` (S5) imports it once, explicitly, into SQLite.
    pub fn legacy_history_file(&self) -> PathBuf {
        self.home_dir.join(LEGACY_HISTORY_FILE_NAME)
    }

    pub fn db_file(&self) -> PathBuf {
        self.settings_dir.join(DB_FILE_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_layout_matches_blueprint_d4() {
        let (settings_dir, log_dir) = layout(
            Os::Windows,
            Path::new("C:/Users/dev"),
            Path::new("C:/Users/dev/AppData/Roaming"),
            Path::new("C:/Users/dev/AppData/Local"),
        );
        assert_eq!(
            settings_dir,
            Path::new("C:/Users/dev/AppData/Roaming/codepack")
        );
        assert_eq!(
            log_dir,
            Path::new("C:/Users/dev/AppData/Local/codepack/logs")
        );
    }

    #[test]
    fn macos_layout_matches_blueprint_d4() {
        let (settings_dir, log_dir) = layout(
            Os::Mac,
            Path::new("/Users/dev"),
            Path::new("/Users/dev/Library/Application Support"),
            Path::new(""),
        );
        assert_eq!(
            settings_dir,
            Path::new("/Users/dev/Library/Application Support/codepack")
        );
        assert_eq!(log_dir, Path::new("/Users/dev/Library/Logs/codepack"));
    }

    #[test]
    fn linux_layout_matches_blueprint_d4() {
        let (settings_dir, log_dir) = layout(
            Os::Linux,
            Path::new("/home/dev"),
            Path::new("/home/dev/.config"),
            Path::new(""),
        );
        assert_eq!(settings_dir, Path::new("/home/dev/.config/codepack"));
        assert_eq!(log_dir, Path::new("/home/dev/.local/state/codepack"));
    }

    #[test]
    fn for_root_never_leaves_the_given_root() {
        let root = Path::new("/tmp/codepack-test-root");
        let paths = AppPaths::for_root(root);
        assert!(paths.settings_dir().starts_with(root));
        assert!(paths.log_dir().starts_with(root));
        assert!(paths.legacy_settings_file().starts_with(root));
    }

    #[test]
    fn legacy_settings_file_is_a_flat_file_in_home() {
        let paths = AppPaths::for_root(Path::new("/tmp/codepack-test-root"));
        assert_eq!(
            paths.legacy_settings_file().file_name().unwrap(),
            ".project_exporter_desktop.json"
        );
        assert_eq!(
            paths.legacy_settings_file().parent(),
            Some(paths.home_dir.as_path())
        );
    }

    #[test]
    fn settings_file_lives_under_settings_dir() {
        let paths = AppPaths::for_root(Path::new("/tmp/codepack-test-root"));
        assert_eq!(paths.settings_file().parent(), Some(paths.settings_dir()));
        assert_eq!(paths.settings_file().file_name().unwrap(), "settings.json");
    }

    #[test]
    fn legacy_history_file_is_a_flat_file_in_home_next_to_legacy_settings() {
        let paths = AppPaths::for_root(Path::new("/tmp/codepack-test-root"));
        assert_eq!(
            paths.legacy_history_file().file_name().unwrap(),
            ".project_exporter_history.json"
        );
        assert_eq!(
            paths.legacy_history_file().parent(),
            Some(paths.home_dir.as_path())
        );
        assert_eq!(
            paths.legacy_history_file().parent(),
            paths.legacy_settings_file().parent()
        );
    }

    #[test]
    fn db_file_lives_under_settings_dir() {
        let paths = AppPaths::for_root(Path::new("/tmp/codepack-test-root"));
        assert_eq!(paths.db_file().parent(), Some(paths.settings_dir()));
        assert_eq!(paths.db_file().file_name().unwrap(), "codepack.db");
    }
}
