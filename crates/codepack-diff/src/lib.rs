//! Differential export selection and project snapshots (BLUEPRINT §A.6).
//!
//! ## Scope boundary (stage S4, `ROADMAP.md`)
//!
//! This crate depends only on `codepack-core`. It does not walk with
//! `codepack-scanner`'s ignore/stack rules and does not consult
//! `codepack-security`'s safety modes: the ignored-directory-name set and the
//! "is this a countable text file" predicate `snapshot_project` needs are supplied by
//! the caller (a future engine stage), never hardcoded here a third time
//! (`docs/decisions/open-questions.md` Q7).
//!
//! `last_export` mode takes the previous snapshot as a plain function argument
//! (`Option<&Snapshot>`); this crate never looks one up from disk or a database —
//! that persistence lives in `codepack-storage`, stage S5. Legacy's `incremental.py`
//! mtime/size baseline mechanism has zero real callers outside its own test file
//! (`resolve_incremental_selection`/`save_incremental_baseline` are only ever invoked
//! from `tests/test_incremental.py`; every production call site hardcodes
//! `IncrementalSelection(enabled=False)`) and is intentionally not ported.
//! `diff_target_ref` is a confirmed-dead legacy config field (`git_ref` mode always
//! diffs `base..HEAD`, never `base..target`); parity means replicating that
//! limitation, not fixing it — see [`DiffOptions`].
//!
//! Combining diff-selection with force-include/-exclude rules or an incremental-mtime
//! selector is `codepack-engine`'s job in a later stage (S9's
//! `combined_selected_paths`), not this crate's.

mod error;
mod normalize;
mod options;
pub mod report;
pub mod selection;
pub mod snapshot;

pub use error::{DiffError, Result};
pub use options::DiffOptions;
pub use report::write_diff_report;
pub use selection::{
    DiffFile, DiffSelection, FileStatus, all_selection, diff_against_snapshot, git_ref_selection,
    resolve_diff_selection, uncommitted_selection,
};
pub use snapshot::{Snapshot, SnapshotFile, count_loc_if_countable, hash_file, snapshot_project};
