//! `02_file_statistics.txt`, ported from legacy
//! `reports/insights/file_statistics.py::write_file_statistics_report`.

use std::path::Path;

use codepack_tokens::format_bytes;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::paths::path_depth;
use crate::plugin::ReportJob;
use crate::profile;

pub const JOB: ReportJob = ReportJob {
    filename: "02_file_statistics.txt",
    profiles: profile::FILE_STATISTICS_TXT,
    description: "File counts by extension, deepest paths, empty / suspicious files.",
    run: write_file_statistics_report,
};

const LIST_LIMIT: usize = 100;
/// Legacy: "conservative; Windows `MAX_PATH` is 260, but tools like `robocopy` fail earlier."
const LONG_PATH_THRESHOLD: usize = 240;

fn is_non_ascii(text: &str) -> bool {
    text.chars().any(|ch| ch as u32 > 127)
}

fn write_list(out: &mut String, title: &str, items: &[String]) {
    out.push_str(&format!("\n--- {title} ---\n"));
    if items.is_empty() {
        out.push_str("None detected.\n");
        return;
    }
    for item in items.iter().take(LIST_LIMIT) {
        out.push_str(item);
        out.push('\n');
    }
    if items.len() > LIST_LIMIT {
        out.push_str(&format!("... and {} more\n", items.len() - LIST_LIMIT));
    }
}

fn write_file_statistics_report(
    ctx: &ReportContext<'_>,
    output_file: &Path,
) -> Result<(), ReportError> {
    let inventory = ctx.inventory;

    let mut sorted_by_size: Vec<&crate::context::InventoryFile> = inventory.files.iter().collect();
    sorted_by_size.sort_by_key(|file| std::cmp::Reverse(file.size));

    let mut sorted_by_depth: Vec<&crate::context::InventoryFile> = inventory.files.iter().collect();
    sorted_by_depth.sort_by_key(|file| std::cmp::Reverse(path_depth(&file.relative_path)));

    let empty_files: Vec<String> = inventory
        .files
        .iter()
        .filter(|file| file.size == 0)
        .map(|file| file.relative_path.clone())
        .collect();
    let spaced: Vec<String> = inventory
        .files
        .iter()
        .filter(|file| file.relative_path.contains(' '))
        .map(|file| file.relative_path.clone())
        .collect();
    let non_ascii: Vec<String> = inventory
        .files
        .iter()
        .filter(|file| is_non_ascii(&file.relative_path))
        .map(|file| file.relative_path.clone())
        .collect();
    let long_paths: Vec<(String, usize)> = inventory
        .files
        .iter()
        .filter(|file| file.relative_path.len() >= LONG_PATH_THRESHOLD)
        .map(|file| (file.relative_path.clone(), file.relative_path.len()))
        .collect();

    let mut out = String::new();
    out.push_str("=== File Statistics ===\n");
    out.push_str(&format!("Generated: {}\n", ctx.plan.generated_at));
    out.push_str(&"=".repeat(100));
    out.push_str("\n\n");

    out.push_str("--- Files by extension ---\n");
    out.push_str(&format!(
        "{:<20} {:>10} {:>14}\n",
        "Extension", "Files", "Total size"
    ));
    out.push_str(&format!(
        "{} {} {}\n",
        "-".repeat(20),
        "-".repeat(10),
        "-".repeat(14)
    ));
    for stat in &inventory.by_extension {
        out.push_str(&format!(
            "{:<20} {:>10} {:>14}\n",
            stat.extension,
            stat.count,
            format_bytes(stat.total_size)
        ));
    }

    out.push_str("\n--- Top 25 largest files ---\n");
    for file in sorted_by_size.iter().take(25) {
        out.push_str(&format!(
            "{:>12}  {}\n",
            format_bytes(file.size),
            file.relative_path
        ));
    }

    out.push_str("\n--- Top 25 deepest files ---\n");
    for file in sorted_by_depth.iter().take(25) {
        out.push_str(&format!(
            "depth={:>2}  {}\n",
            path_depth(&file.relative_path),
            file.relative_path
        ));
    }

    write_list(&mut out, "Empty files", &empty_files);
    write_list(&mut out, "Files with spaces in names", &spaced);
    write_list(&mut out, "Files with non-ASCII paths", &non_ascii);

    out.push_str("\n--- Potential Windows long-path risks >= 240 characters ---\n");
    if long_paths.is_empty() {
        out.push_str("None detected.\n");
    } else {
        for (path, length) in long_paths.iter().take(LIST_LIMIT) {
            out.push_str(&format!("{length:>4} chars  {path}\n"));
        }
        if long_paths.len() > LIST_LIMIT {
            out.push_str(&format!("... and {} more\n", long_paths.len() - LIST_LIMIT));
        }
    }

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
    fn lists_empty_spaced_and_extension_breakdown() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("empty.txt"), "").unwrap();
            std::fs::write(root.join("has space.txt"), "x").unwrap();
            std::fs::create_dir_all(root.join("src")).unwrap();
            std::fs::write(root.join("src/main.py"), "x = 1\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_file_statistics_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.starts_with("=== File Statistics ==="));
        assert!(content.contains("empty.txt"));
        assert!(content.contains("has space.txt"));
        assert!(content.contains("py"));
    }

    #[test]
    fn reports_none_detected_when_no_suspicious_files_exist() {
        let fixture = Fixture::new(|root| {
            std::fs::write(root.join("main.py"), "x = 1\n").unwrap();
        });
        let ctx = fixture.context("full");
        let out_dir = tempfile::tempdir().unwrap();
        let output_file = out_dir.path().join(JOB.filename);

        write_file_statistics_report(&ctx, &output_file).unwrap();

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("--- Empty files ---\nNone detected."));
    }
}
