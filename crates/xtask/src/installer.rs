//! The root-level installer duplicate (audit 2026-09-07, D-1: an owner request, verbatim
//! — the `.exe` `cargo xtask package` already produces should also sit at the repository
//! root under a permanent, friendly name, tracked in git, so a person can find and
//! download it from GitHub without knowing `target/release/bundle/...` exists.
//!
//! **The trade this makes, on purpose.** A binary in git is permanent: every clone
//! downloads every version that ever existed, not only the latest. `docs/__arch__/open-
//! questions.md`'s Q44 records the owner's decision and that cost; a Release or Git LFS
//! moves the same duplicate off the clone path without changing anything downstream of
//! this module, should that cost stop being worth it later.
//!
//! **Windows only, and only the NSIS installer.** The owner asked specifically for the
//! `.exe`; each Linux format (`deb`/`rpm`/`AppImage`) kept here would be one more binary
//! in git history forever for a convenience Windows already gets from one. [`publish`]
//! is a silent no-op wherever `tauri build` did not produce an `nsis` directory, driven
//! by which format directory actually exists rather than `cfg!(windows)`, so it can be
//! exercised by a test without pretending to be a different platform.
//!
//! **Staying fresh is the real risk, not size.** `setup.exe`'s name never changes from
//! release to release — that is the point, a stable README link — which means nothing
//! about the file itself says which version it is, or whether it was rebuilt after the
//! last version bump. [`SETUP_TXT`] carries that: version, build time, commit, source
//! path and a checksum, in the same plain `key: value` shape already established by
//! `SHA256SUMS.txt`. [`check_gate`] is the enforcement half — a gate step, in the same
//! genre as `sync-agents --check`, that fails the build the day `setup.exe` and
//! `Cargo.toml`'s version disagree, the file on disk stops matching its own recorded
//! checksum, or the recorded `source:` names a different version's installer than the one
//! being released. That last check exists because the first two proved insufficient on
//! their own: during 2.0.1 the wrong binary was published with entirely self-consistent
//! metadata over it (see [`pick_installer`]). It still cannot prove `setup.exe` was built
//! from the commit it sits in — that would mean rebuilding the installer inside the gate
//! — but it now proves the bytes came from a build of the version being released, which
//! is the part a rushed release most often gets wrong.

use std::path::Path;

use crate::{git_head_commit, sha256_hex};

/// `setup.exe`'s permanent name — referenced by the README link, never renamed.
const INSTALLER_FILE: &str = "setup.exe";
const INFO_FILE: &str = "SETUP.txt";

/// The installer for `version`, chosen by name rather than by whichever `.exe` the
/// directory happens to list first.
///
/// `target/release/bundle/nsis/` accumulates: `tauri build` writes
/// `codepack_<version>_x64-setup.exe` and never removes the previous one, so after a
/// version bump the directory holds both. Taking the first entry — which is what this
/// did until 2026-09-08 — meant publishing `codepack_2.0.0_x64-setup.exe` while
/// `SETUP.txt`, whose version comes from `CARGO_PKG_VERSION`, declared 2.0.1. The gate
/// could not catch it either: it compares `SETUP.txt`'s version against `Cargo.toml`
/// (both 2.0.1) and the file's checksum against `SETUP.txt`'s (both taken from the same
/// stale file), so both halves agreed with each other while describing the wrong binary.
/// It was found by reading the build output, which named one file and published another.
///
/// Matching on the version makes the mismatch impossible instead of merely detectable,
/// and a build that produced no matching installer now says so — naming what it did find,
/// because "no installer" and "an installer for a different version" need different
/// fixes.
fn pick_installer(nsis_dir: &Path, version: &str) -> Result<std::path::PathBuf, String> {
    let mut executables: Vec<std::path::PathBuf> = std::fs::read_dir(nsis_dir)
        .map_err(|error| format!("cannot list {}: {error}", nsis_dir.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("exe"))
        .collect();
    executables.sort();

    if let Some(matching) = executables.iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(version))
    }) {
        return Ok(matching.clone());
    }

    if executables.is_empty() {
        return Err(format!(
            "{} produced no .exe to publish",
            nsis_dir.display()
        ));
    }
    let found: Vec<String> = executables
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .map(str::to_string)
        .collect();
    Err(format!(
        "{} holds no installer for version {version} — found {found:?}. The bundler \
         names its output after the version, so this means the build did not produce \
         {version}: rebuild, or clear the stale installers out of that directory.",
        nsis_dir.display()
    ))
}

