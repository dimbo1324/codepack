//! Report writers for the three legacy-parity artifact formats: `.txt` (human-
//! readable), `.json` (machine-readable contract), `.sarif` (2.1.0, for code-review
//! and security tooling). Each format's exact structure is ported from legacy
//! `reports/insights/security.py::write_security_scan_report`.

mod json;
mod sarif;
mod txt;

pub use json::write_json_report;
pub use sarif::write_sarif_report;
pub use txt::write_txt_report;
