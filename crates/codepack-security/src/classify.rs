//! Text/binary classification.
//!
//! This crate carried its own copy of `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS`/
//! `TEXT_FILENAMES_WITHOUT_EXTENSION` and the two classification functions, because
//! ROADMAP's dependency table let S3 depend only on S1 and `codepack-scanner` was not
//! available to it. The duplicate was documented as tech debt (open question Q7) and is
//! removed now that the definition lives in `codepack-core`, which this crate already
//! depends on. Both copies were verified identical entry for entry before the move.
//!
//! The names stay re-exported so existing `codepack_security::` imports keep working.

pub use codepack_core::classify::{
    BINARY_EXTENSIONS, BINARY_SAMPLE_BYTES, TEXT_EXTENSIONS, TEXT_FILENAMES_WITHOUT_EXTENSION,
    looks_binary, should_consider_text_file,
};
