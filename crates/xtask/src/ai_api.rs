//! Checks `codepack-ai-api`, the one crate the gate deliberately cannot see.
//!
//! Excluding a package from the workspace buys a great deal — no `keyring` on Linux, no
//! `ureq` in the product's `Cargo.lock`, nothing extra in `cargo deny`'s graph — and
//! costs exactly one thing: `cargo xtask gate` stops compiling it, so it can rot without
//! anyone noticing. That cost was accepted knowingly (owner decision 2026-09-06, Q41),
//! and this command is what keeps it bounded.
//!
//! It is not part of `gate`, and putting it there would undo the exclusion: the whole
//! point is that a routine check on a Linux machine does not need a Secret Service
//! backend. Run it when touching this crate, and before finishing stage S13.

use std::path::Path;

use crate::step;

const MANIFEST: &str = "crates/codepack-ai-api/Cargo.toml";

/// Formats, lints and tests the excluded crate with the same denials the workspace uses.
///
/// The lint levels are duplicated in that crate's own manifest, because an excluded
/// package cannot inherit `[workspace.lints]`. Running clippy here with `-D warnings` is
/// what proves the duplicate is still doing its job rather than quietly drifting.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    step(
        root,
        "ai-api: format",
        "cargo",
        &["fmt", "--manifest-path", MANIFEST, "--", "--check"],
    )?;

    step(
        root,
        "ai-api: clippy",
        "cargo",
        &[
            "clippy",
            "--manifest-path",
            MANIFEST,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    step(
        root,
        "ai-api: tests",
        "cargo",
        &["test", "--manifest-path", MANIFEST],
    )?;

    println!(
        "\nai-api ok. Reminder: `cargo xtask gate` does not run this — the crate is \
         excluded from the workspace so that `keyring` and `ureq` stay out of every \
         platform's build."
    );
    Ok(())
}

/// Both tests here are against the real repository rather than a synthetic fixture —
/// unlike the other xtask checks, this command has no fixture-driven unit test to keep
/// apart from them (audit 2026-09-07, T-12): the module name says so directly instead
/// of leaving a reader to notice by reading each test's own body.
#[cfg(test)]
mod against_the_real_repository {
    use super::*;

    /// The manifest this command drives has to exist, or the command silently checks
    /// nothing — which is the failure mode the exclusion already risks.
    #[test]
    fn the_excluded_manifest_is_where_this_command_expects_it() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root is two levels above this crate");
        assert!(
            root.join(MANIFEST).is_file(),
            "{MANIFEST} is missing; `cargo xtask ai-api` would check nothing"
        );
    }

    /// And it really is excluded. If somebody adds it back to `members`, this command
    /// becomes redundant and the exclusion's whole reason has quietly gone.
    #[test]
    fn the_crate_is_still_excluded_from_the_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root is two levels above this crate");
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("root manifest");
        let parsed: toml::Table = manifest.parse().expect("root manifest is valid TOML");

        let excluded = parsed
            .get("workspace")
            .and_then(|workspace| workspace.get("exclude"))
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .any(|entry| entry == "crates/codepack-ai-api")
            })
            .unwrap_or(false);

        assert!(
            excluded,
            "crates/codepack-ai-api must stay in workspace.exclude — see Q41"
        );
    }

    /// The version is the one thing the exclusion cannot keep honest by itself.
    ///
    /// Every workspace member inherits `version.workspace = true`, so a release bump
    /// moves all of them at once. This crate spells its version out, because an excluded
    /// package cannot inherit — and nothing until now compared the two. A release would
    /// simply leave it a version behind, and the first sign would be a stale number in
    /// whatever S13 eventually ships. Added during the 2.0.1 release, where it had
    /// already happened once.
    #[test]
    fn the_excluded_crate_carries_the_same_version_as_the_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root is two levels above this crate");

        let version_of = |manifest: toml::Table, table: &str| -> Option<String> {
            manifest
                .get(table)
                .and_then(|section| section.get("package"))
                .or_else(|| manifest.get(table))
                .and_then(|package| package.get("version"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        };

        let workspace: toml::Table = std::fs::read_to_string(root.join("Cargo.toml"))
            .expect("root manifest")
            .parse()
            .expect("root manifest is valid TOML");
        let excluded: toml::Table = std::fs::read_to_string(root.join(MANIFEST))
            .expect("excluded manifest")
            .parse()
            .expect("excluded manifest is valid TOML");

        let workspace_version =
            version_of(workspace, "workspace").expect("[workspace.package] version");
        let excluded_version = version_of(excluded, "package").expect("[package] version");

        assert_eq!(
            excluded_version, workspace_version,
            "codepack-ai-api is version {excluded_version} while the workspace is \
             {workspace_version}; an excluded package cannot inherit the bump, so it has \
             to be moved by hand in crates/codepack-ai-api/Cargo.toml"
        );
    }
}
