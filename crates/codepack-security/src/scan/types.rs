//! The vocabulary a scan answers in.
//!
//! Split out of `mod.rs` on 2026-09-05. These are the crate's published types — a
//! `Finding` is what reaches `06_security_scan.json` and SARIF — so they sit apart from
//! the detectors that produce them and the pass that collects them.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    SensitiveFile,
    PotentialSecret,
    RiskyCode,
    /// A file past `codepack_core::classify::ABSOLUTE_MAX_TEXT_FILE_READ_BYTES` was
    /// scanned only up to that ceiling, not in full (audit 2026-09-07, P-2/Q-3). Legacy
    /// never produced this kind — it has no golden-reference equivalent — because legacy
    /// had no ceiling at all and simply read the whole file.
    PartialScan,
}

impl FindingKind {
    /// The stable machine-readable name used wherever a finding's kind is rendered as a
    /// string rather than serialized directly — `codepack-cli`'s `scan`/`verify` reports
    /// and `codepack-engine`'s cancelled-run summary. One definition rather than three
    /// (audit 2026-09-07, Q-lint): all three had drifted into identical copies of this
    /// exact match, which is also the way a new variant like [`FindingKind::PartialScan`]
    /// could have reached two of them and not the third.
    pub fn label(self) -> &'static str {
        match self {
            FindingKind::SensitiveFile => "sensitive_file",
            FindingKind::PotentialSecret => "potential_secret",
            FindingKind::RiskyCode => "risky_code",
            FindingKind::PartialScan => "partial_scan",
        }
    }
}

/// Mirrors legacy's flat finding dict exactly: `type`, `severity`, `confidence`,
/// `file`, `line`, `rule`, `message`. `message` is **always** either a fixed,
/// hard-coded description or the output of [`keyword::redacted_line`] — invariant I3:
/// the raw matched substring never reaches this field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    #[serde(rename = "type")]
    pub kind: FindingKind,
    pub severity: String,
    pub confidence: String,
    pub file: String,
    pub line: Option<usize>,
    pub rule: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScanSummary {
    pub sensitive_files: usize,
    pub potential_secrets: usize,
    pub risky_code: usize,
    /// Files scanned only up to `ABSOLUTE_MAX_TEXT_FILE_READ_BYTES`, not in full (audit
    /// 2026-09-07, P-2/Q-3). Additive field: a consumer built against the pre-2026-09-07
    /// shape simply never sees it, the same way it would silently ignore a hand-added
    /// key today — nothing existing changes name, type, or meaning.
    pub partial_scans: usize,
    pub total_findings: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub findings: Vec<Finding>,
}

/// Rebuilds a result from a subset of its findings, recounting the summary by kind so
/// it describes what is actually there.
///
/// Used after the allowlist removes accepted findings: a summary left saying "3 secrets"
/// over a list of one is the kind of quiet inconsistency that makes a report untrusted.
pub fn result_from_findings(findings: Vec<Finding>) -> ScanResult {
    let mut summary = ScanSummary::default();
    for finding in &findings {
        match finding.kind {
            FindingKind::SensitiveFile => summary.sensitive_files += 1,
            FindingKind::PotentialSecret => summary.potential_secrets += 1,
            FindingKind::RiskyCode => summary.risky_code += 1,
            FindingKind::PartialScan => summary.partial_scans += 1,
        }
    }
    summary.total_findings = findings.len();
    ScanResult { summary, findings }
}

/// What a caller may vary about a scan. Everything defaults to the behaviour every
/// release so far produced, so [`scan_project`] and
/// [`scan_project_with_options`] with a default value are the same run.
#[derive(Clone, Copy, Default)]
pub struct ScanOptions<'a> {
    /// Redactor used to build every finding message.
    ///
    /// `None` is the plain, indistinguishable placeholder. A labelled redactor makes
    /// `<REDACTED:s1>` reach `06_security_scan.json`, SARIF and the text report, so a
    /// reader can tell whether two findings are one credential or two — the gap Q34
    /// named, where labels reached the text dump and the git reports but stopped at the
    /// scanner's own artifacts.
    ///
    /// The artifact *shape* is unchanged either way: no field is added, removed or
    /// retyped, only the vocabulary inside `message`, and only when the caller opts in.
    /// That is deliberate — `06_security_scan.json`'s `schema_version` is matched
    /// against the archived legacy implementation by the golden references, so a bump
    /// here could never be satisfied by regenerating them.
    pub redactor: Option<&'a crate::Redactor>,
    /// Let a failed vendor checksum weaken a provider finding.
    ///
    /// Off unless asked for, because the recipe is reverse-engineered rather than
    /// published — see [`crate::patterns::checksum`] for why an unverifiable algorithm
    /// may not be allowed to demote a real token.
    pub strict_token_checksums: bool,
    /// Where an already-scanned file's content-derived findings may be reused from.
    ///
    /// `None` scans everything, which is what every release so far did. See
    /// [`crate::cache`] for what is cacheable, what the key has to cover, and why the
    /// store itself lives outside this crate.
    pub cache: Option<&'a dyn crate::cache::FileScanCache>,
}

/// Written out rather than derived: a cache is a trait object, and requiring every
/// implementation to be `Debug` for the sake of this line would be the tail wagging the
/// dog. Whether one is present is the only part worth printing anyway.
impl std::fmt::Debug for ScanOptions<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScanOptions")
            .field("redactor", &self.redactor)
            .field("strict_token_checksums", &self.strict_token_checksums)
            .field("cache", &self.cache.map(|_| "present"))
            .finish()
    }
}
