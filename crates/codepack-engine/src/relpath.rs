//! Shared helper: turns a backslash-joined `PlannedFile.relative_path` (this project's
//! join convention, the same on every OS — see `codepack_scanner::plan`) into real path
//! components. Never `Path::new(rel_str)` directly: on Linux/macOS a backslash is an
//! ordinary filename character, not a separator, which would silently corrupt every
//! nested path.
//!
//! Shared by [`crate::copy`] (pipeline step 2, copying planned files onto disk) and
//! [`crate::analytics`] (pipeline step 6, turning the re-plan's planned files back into
//! a `Vec<PathBuf>` for `codepack_security::scan_project`) — both call sites must never
//! silently diverge on this conversion.

use std::path::PathBuf;

pub(crate) fn to_relative_path(relative_path: &str) -> PathBuf {
    relative_path.split('\\').collect()
}