/// Copies the NSIS installer from `bundle_root/nsis/*.exe` to `root/setup.exe`, and
/// writes `root/SETUP.txt` describing it. Does nothing, successfully, when
/// `bundle_root/nsis` was never produced — the normal case on every platform but
/// Windows.
pub(crate) fn publish(root: &Path, bundle_root: &Path) -> Result<(), String> {
    let nsis_dir = bundle_root.join("nsis");
    if !nsis_dir.is_dir() {
        return Ok(());
    }

    let version = env!("CARGO_PKG_VERSION");
    let installer = pick_installer(&nsis_dir, version)?;

    let bytes = std::fs::read(&installer)
        .map_err(|error| format!("cannot read {}: {error}", installer.display()))?;
    let sha256 = sha256_hex(&bytes);

    let setup_exe = root.join(INSTALLER_FILE);
    std::fs::write(&setup_exe, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", setup_exe.display()))?;

    let commit = git_head_commit(root);
    let built_at = codepack_core::time::UtcDateTime::now().format_iso8601_utc();
    let source = installer.strip_prefix(root).unwrap_or(&installer).display();

    let info = format!(
        "codepack setup\n\
         ------------------------------------------------------------\n\
         file:      {INSTALLER_FILE}\n\
         version:   {version}\n\
         platform:  Windows x64 (NSIS, install for the current user)\n\
         built:     {built_at}\n\
         commit:    {commit}\n\
         source:    {source}\n\
         sha256:    {sha256}\n\
         \n\
         Verify before running (PowerShell):\n\
         \x20   Get-FileHash .\\{INSTALLER_FILE} -Algorithm SHA256\n\
         \n\
         This installer is unsigned: SmartScreen will warn about an unknown publisher.\n\
         Signing is planned for stage S14.\n"
    );
    let info_path = root.join(INFO_FILE);
    std::fs::write(&info_path, &info)
        .map_err(|error| format!("cannot write {}: {error}", info_path.display()))?;

    println!("\n{INSTALLER_FILE}: published from {source} ({version}, sha256 {sha256})");
    Ok(())
}

/// What [`parse_info`] reads back out of `SETUP.txt` — only the three fields the gate
/// actually checks.
struct InstallerInfo {
    version: String,
    sha256: String,
    /// The bundler-named file this was copied from. Carries the version in its own name,
    /// which is what makes the "published the wrong build" check below possible.
    source: String,
}

/// A deliberately plain line-by-line reader, not a format parser: `SETUP.txt` is prose
/// for a person first, so this reads past everything it does not recognize rather than
/// rejecting a file whose wording changes around the three lines that matter.
fn parse_info(text: &str) -> Result<InstallerInfo, String> {
    let field = |prefix: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(prefix))
            .map(|value| value.trim().to_string())
            .ok_or_else(|| format!("{INFO_FILE} has no `{}` line", prefix.trim_end()))
    };
    Ok(InstallerInfo {
        version: field("version:")?,
        sha256: field("sha256:")?,
        source: field("source:")?,
    })
}

/// The gate step: `setup.exe` and `SETUP.txt` must exist, agree with each other, and
/// agree with the workspace version — the exact situation this exists to catch is a
/// version bump that never rebuilt the installer.
///
/// Cannot confirm `setup.exe` was built from the commit it now sits in; building the
/// installer inside the gate to check that would cost the ten minutes this check exists
/// to avoid. Said in the check's own passing message rather than left implicit, so a
/// green gate does not read as a stronger guarantee than it is.
pub(crate) fn check_gate(root: &Path) -> Result<(), String> {
    check_gate_at(root, env!("CARGO_PKG_VERSION"))
}

