//! Task runner and quality gate for the codepack workspace.
//!
//! Run with `cargo xtask <command>`; see `.ai/project/11-commands.md`.

// A task runner is expected to write to stdout; the workspace lint targets libraries.
#![allow(clippy::print_stdout)]

mod sync_agents;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
codepack task runner

Usage: cargo xtask <command> [options]

Commands:
  gate [--quick]          Full quality gate; --quick skips the test run
  fmt                     Format Rust sources
  lint                    Clippy across the workspace with warnings denied
  test                    Run workspace tests
  sync-agents [--check]   Regenerate AGENTS.md from the .ai/ modules
  doctor                  Read-only environment diagnostics
";

fn repo_root() -> PathBuf {
    // crates/xtask -> crates -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask always lives two levels below the repository root")
        .to_path_buf()
}

fn run(root: &Path, program: &str, args: &[&str]) -> bool {
    println!("$ {program} {}", args.join(" "));
    match Command::new(program).args(args).current_dir(root).status() {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("failed to launch `{program}`: {error}");
            false
        }
    }
}

fn step(root: &Path, label: &str, program: &str, args: &[&str]) -> Result<(), String> {
    println!("\n=== {label} ===");
    if run(root, program, args) {
        Ok(())
    } else {
        Err(label.to_string())
    }
}

fn gate(root: &Path, quick: bool) -> Result<(), String> {
    step(root, "format", "cargo", &["fmt", "--all", "--check"])?;
    step(
        root,
        "clippy",
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    if !quick {
        step(root, "tests", "cargo", &["test", "--workspace"])?;
    }
    println!("\n=== agents sync ===");
    sync_agents::run(root, true).map_err(|error| format!("sync-agents: {error}"))?;
    Ok(())
}

fn doctor(root: &Path) {
    println!("repository root: {}", root.display());
    for (label, program, args) in [
        ("cargo", "cargo", ["--version"]),
        ("rustc", "rustc", ["--version"]),
        ("node", "node", ["--version"]),
        ("pnpm", "pnpm", ["--version"]),
    ] {
        let reported = Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
        match reported {
            Some(version) => println!("  {label:<6} {version}"),
            None => println!("  {label:<6} not found"),
        }
    }
    let tauri = root.join("apps/desktop/src-tauri");
    println!(
        "  tauri  {}",
        if tauri.is_dir() {
            "present"
        } else {
            "absent (arrives in stage S11)"
        }
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        println!("{USAGE}");
        return ExitCode::from(2);
    };
    let root = repo_root();
    let has = |flag: &str| args.iter().any(|arg| arg == flag);

    let outcome = match command.as_str() {
        "gate" => gate(&root, has("--quick")),
        "fmt" => step(&root, "format", "cargo", &["fmt", "--all"]),
        "lint" => step(
            &root,
            "clippy",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        "test" => step(&root, "tests", "cargo", &["test", "--workspace"]),
        "sync-agents" => {
            sync_agents::run(&root, has("--check")).map_err(|error| format!("sync-agents: {error}"))
        }
        "doctor" => {
            doctor(&root);
            Ok(())
        }
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        unknown => {
            eprintln!("unknown command: {unknown}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failed) => {
            eprintln!("\nFAILED: {failed}");
            ExitCode::FAILURE
        }
    }
}
