//! Text/binary classification.
//!
//! The implementation and its three constant sets moved to `codepack-core` when open
//! question Q7 was closed (owner decision 2026-07-25): they were duplicated here and in
//! `codepack-security`, because ROADMAP's dependency table let S3 depend only on S1.
//! Both copies were verified identical entry for entry before the move.
//!
//! The names stay re-exported from this crate so existing `codepack_scanner::` imports
//! keep working — the point of the move was to remove a second definition, not to make
//! callers chase it.

pub use codepack_core::classify::{BINARY_SAMPLE_BYTES, looks_binary, should_consider_text_file};
