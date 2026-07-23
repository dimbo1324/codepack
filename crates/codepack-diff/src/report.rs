//! `write_diff_report()`: Markdown matching legacy `29_export_comparison_report.md`'s
//! structure, ported from `diff_service.py::write_diff_report` verbatim — including
//! its Russian section headings. This file is a user-facing artifact, not
//! agent-facing infrastructure; `.ai/project/10-project-map.md`'s English/Russian
//! language policy governs documents and code, not report *content*, and legacy's own
//! report text is Russian throughout.

use std::fs;
use std::path::Path;

use crate::error::{DiffError, Result};
use crate::selection::{DiffSelection, FileStatus};

const TRUNCATE_AT: usize = 500;

pub fn write_diff_report(output_file: &Path, selection: &DiffSelection) -> Result<()> {
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent).map_err(|source| DiffError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let rendered = render_markdown(selection);
    fs::write(output_file, rendered).map_err(|source| DiffError::Write {
        path: output_file.to_path_buf(),
        source,
    })
}

fn render_markdown(selection: &DiffSelection) -> String {
    let changed = selection
        .files
        .iter()
        .filter(|item| item.status != FileStatus::Deleted)
        .count();
    let deleted = selection.deleted().count();

    let mut out = String::new();
    out.push_str("# Дифференциальный экспорт\n\n");
    out.push_str(&format!("- Режим: {}\n", selection.mode));
    out.push_str(&format!("- База: {}\n", selection.base));
    out.push_str(&format!(
        "- Ограничение набора файлов: {}\n",
        if selection.is_limited() {
            "да"
        } else {
            "нет"
        }
    ));
    out.push_str(&format!(
        "- Изменённых файлов в архиве: {}\n",
        format_count(changed as u64)
    ));
    out.push_str(&format!(
        "- Удалённых файлов в manifest: {}\n\n",
        format_count(deleted as u64)
    ));

    if let Some(warning) = &selection.warning {
        out.push_str(&format!("Предупреждение: {warning}\n\n"));
    }

    for (status, title) in [
        (FileStatus::Added, "Добавленные"),
        (FileStatus::Modified, "Изменённые"),
        (FileStatus::Renamed, "Переименованные"),
        (FileStatus::Deleted, "Удалённые"),
    ] {
        let items: Vec<&_> = selection
            .files
            .iter()
            .filter(|item| item.status == status)
            .collect();
        out.push_str(&format!("## {title}\n\n"));
        if items.is_empty() {
            out.push_str("- нет\n\n");
            continue;
        }
        for item in items.iter().take(TRUNCATE_AT) {
            let old = item
                .old_path
                .as_ref()
                .map(|path| format!(" (ранее: `{path}`)"))
                .unwrap_or_default();
            out.push_str(&format!("- `{}`{old}\n", item.relative_path));
        }
        if items.len() > TRUNCATE_AT {
            out.push_str(&format!(
                "- ... и ещё {}\n",
                format_count((items.len() - TRUNCATE_AT) as u64)
            ));
        }
        out.push('\n');
    }
    out
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::DiffFile;

    fn selection_with(files: Vec<DiffFile>, warning: Option<&str>) -> DiffSelection {
        DiffSelection {
            mode: "last_export".to_string(),
            base: "последний экспорт".to_string(),
            paths: Some(std::collections::HashSet::new()),
            files,
            warning: warning.map(str::to_string),
        }
    }

    #[test]
    fn renders_headings_and_counts() {
        let files = vec![
            DiffFile {
                relative_path: "a.py".to_string(),
                status: FileStatus::Added,
                old_path: None,
            },
            DiffFile {
                relative_path: "b.py".to_string(),
                status: FileStatus::Modified,
                old_path: None,
            },
            DiffFile {
                relative_path: "new.py".to_string(),
                status: FileStatus::Renamed,
                old_path: Some("old.py".to_string()),
            },
            DiffFile {
                relative_path: "gone.py".to_string(),
                status: FileStatus::Deleted,
                old_path: None,
            },
        ];
        let markdown = render_markdown(&selection_with(files, None));

        assert!(markdown.starts_with("# Дифференциальный экспорт"));
        assert!(markdown.contains("## Добавленные"));
        assert!(markdown.contains("## Изменённые"));
        assert!(markdown.contains("## Переименованные"));
        assert!(markdown.contains("## Удалённые"));
        assert!(markdown.contains("- Изменённых файлов в архиве: 3\n"));
        assert!(markdown.contains("- Удалённых файлов в manifest: 1\n"));
        assert!(markdown.contains("`new.py` (ранее: `old.py`)"));
    }

    #[test]
    fn renders_warning_when_present() {
        let markdown = render_markdown(&selection_with(Vec::new(), Some("test warning")));
        assert!(markdown.contains("Предупреждение: test warning\n\n"));
    }

    #[test]
    fn empty_category_renders_none_marker() {
        let markdown = render_markdown(&selection_with(Vec::new(), None));
        assert!(markdown.contains("- нет\n"));
    }

    #[test]
    fn truncates_after_500_items_with_a_tail_count() {
        let files: Vec<DiffFile> = (0..520)
            .map(|i| DiffFile {
                relative_path: format!("file{i:04}.py"),
                status: FileStatus::Added,
                old_path: None,
            })
            .collect();
        let markdown = render_markdown(&selection_with(files, None));

        assert!(markdown.contains("- ... и ещё 20\n"));
    }

    #[test]
    fn write_diff_report_creates_parent_directories_and_writes_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("nested/report.md");
        let selection = selection_with(Vec::new(), None);

        write_diff_report(&output, &selection).unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.starts_with("# Дифференциальный экспорт"));
    }
}
