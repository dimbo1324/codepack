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
//! `Cargo.toml`'s version disagree, or the file on disk stops matching its own recorded
//! checksum. It cannot prove `setup.exe` was built from the commit it sits in (that would
//! mean rebuilding the installer inside the gate); it proves the two files agree with
//! each other and with the workspace version, which is the part a rushed release most
//! often gets wrong.

use std::path::Path;

use sha2::{Digest, Sha256};

/// `setup.exe`'s permanent name — referenced by the README link, never renamed.
const INSTALLER_FILE: &str = "setup.exe";
const INFO_FILE: &str = "SETUP.txt";

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn git_head_commit(root: &Path) -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
        // Packaging must not fail just because the tree it is building from is not a
        // git checkout (a source tarball, say) — `SETUP.txt` says so honestly instead.
        .unwrap_or_else(|| "unknown".to_string())
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

    let installer = std::fs::read_dir(&nsis_dir)
        .map_err(|error| format!("cannot list {}: {error}", nsis_dir.display()))?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("exe"))
        .ok_or_else(|| format!("{} produced no .exe to publish", nsis_dir.display()))?;

    let bytes = std::fs::read(&installer)
        .map_err(|error| format!("cannot read {}: {error}", installer.display()))?;
    let sha256 = sha256_hex(&bytes);

    let setup_exe = root.join(INSTALLER_FILE);
    std::fs::write(&setup_exe, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", setup_exe.display()))?;

    let version = env!("CARGO_PKG_VERSION");
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

/// What [`parse_info`] reads back out of `SETUP.txt` — only the two fields the gate
/// actually checks.
struct InstallerInfo {
    version: String,
    sha256: String,
}

/// A deliberately plain line-by-line reader, not a format parser: `SETUP.txt` is prose
/// for a person first, so this reads past everything it does not recognize rather than
/// rejecting a file whose wording changes around the two lines that matter.
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

    fn fake_nsis_bundle(bundle_root: &Path, exe_bytes: &[u8]) -> PathBuf {
        let nsis = bundle_root.join("nsis");
        std::fs::create_dir_all(&nsis).unwrap();
        let exe = nsis.join("codepack_9.9.9_x64-setup.exe");
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
