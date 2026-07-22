//! Headless command-line interface for codepack.
//!
//! Implemented in stage S10 — see `ROADMAP.md`. The binary exists from stage S0 so the
//! workspace, CI, and packaging paths are exercised before the interface lands.

// A CLI is expected to write to stdout; the workspace lint targets library crates.
#![allow(clippy::print_stdout)]

fn main() {
    println!("codepack {} — not implemented yet (stage S10).", env!("CARGO_PKG_VERSION"));
}