fn check_gate_at(root: &Path, expected_version: &str) -> Result<(), String> {
    let setup_exe = root.join(INSTALLER_FILE);
    let info_path = root.join(INFO_FILE);

    if !setup_exe.is_file() && !info_path.is_file() {
        // Neither file exists: D-1 has not landed in this checkout yet (or never will,
        // on a fork that dropped it). Nothing to check, and nothing wrong either.
        return Ok(());
    }
    if !setup_exe.is_file() {
        return Err(format!("{INFO_FILE} exists but {INSTALLER_FILE} does not"));
    }
    if !info_path.is_file() {
        return Err(format!("{INSTALLER_FILE} exists but {INFO_FILE} does not"));
    }

    let text = std::fs::read_to_string(&info_path)
        .map_err(|error| format!("cannot read {}: {error}", info_path.display()))?;
    let info = parse_info(&text)?;

    if info.version != expected_version {
        return Err(format!(
            "{INFO_FILE} says version {} but Cargo.toml says {expected_version} — \
             {INSTALLER_FILE} was not rebuilt after the version bump",
            info.version
        ));
    }

    // The version line alone cannot catch publishing the wrong build: it is written from
    // `CARGO_PKG_VERSION`, so it says the right thing even when the bytes beside it came
    // from a previous version's installer still sitting in the bundle directory. The
    // bundler puts the version in the file's own name, so the recorded source is the one
    // field that describes the *bytes* rather than the build that copied them. Added
    // 2026-09-08, after exactly that happened during the 2.0.1 release.
    if !info.source.contains(expected_version) {
        return Err(format!(
            "{INFO_FILE} records version {expected_version} but was published from \
             `{}`, which is a different version's installer — {INSTALLER_FILE} holds the \
             wrong build. Re-run `cargo xtask package`.",
            info.source
        ));
    }

    let bytes = std::fs::read(&setup_exe)
        .map_err(|error| format!("cannot read {}: {error}", setup_exe.display()))?;
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != info.sha256 {
        return Err(format!(
            "{INSTALLER_FILE}'s sha256 ({actual_sha256}) does not match the one recorded \
             in {INFO_FILE} ({}) — the file was replaced or corrupted after publishing",
            info.sha256
        ));
    }

    println!(
        "{INSTALLER_FILE}: version {} matches Cargo.toml, sha256 matches {INFO_FILE} \
         (this does not prove it was built from the current commit)",
        info.version
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Named after the current version, because the real bundler is: `publish` picks the
    /// installer whose name carries the version it is publishing, so a fixture with an
    /// arbitrary version would exercise the failure path rather than the ordinary one.
    fn fake_nsis_bundle(bundle_root: &Path, exe_bytes: &[u8]) -> PathBuf {
        let nsis = bundle_root.join("nsis");
        std::fs::create_dir_all(&nsis).unwrap();
        let exe = nsis.join(format!(
            "codepack_{}_x64-setup.exe",
            env!("CARGO_PKG_VERSION")
        ));
        std::fs::write(&exe, exe_bytes).unwrap();
        exe
    }

    #[test]
    fn no_nsis_directory_is_a_silent_no_op() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();

        publish(root.path(), bundle_root.path()).unwrap();

        assert!(!root.path().join(INSTALLER_FILE).exists());
        assert!(!root.path().join(INFO_FILE).exists());
    }

    #[test]
    fn the_nsis_installer_is_copied_and_described() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        fake_nsis_bundle(bundle_root.path(), b"pretend installer bytes");

        publish(root.path(), bundle_root.path()).unwrap();

        let copied = std::fs::read(root.path().join(INSTALLER_FILE)).unwrap();
        assert_eq!(copied, b"pretend installer bytes");

        let info = std::fs::read_to_string(root.path().join(INFO_FILE)).unwrap();
        assert!(info.contains(&format!("version:   {}", env!("CARGO_PKG_VERSION"))));
        assert!(info.contains(&sha256_hex(b"pretend installer bytes")));
        assert!(info.contains("unsigned"));
    }

    #[test]
    fn an_nsis_directory_with_no_exe_in_it_is_an_error_naming_it() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(bundle_root.path().join("nsis")).unwrap();
        std::fs::write(bundle_root.path().join("nsis/SHA256SUMS.txt"), "").unwrap();

        let error = publish(root.path(), bundle_root.path()).unwrap_err();
        assert!(error.contains("nsis"), "{error}");
    }

    #[test]
    fn neither_file_present_passes_the_gate_as_not_applicable() {
        let root = tempfile::tempdir().unwrap();
        check_gate_at(root.path(), "1.0.0").unwrap();
    }

    #[test]
    fn a_matching_pair_passes_the_gate() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        fake_nsis_bundle(bundle_root.path(), b"matching bytes");
        publish(root.path(), bundle_root.path()).unwrap();

        check_gate_at(root.path(), env!("CARGO_PKG_VERSION")).unwrap();
    }

    /// The regression this pair exists for, found during the 2.0.1 release.
    ///
    /// `tauri build` never deletes the previous version's installer, so after a bump the
    /// directory holds both. Publishing whichever the filesystem listed first put the
    /// **older** binary in `setup.exe` while `SETUP.txt` — written from
    /// `CARGO_PKG_VERSION` — declared the new version over it.
    #[test]
    fn the_installer_for_the_current_version_wins_over_a_stale_one_beside_it() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        let nsis = bundle_root.path().join("nsis");
        std::fs::create_dir_all(&nsis).unwrap();

        // Sorts before the current version either way (2.0.1 today, and any later bump),
        // which is exactly the ordering that produced the defect.
        std::fs::write(nsis.join("codepack_0.0.1_x64-setup.exe"), b"stale build").unwrap();
        let current = nsis.join(format!(
            "codepack_{}_x64-setup.exe",
            env!("CARGO_PKG_VERSION")
        ));
        std::fs::write(&current, b"current build").unwrap();

        publish(root.path(), bundle_root.path()).unwrap();

        assert_eq!(
            std::fs::read(root.path().join(INSTALLER_FILE)).unwrap(),
            b"current build",
            "the stale installer was published over the current one"
        );
        check_gate_at(root.path(), env!("CARGO_PKG_VERSION")).unwrap();
    }

    /// And the gate now catches it even if something else republishes the wrong build:
    /// the recorded `source:` names the file the bytes came from, which carries its own
    /// version. Without this the two halves agree with each other while describing the
    /// wrong binary, which is why the defect above reached a release.
    #[test]
    fn a_setup_txt_published_from_another_versions_installer_fails_the_gate() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        fake_nsis_bundle(bundle_root.path(), b"bytes");
        publish(root.path(), bundle_root.path()).unwrap();

        // Rewrite only the source line, leaving version and checksum internally
        // consistent — precisely the shape the old check could not see through.
        let info_path = root.path().join(INFO_FILE);
        let text = std::fs::read_to_string(&info_path).unwrap();
        let tampered: String = text
            .lines()
            .map(|line| {
                if line.starts_with("source:") {
                    "source:    target/release/bundle/nsis/codepack_0.0.1_x64-setup.exe".to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&info_path, tampered).unwrap();

        let error = check_gate_at(root.path(), env!("CARGO_PKG_VERSION")).unwrap_err();
        assert!(error.contains("wrong build"), "{error}");
        assert!(error.contains("0.0.1"), "{error}");
    }

    /// A directory with installers but none for this version is a different problem from
    /// an empty one — a build that produced the wrong version rather than none at all —
    /// and the message says which, listing what it did find.
    #[test]
    fn no_installer_for_this_version_names_what_it_found_instead() {
        let nsis = tempfile::tempdir().unwrap();
        std::fs::write(nsis.path().join("codepack_0.0.1_x64-setup.exe"), b"other").unwrap();

        let error = pick_installer(nsis.path(), "2.0.1").unwrap_err();
        assert!(error.contains("2.0.1"), "{error}");
        assert!(error.contains("codepack_0.0.1_x64-setup.exe"), "{error}");
        assert!(error.contains("rebuild"), "{error}");
    }

    #[test]
    fn a_version_bump_without_a_rebuild_fails_the_gate_by_name() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        fake_nsis_bundle(bundle_root.path(), b"stale installer");
        publish(root.path(), bundle_root.path()).unwrap();

        let error = check_gate_at(root.path(), "99.0.0").unwrap_err();
        assert!(error.contains("99.0.0"), "{error}");
        assert!(error.contains(env!("CARGO_PKG_VERSION")), "{error}");
    }

    #[test]
    fn a_corrupted_installer_fails_the_gate_on_its_checksum() {
        let root = tempfile::tempdir().unwrap();
        let bundle_root = tempfile::tempdir().unwrap();
        fake_nsis_bundle(bundle_root.path(), b"original bytes");
        publish(root.path(), bundle_root.path()).unwrap();

        // Simulates the file being replaced or corrupted after `SETUP.txt` was written,
        // without the checksum inside `SETUP.txt` changing to match.
        std::fs::write(root.path().join(INSTALLER_FILE), b"tampered bytes").unwrap();

        let error = check_gate_at(root.path(), env!("CARGO_PKG_VERSION")).unwrap_err();
        assert!(error.contains("sha256"), "{error}");
    }

    #[test]
    fn setup_exe_without_setup_txt_is_named_as_the_missing_half() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(INSTALLER_FILE), b"orphan").unwrap();

        let error = check_gate_at(root.path(), "1.0.0").unwrap_err();
        assert!(error.contains(INFO_FILE), "{error}");
    }
}
