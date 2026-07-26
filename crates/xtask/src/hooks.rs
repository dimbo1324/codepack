//! Installs the repository's git hooks.
//!
//! The hooks live in a tracked `.githooks/` directory and are activated by pointing
//! `core.hooksPath` at it. `.git/hooks/` is deliberately not used: it is untracked, so a
//! hook written there exists only on the machine that wrote it and silently disappears on
//! every fresh clone — which is the failure mode that makes teams stop trusting hooks.

use std::path::Path;
use std::process::Command;

const HOOKS_DIR: &str = ".githooks";

pub(crate) fn install(root: &Path) -> Result<(), String> {
    let hooks = root.join(HOOKS_DIR);
    if !hooks.join("pre-commit").is_file() {
        return Err(format!(
            "{}/pre-commit is missing — the hook is tracked in the repository, so this \
             means the checkout is incomplete rather than that setup is needed",
            HOOKS_DIR
        ));
    }

    println!("$ git config core.hooksPath {HOOKS_DIR}");
    let status = Command::new("git")
        .args(["config", "core.hooksPath", HOOKS_DIR])
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to launch `git`: {error}"))?;
    if !status.success() {
        return Err("git config core.hooksPath failed".to_string());
    }

    println!(
        "\npre-commit hook active. It formats the files you stage and re-stages them.\n\
         Bypass once with `git commit --no-verify`; the gate still checks formatting later."
    );
    Ok(())
}
