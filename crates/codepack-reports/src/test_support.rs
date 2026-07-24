//! Shared fixture helpers for colocated unit tests across `reports/*`. Only compiled
//! under `cfg(test)` — see the `#[cfg(test)]` on this module's declaration in `lib.rs`.

use std::path::Path;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_scanner::{ExportIgnoreRules, ExportPlan, ScanOptions, build_export_plan};

use crate::context::{Inventory, ReportContext};

pub(crate) fn build_plan(root: &Path) -> ExportPlan {
    build_export_plan(
        root,
        &ScanOptions::default(),
        &ExportIgnoreRules::default(),
        &codepack_scanner::no_safety_classification,
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Owns everything a [`ReportContext`] borrows from, so a test can build one fixture
/// and call `.context("full")` as many times as it needs.
pub(crate) struct Fixture {
    pub root: tempfile::TempDir,
    pub plan: ExportPlan,
    pub inventory: Inventory,
    pub config: Config,
    pub cancel: CancellationToken,
}

impl Fixture {
    pub(crate) fn new(write_files: impl FnOnce(&Path)) -> Self {
        let root = tempfile::tempdir().unwrap();
        write_files(root.path());
        let plan = build_plan(root.path());
        let inventory = Inventory::from_plan(&plan);
        Self {
            root,
            plan,
            inventory,
            config: Config::default(),
            cancel: CancellationToken::new(),
        }
    }

    pub(crate) fn context(&self, profile: &'static str) -> ReportContext<'_> {
        ReportContext {
            source_root: self.root.path().to_path_buf(),
            staging_root: self.root.path().to_path_buf(),
            inventory: &self.inventory,
            plan: &self.plan,
            scan: None,
            diff: None,
            config: &self.config,
            cancel: &self.cancel,
            profile,
        }
    }
}
