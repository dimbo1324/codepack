//! The pnpm half of the toolchain: Prettier formatting, and the frontend gate checks.
//!
//! Everything here is **optional by design**. A fresh clone that has never run
//! `pnpm install` must still be able to run `cargo xtask gate` and get a meaningful
//! answer about the Rust side, because most work in this repository never touches the
//! frontend. So a missing `node_modules` prints a notice and skips, rather than failing
//! the gate on an absent dependency the task did not need. CI installs dependencies, so
//! there the checks always really run.

use std::path::Path;
use std::process::Command;

/// pnpm ships as `pnpm.CMD` on Windows, and `std::process::Command` only ever appends
/// `.exe` when resolving a bare name — so naming it `pnpm` fails with "program not found"
/// on the one platform this project currently targets.
const PNPM: &str = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

/// Prettier lives in the workspace root's `node_modules`; `svelte-check` and ESLint live
/// in the UI package's. One `pnpm install` produces both, so both are checked — a partial
/// install should skip loudly rather than fail halfway through the gate.
pub(crate) fn dependencies_installed(root: &Path) -> bool {
    root.join("node_modules").is_dir() && root.join("apps/desktop/ui/node_modules").is_dir()
}

pub(crate) fn skip_notice() {
    println!(
        "  skipped: apps/desktop/ui/node_modules is missing.\n  \
         Run `pnpm install` to include the frontend in this command."
    );
}

fn pnpm(root: &Path, label: &str, args: &[&str]) -> Result<(), String> {
    println!("$ {PNPM} {}", args.join(" "));
    let status = Command::new(PNPM)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("{label}: failed to launch `{PNPM}`: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(label.to_string())
    }
}

/// Rewrites frontend and configuration files in place. Paired with `cargo fmt --all` by
/// `cargo xtask fmt`, so one command covers both toolchains.
pub(crate) fn format_write(root: &Path) -> Result<(), String> {
    pnpm(root, "prettier", &["run", "format:write"])
}

/// The frontend's share of the gate, in cheapest-first order so an obvious formatting
/// slip does not wait behind a full type-check.
pub(crate) fn gate_checks(root: &Path) -> Result<(), String> {
    pnpm(root, "prettier --check", &["run", "format"])?;
    pnpm(
        root,
        "svelte-check",
        &["--filter", "@codepack/ui", "typecheck"],
    )?;
    pnpm(root, "eslint", &["--filter", "@codepack/ui", "lint"])
}

/// Builds the Windows installer: frontend bundle, release binary, then the NSIS package.
///
/// Runs from `apps/desktop`, and that detail is the whole trick. The Tauri CLI locates a
/// project by looking for `tauri.conf.json` in a *subfolder* of the working directory, and
/// this repository keeps the shell in `apps/desktop/src-tauri` beside the frontend in
/// `apps/desktop/ui`. Invoked from `ui`, the config is a sibling rather than a child and
/// the CLI aborts with "couldn't recognize the current folder as a Tauri project" — which
/// is exactly what the previously documented `--filter @codepack/ui exec tauri dev`
/// command did. `apps/desktop` is the directory that contains both halves.
///
/// The CLI is a pnpm dev dependency rather than a global install, so the version that
/// builds a release is pinned by `pnpm-lock.yaml` like everything else. `tauri build` runs
/// `beforeBuildCommand` itself, so the frontend is not built twice.
pub(crate) fn package(root: &Path) -> Result<(), String> {
    if !dependencies_installed(root) {
        return Err("packaging needs the frontend toolchain: run `pnpm install` first".to_string());
    }
    pnpm(
        &root.join("apps/desktop"),
        "tauri build",
        &["exec", "tauri", "build"],
    )?;

    let bundle = root.join("target/release/bundle/nsis");
    match std::fs::read_dir(&bundle) {
        Ok(entries) => {
            println!("\ninstaller(s) in {}:", bundle.display());
            for entry in entries.flatten() {
                let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
                println!(
                    "  {} ({:.1} MiB)",
                    entry.file_name().to_string_lossy(),
                    size as f64 / (1024.0 * 1024.0)
                );
            }
            Ok(())
        }
        // The build reported success, so a missing directory means the bundler wrote
        // somewhere unexpected rather than that packaging failed. Say which, instead of
        // claiming an installer exists.
        Err(error) => Err(format!(
            "tauri build succeeded but {} is unreadable: {error}",
            bundle.display()
        )),
    }
}
