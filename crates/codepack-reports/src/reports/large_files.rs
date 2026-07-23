//! `25_large_files_report.md`, ported from legacy
//! `reports/insights/large_files.py::write_large_files_report`.

use std::collections::HashMap;
use std::path::Path;

use codepack_tokens::format_bytes;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "25_large_files_report.md",
    profiles: profile::LARGE_FILES_REPORT_MD,
    description: "Large file/folder inspector.",
    run: write_large_files_report,
};

/// Every ancestor folder of `relative_path` accumulates that file's size (legacy: `for
/// part in parts: current = current / part; folder_sizes[current] += size`), not just
/// the immediate parent.
fn folder_sizes(files: &[crate::context::InventoryFile]) -> HashMap<String, u64> {
    let mut sizes: HashMap<String, u64> = HashMap::new();
    for file in files {
        let segments: Vec<&str> = file.relative_path.split('\\').collect();
        if segments.len() < 2 {
            continue;
        }
        let mut current = String::new();
        for segment in &segments[..segments.len() - 1] {
            if current.is_empty() {
                current = (*segment).to_string();
            } else {
                current.push('\\');
                current.push_str(segment);
            }
            *sizes.entry(current.clone()).or_insert(0) += file.size;
        }
    }
    sizes
}

fn write_large_files_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let mut sorted_by_size: Vec<&crate::context::InventoryFile> =
        ctx.inventory.files.iter().collect();
    sorted_by_size.sort_by_key(|file| std::cmp::Reverse(file.size));

    let sizes = folder_sizes(&ctx.inventory.files);
    let mut folder_list: Vec<(&String, &u64)> = sizes.iter().collect();
    folder_list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let mut out = String::new();
    out.push_str("# Large File Inspector\n\n");
    out.push_str(&format!("Generated: {}\n\n", ctx.plan.generated_at));

    out.push_str("## Top 30 largest files\n\n");
    if sorted_by_size.is_empty() {
        out.push_str("No files detected.\n");
    }
    for file in sorted_by_size.iter().take(30) {
        out.push_str(&format!(
            "- `{}` — {}\n",
            file.relative_path,
            format_bytes(file.size)
        ));
    }

    out.push_str("\n## Top 20 largest folders\n\n");
    if folder_list.is_empty() {
        out.push_str("No folders detected.\n");
    }
    for (folder, size) in folder_list.iter().take(20) {
        out.push_str(&format!("- `{folder}` — {}\n", format_bytes(**size)));
    }

    out.push_str("\n## Archive-size guidance\n\n");
    out.push_str(
        "- Safe Export mode removes common local databases, archives and credentials before packaging.\n",
    );
    out.push_str(
        "- If the single ZIP exceeds the configured limit, archives are split by logical groups.\n",
    );
    out.push_str(
        "- Very large individual files should normally be excluded from AI/code-review exports.\n",
    );

    std::fs::write(output_file, out).map_err(|source| ReportError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    #[test]
    fn lists_largest_files_and_folders() {
        let fixture = Fixture::new(|root| {
            std::fs::create_dir_all(root.join("src").join("utils")).unwrap();
            std::fs::write(root.join("src").join("main.py"), vec![b'x'; 200]).unwrap();
            std::fs::write(
                root.join("src").join("utils").join("helper.py"),
                vec![b'y'; 100],
            )
            .unwrap();
            std::fs::write(root.join("small.txt"), b"hi").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_large_files_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("# Large File Inspector"));
        assert!(content.contains("## Top 30 largest files"));
        assert!(content.contains("src\\main.py"));
        assert!(content.contains("## Top 20 largest folders"));
        assert!(content.contains("- `src` —"));
        assert!(content.contains("- `src\\utils` —"));
    }

    #[test]
    fn folder_sizes_accumulate_into_every_ancestor() {
        let files = vec![crate::context::InventoryFile {
            relative_path: "a\\b\\c.txt".to_string(),
            size: 10,
            extension: "txt".to_string(),
            language: None,
        }];
        let sizes = folder_sizes(&files);
        assert_eq!(sizes.get("a"), Some(&10));
        assert_eq!(sizes.get("a\\b"), Some(&10));
        assert_eq!(sizes.get("a\\b\\c.txt"), None);
    }

    #[test]
    fn reports_none_detected_when_project_is_empty() {
        let fixture = Fixture::new(|_root| {});
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_large_files_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("No files detected."));
        assert!(content.contains("No folders detected."));
    }
}
