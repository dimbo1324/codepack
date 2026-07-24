//! `14_dependency_graph.md` / `14_dependency_graph.mmd`, ported from legacy
//! `reports/insights/dependency_graph.py::write_dependency_graph_reports`. Consumes the
//! shared [`crate::graph::collect`] primitive — this report is that primitive's
//! original namesake and first consumer (also reused by `16_key_files_report.md` and
//! `23_refactoring_opportunities.md`; see `graph.rs`'s module doc for the full list).
//!
//! Writes two sibling files from one job, matching this crate's existing
//! `06_security_scan.*`/security-scan-adapter precedent for a job that produces more
//! than one artifact per run.

use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::graph::collect;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "14_dependency_graph.md",
    profiles: profile::DEPENDENCY_GRAPH_MD,
    description: "Internal import graph (Markdown table + Mermaid diagram siblings).",
    run: write_dependency_graph_reports,
};

const TOP_IMPORTED_LIMIT: usize = 30;
const EDGE_LIMIT: usize = 1000;
const MERMAID_EDGE_LIMIT: usize = 250;

fn node_id(relative_path: &str) -> String {
    relative_path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn write_dependency_graph_reports(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let graph = collect(ctx);
    let in_degree = graph.in_degree();
    let mut top_imported: Vec<(&String, &usize)> = in_degree.iter().collect();
    top_imported.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let mut out = String::new();
    out.push_str("# Dependency Graph\n\n");
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));
    out.push_str(
        "This report maps internal imports/references using static heuristics. It is intentionally dependency-free and may not resolve every alias.\n\n",
    );
    out.push_str(&format!("- Files in graph: **{}**\n", graph.edges.len()));
    out.push_str(&format!("- Internal edges: **{}**\n\n", graph.edge_count()));

    out.push_str("## Most imported internal files\n\n");
    if top_imported.is_empty() {
        out.push_str("No internal imports were resolved.\n");
    } else {
        for (path, count) in top_imported.iter().take(TOP_IMPORTED_LIMIT) {
            out.push_str(&format!("- `{path}` — imported by {count} file(s)\n"));
        }
    }

    out.push_str("\n## Internal import edges\n\n");
    let mut emitted = 0usize;
    'outer: for (source, targets) in &graph.edges {
        if targets.is_empty() {
            continue;
        }
        out.push_str(&format!("### `{source}`\n\n"));
        for target in targets {
            out.push_str(&format!("- `{target}`\n"));
            emitted += 1;
            if emitted >= EDGE_LIMIT {
                out.push_str("\n_Output truncated after 1,000 edges._\n");
                break 'outer;
            }
        }
        out.push('\n');
    }

    std::fs::write(output_file, out).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })?;

    let mut mermaid = String::new();
    mermaid.push_str("graph TD\n");
    let mut mermaid_emitted = 0usize;
    'mermaid: for (source, targets) in &graph.edges {
        for target in targets {
            mermaid.push_str(&format!(
                "  {}[\"{source}\"] --> {}[\"{target}\"]\n",
                node_id(source),
                node_id(target)
            ));
            mermaid_emitted += 1;
            if mermaid_emitted >= MERMAID_EDGE_LIMIT {
                mermaid.push_str("  %% Mermaid output truncated after 250 edges.\n");
                break 'mermaid;
            }
        }
    }
    if mermaid_emitted == 0 {
        let project_name = ctx
            .staging_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        mermaid.push_str(&format!("  root[\"{project_name}\"]\n"));
    }

    let mermaid_file = output_file.with_extension("mmd");
    std::fs::write(&mermaid_file, mermaid).map_err(|source| ReportError::Write {
        path: mermaid_file,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    #[test]
    fn writes_markdown_and_mermaid_siblings_with_resolved_edges() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("utils.py"), "").unwrap();
            std::fs::write(root.join("main.py"), "import utils\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_dependency_graph_reports(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("# Dependency Graph"));
        assert!(content.contains("## Most imported internal files"));
        assert!(content.contains("`utils.py` — imported by 1 file(s)"));
        assert!(content.contains("### `main.py`"));
        assert!(content.contains("- `utils.py`"));

        let mermaid_path = out_dir.path().join("14_dependency_graph.mmd");
        assert!(mermaid_path.exists());
        let mermaid = std::fs::read_to_string(&mermaid_path).unwrap();
        assert!(mermaid.starts_with("graph TD"));
        assert!(mermaid.contains("-->"));
    }

    #[test]
    fn reports_no_internal_imports_and_a_placeholder_mermaid_node_when_empty() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_dependency_graph_reports(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("No internal imports were resolved."));

        let mermaid =
            std::fs::read_to_string(out_dir.path().join("14_dependency_graph.mmd")).unwrap();
        assert!(mermaid.contains("root["));
    }
}
