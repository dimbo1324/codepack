//! Enforces invariant I1: `codepack-ai` is the only crate allowed to reach the network.
//!
//! The invariant already existed as a written rule. A rule everyone has to remember is
//! a rule that eventually gets forgotten by whoever adds a dependency in a hurry, and
//! this particular lapse would be invisible — a crate gaining an HTTP client changes no
//! behaviour until the day it makes a request. So the build checks it, the same way the
//! desktop enforces filesystem isolation with capabilities rather than with a convention.
//!
//! The check reads manifests, not code. That is the right layer: a crate cannot make an
//! HTTP request without declaring a client as a dependency, and a manifest is
//! unambiguous where a source grep would drown in comments, doc examples, and the word
//! "http" appearing in a URL.

use std::path::Path;

/// The one crate permitted a network client. See `crates/codepack-ai/src/lib.rs`.
const ALLOWED: &str = "codepack-ai";

/// Dependencies that can perform network I/O.
///
/// Deliberately a denylist of known clients rather than an attempt at completeness: it
/// catches the realistic mistake — somebody reaching for the crate they always reach for
/// — without pretending to prove a negative. `git2` is absent on purpose: S4 pins it to
/// `default-features = false` precisely to exclude its network features, and that
/// narrowing is verified by the manifest it already lives in.
const NETWORK_CRATES: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "isahc",
    "surf",
    "attohttpc",
    "curl",
    "tonic",
    "actix-web",
    "axum",
    "warp",
    "tiny_http",
];

/// Check every workspace manifest, naming each crate that declares a network client.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let mut offenders: Vec<String> = Vec::new();

    for manifest in workspace_manifests(root)? {
        let Some(package) = manifest
            .file_name()
            .and_then(|_| package_dir_name(&manifest))
        else {
            continue;
        };
        if package == ALLOWED {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;

        for dependency in declared_dependencies(&text) {
            if NETWORK_CRATES.contains(&dependency.as_str()) {
                offenders.push(format!("{package} depends on {dependency}"));
            }
        }
    }

    if offenders.is_empty() {
        println!("network isolation ok: only {ALLOWED} may reach the network (invariant I1).");
        return Ok(());
    }

    Err(format!(
        "invariant I1 violated — a crate other than {ALLOWED} declares a network \
         client:\n  {}\n\nAll analysis is local. If a new stage genuinely needs the \
         network, that is an owner decision recorded in \
         docs/decisions/open-questions.md, not a dependency added in passing.",
        offenders.join("\n  ")
    ))
}

/// Every `Cargo.toml` under `crates/` plus the desktop shell's own.
fn workspace_manifests(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut manifests = Vec::new();

    let crates_dir = root.join("crates");
    let entries = std::fs::read_dir(&crates_dir)
        .map_err(|error| format!("could not read {}: {error}", crates_dir.display()))?;
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }

    let desktop = root.join("apps/desktop/src-tauri/Cargo.toml");
    if desktop.is_file() {
        manifests.push(desktop);
    }

    Ok(manifests)
}

fn package_dir_name(manifest: &Path) -> Option<String> {
    manifest
        .parent()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// Dependency names declared in a manifest.
///
/// A deliberately small parser rather than a TOML dependency: it reads the key on the
/// left of `=` inside any `[*dependencies*]` table, which is the whole shape that
/// matters here. Inline tables (`dep = { ... }`) and plain versions both reduce to the
/// same key.
fn declared_dependencies(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_dependencies = false;

    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line.contains("dependencies");
            continue;
        }
        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_names_are_read_from_every_dependency_table() {
        let manifest = r#"
[package]
name = "example"
reqwest = "not a dependency, this is the package table"

[dependencies]
serde = "1"
ureq = { version = "3", features = ["rustls"] }

[dev-dependencies]
tempfile = "3"

[target.'cfg(windows)'.dependencies]
windows-sys = "0.61"
"#;
        let names = declared_dependencies(manifest);
        assert!(names.contains(&"serde".to_string()));
        assert!(names.contains(&"ureq".to_string()));
        assert!(names.contains(&"tempfile".to_string()));
        assert!(names.contains(&"windows-sys".to_string()));
        // The key in `[package]` must not be mistaken for a dependency.
        assert_eq!(names.iter().filter(|n| *n == "reqwest").count(), 0);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let manifest = "[dependencies]\n# reqwest = \"0.13\"\n\nserde = \"1\"\n";
        assert_eq!(declared_dependencies(manifest), vec!["serde".to_string()]);
    }

    /// Build a throwaway workspace tree. `xtask` is deliberately dependency-free (its
    /// own manifest says so), so this uses the process id rather than `tempfile`.
    fn scratch_workspace(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("codepack-i1-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("crates")).unwrap();
        root
    }

    fn write_crate(root: &Path, name: &str, manifest: &str) {
        let dir = root.join("crates").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
    }

    #[test]
    fn a_network_client_in_an_ordinary_crate_fails_the_check() {
        // The guard is worthless unless it actually fires. Planting the dependency in
        // the real repository cannot prove this: `reqwest` drags in a tree that fails
        // the licence step first, so the gate dies before reaching here.
        let root = scratch_workspace("offender");
        write_crate(
            &root,
            "codepack-scanner",
            "[dependencies]\nreqwest = \"0.13\"\n",
        );

        let error = check(&root).unwrap_err();
        assert!(
            error.contains("codepack-scanner depends on reqwest"),
            "{error}"
        );
        assert!(error.contains("invariant I1"), "{error}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_allowed_crate_may_declare_one() {
        let root = scratch_workspace("allowed");
        write_crate(&root, ALLOWED, "[dependencies]\nureq = \"3\"\n");
        write_crate(&root, "codepack-core", "[dependencies]\nserde = \"1\"\n");

        assert!(check(&root).is_ok());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_listed_client_is_detected() {
        // A denylist that silently stopped matching an entry would be worse than none.
        for client in NETWORK_CRATES {
            let root = scratch_workspace(client);
            write_crate(
                &root,
                "codepack-core",
                &format!("[dependencies]\n{client} = \"1\"\n"),
            );
            assert!(check(&root).is_err(), "{client} was not detected");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn the_real_workspace_passes_its_own_check() {
        // The guard is only worth having if it runs against reality — a check that only
        // ever sees synthetic input proves nothing about this repository.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        check(&root).expect("the workspace must satisfy invariant I1");
    }
}
