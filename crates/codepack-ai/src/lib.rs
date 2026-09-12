//! Stage S13 — the offline half of the AI integration.
//!
//! Claude Code and Codex already read the filesystem, so they get a prompt file next to
//! the bundle and a command to run: no network, no key, invariant I1 untouched. That is
//! all this crate does, and it is the only part either front end uses.
//!
//! ## Where the API path went
//!
//! It was the `api` feature here, on by default, and it is now the separate
//! `codepack-ai-api` crate. Both front ends already took this crate with
//! `default-features = false`, so no binary ever linked a transport; but a workspace
//! member is compiled with its own defaults by `cargo test --workspace`, so `keyring`
//! and `ureq` were built on every platform for code no user could reach, and on Linux
//! `keyring` wants a Secret Service backend. A dead path was obstructing the build
//! everywhere but Windows, so the split became a crate boundary rather than a feature
//! flag (audit 2026-09-05 No. 26; owner decision 2026-09-06, Q41).
//!
//! That crate then sat outside the workspace entirely until 2026-09-12, when the owner
//! chose to finish the stage rather than keep a dead path alive. It has a command and a
//! screen now, so it is back in the product and gated like every other member. The
//! boundary stayed, because it is what keeps the transport in one crate the
//! `network isolation` gate step can name — invariant I1's single exception, reachable
//! only from the two front ends and never from under the export pipeline.
//!
//! Nothing here starts on its own. [`handoff::prepare`] writes one Markdown file when a
//! user asks for it.

pub mod error;
pub mod handoff;

pub use error::{AiError, Refusal};
pub use handoff::{AGENTS, Handoff, LocalAgent};
