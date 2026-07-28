//! Manual `PATH`/`PATHEXT` binary resolution.
//!
//! Not the `which` crate: this repository targets Windows 10/11 only (owner decision
//! 2026-07-26) and needs exactly one thing — "does this bare name resolve to a runnable
//! file on `PATH`, respecting `PATHEXT`" — which a bare `std::process::Command::new`
//! does not check for us up front (it would only fail, per file, at spawn time, and
//! `rustfmt` alone would never resolve without extension handling). A dependency that
//! also handles Unix executable-bit checks, custom search paths, and cross-platform
//! quirks is more than this one lookup needs (`.ai/universal/05-security-and-secrets.md`).

use std::path::{Path, PathBuf};

/// Finds `binary_name` on `PATH`. If the name already carries an extension, only that
/// exact name is checked; otherwise every `PATHEXT` extension is tried in order, exactly
/// as Windows' own `CreateProcess` search would.
pub(super) fn find_on_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    if Path::new(binary_name).extension().is_some() {
        return std::env::split_paths(&path_var)
            .map(|dir| dir.join(binary_name))
            .find(|candidate| candidate.is_file());
    }

    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let extensions: Vec<&str> = pathext.split(';').filter(|ext| !ext.is_empty()).collect();

    for dir in std::env::split_paths(&path_var) {
        for ext in &extensions {
            let candidate = dir.join(format!("{binary_name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_that_cannot_exist_is_not_found() {
        assert!(find_on_path("codepack-sanitize-definitely-not-a-real-tool-9f3b").is_none());
    }

    #[test]
    fn cmd_resolves_on_a_windows_path() {
        // `cmd.exe` ships with every Windows install and is always on `PATH`; this is
        // the cheapest possible proof that the PATHEXT loop actually walks real
        // directories rather than trivially returning `None` for everything.
        assert!(find_on_path("cmd").is_some());
    }
}
