//! `cargo xtask golden` — regenerates the golden reference artifacts by running the
//! archived legacy implementation against `tests/golden/fixtures`.
//!
//! This is the one command in the workspace that needs Python, and the one that
//! executes `docs/__arch__/codepack-main.zip`. It is deliberately **not** part of
//! `gate`: CI never runs legacy, it only compares against the references this command
//! commits. See `tests/golden/generate_reference.py` for the comparison specification.
//!
//! The archive is unpacked with Python's own `zipfile` rather than a Rust crate on
//! purpose — `xtask` is intentionally dependency-free (see its `Cargo.toml`), and this
//! command already requires a Python interpreter, so using one for both steps costs
//! nothing and keeps the gate instant to build.

use std::path::Path;
use std::process::Command;

const LEGACY_ARCHIVE: &str = "docs/__arch__/codepack-main.zip";
const LEGACY_SRC_SUFFIX: &str = "codepack-main/src";
const FIXTURES_DIR: &str = "tests/golden/fixtures";
const REFERENCE_DIR: &str = "tests/golden/reference";
const GENERATOR: &str = "tests/golden/generate_reference.py";

fn python(root: &Path) -> Result<String, String> {
    for candidate in ["python", "python3", "py"] {
        let ok = Command::new(candidate)
            .arg("--version")
            .current_dir(root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if ok {
            return Ok(candidate.to_string());
        }
    }
    Err(
        "no Python interpreter found on PATH (tried python, python3, py); \
         golden regeneration needs one, the test suite does not"
            .to_string(),
    )
}

pub(crate) fn run(root: &Path) -> Result<(), String> {
    let interpreter = python(root)?;

    let archive = root.join(LEGACY_ARCHIVE);
    if !archive.is_file() {
        return Err(format!("legacy archive not found at {LEGACY_ARCHIVE}"));
    }

    let unpacked = root.join("target/golden-legacy");
    if unpacked.exists() {
        std::fs::remove_dir_all(&unpacked)
            .map_err(|error| format!("could not clear {}: {error}", unpacked.display()))?;
    }

    println!("=== unpacking legacy archive ===");
    let unpack = "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])";
    let status = Command::new(&interpreter)
        .arg("-c")
        .arg(unpack)
        .arg(&archive)
        .arg(&unpacked)
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to launch {interpreter}: {error}"))?;
    if !status.success() {
        return Err("unpacking the legacy archive failed".to_string());
    }

    let legacy_src = unpacked.join(LEGACY_SRC_SUFFIX);
    if !legacy_src.is_dir() {
        return Err(format!(
            "legacy sources not found at {} inside the archive",
            LEGACY_SRC_SUFFIX
        ));
    }

    println!("\n=== running legacy against the golden fixtures ===");
    let status = Command::new(&interpreter)
        .arg(root.join(GENERATOR))
        .arg(&legacy_src)
        .arg(root.join(FIXTURES_DIR))
        .arg(root.join(REFERENCE_DIR))
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to launch {interpreter}: {error}"))?;
    if !status.success() {
        return Err("legacy reference generation failed".to_string());
    }

    let _ = std::fs::remove_dir_all(&unpacked);
    println!("\nreferences written to {REFERENCE_DIR}");
    println!("review the diff before committing: these files are the parity contract");
    Ok(())
}
