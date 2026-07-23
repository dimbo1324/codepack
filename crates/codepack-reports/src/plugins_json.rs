//! [`write_report_plugins_json`]: `REPORT_PLUGINS.json`, ported from legacy
//! `plugins.py::plugin_catalog` — "the beginning of a public plugin API" (BLUEPRINT
//! §A.7/§B.10). This function itself simply returns [`crate::error::ReportError`] on
//! failure like every other writer in this crate; legacy's own call site wraps the
//! write in its own `except Exception: log(...)` **separate** from the per-job
//! fault-tolerance loop, so the same best-effort behavior here is the *caller's*
//! responsibility (log and continue), not something this function should swallow
//! silently itself.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::ReportError;
use crate::plugin::ReportJob;

#[derive(Debug, Clone, Serialize)]
struct PluginEntry {
    filename: &'static str,
    profiles: Vec<&'static str>,
    description: &'static str,
}

pub fn write_report_plugins_json(jobs: &[ReportJob], out_dir: &Path) -> Result<(), ReportError> {
    let entries: Vec<PluginEntry> = jobs
        .iter()
        .map(|job| {
            let mut profiles = job.profiles.to_vec();
            profiles.sort_unstable();
            PluginEntry {
                filename: job.filename,
                profiles,
                description: job.description,
            }
        })
        .collect();

    let rendered = serde_json::to_string_pretty(&entries)
        .map_err(|source| ReportError::Serialize { source })?;
    let output_file = out_dir.join("REPORT_PLUGINS.json");
    fs::write(&output_file, rendered).map_err(|source| ReportError::Write {
        path: output_file,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_job(filename: &'static str, profiles: &'static [&'static str]) -> ReportJob {
        ReportJob {
            filename,
            profiles,
            description: "test description",
            run: |_ctx, output| {
                fs::write(output, "x").map_err(|source| ReportError::Write {
                    path: output.to_path_buf(),
                    source,
                })
            },
        }
    }

    #[test]
    fn writes_sorted_profile_lists_and_expected_shape() {
        let dir = tempfile::tempdir().unwrap();
        let jobs = vec![dummy_job("01_summary.txt", &["full", "quick", "ai_review"])];

        write_report_plugins_json(&jobs, dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join("REPORT_PLUGINS.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let entries = parsed.as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["filename"], "01_summary.txt");
        assert_eq!(
            entries[0]["profiles"],
            serde_json::json!(["ai_review", "full", "quick"])
        );
        assert_eq!(entries[0]["description"], "test description");
    }
}
