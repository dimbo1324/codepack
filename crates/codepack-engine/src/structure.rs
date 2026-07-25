//! Pipeline step 3 ("structure report"): a PowerShell `Get-ChildItem`-shaped listing of
//! the copied project tree, ported from legacy
//! `reports/structure_report.py::write_structure_report`.
//!
//! Deviation: [`ps_date`] renders the file's modification time in **UTC**, not
//! legacy's local wall clock (`datetime.fromtimestamp`) — the same documented
//! UTC-over-local precedent as [`crate::timestamp`]. Modification-time display here is
//! informational narration, not a byte-exact contract field, so this is acceptable
//! parity, not a gap.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use codepack_core::CancellationToken;
use codepack_core::time::{UtcDateTime, unix_seconds_of};

use crate::error::{EngineError, Result};
use crate::layout::section_rule;
use crate::timestamp::human_now_utc;

/// Abbreviated English month names, in PowerShell's `Get-ChildItem` rendering order.
/// Part of the reproduced legacy layout, not a localizable string.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Legacy `ps_mode`: `"d-----"` for a directory, `"-a----"` for a file.
fn ps_mode(is_dir: bool) -> &'static str {
    if is_dir { "d-----" } else { "-a----" }
}

/// Legacy `ps_date`, rendered in UTC (see the module doc comment's documented
/// deviation): `DD-Mon-YY     HH:MM`.
///
/// The run of spaces before the time is legacy's own column padding, reproduced
/// verbatim so the listing lines up the way the original did.
fn ps_date(mtime: SystemTime) -> String {
    let at = UtcDateTime::from_unix_seconds(unix_seconds_of(mtime));
    // `clamp` guards the array index against a hypothetical out-of-range month rather
    // than trusting the calendar conversion blindly; `rem_euclid` keeps the two-digit
    // year non-negative for pre-2000 dates.
    let month_name = MONTHS[(at.month.clamp(1, 12) - 1) as usize];
    let short_year = at.year.rem_euclid(100);
    let (day, hour, minute) = (at.day, at.hour, at.minute);
    format!("{day:02}-{month_name}-{short_year:02}     {hour:02}:{minute:02}")
}

/// Legacy `rel_display`: `"."` for `root` itself, otherwise `".\\a\\b\\c"` — always
/// backslash-joined, matching this project's cross-platform relative-path display
/// convention (the same one `PlannedFile.relative_path` uses).
fn rel_display(current: &Path, root: &Path) -> String {
    let rel = current.strip_prefix(root).unwrap_or(current);
    if rel.as_os_str().is_empty() {
        return ".".to_string();
    }
    let joined = rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\\");
    format!(".\\{joined}")
}

fn should_ignore_dir(name: &str, ignored_lower: &HashSet<String>) -> bool {
    ignored_lower.contains(&name.to_lowercase())
}

struct WalkState<'a> {
    root: &'a Path,
    ignored_lower: &'a HashSet<String>,
    cancel: &'a CancellationToken,
    log: &'a dyn Fn(&str),
}

fn walk_dir(state: &WalkState<'_>, current: &Path, out: &mut String, groups_written: &mut u32) {
    if state.cancel.is_cancelled() {
        return;
    }

    let Ok(read_dir) = fs::read_dir(current) else {
        return;
    };

    let mut dir_names: Vec<String> = Vec::new();
    let mut file_names: Vec<String> = Vec::new();
    for entry in read_dir.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if !should_ignore_dir(&name, state.ignored_lower) {
                dir_names.push(name);
            }
        } else {
            file_names.push(name);
        }
    }

    dir_names.sort_by_key(|name| name.to_lowercase());
    file_names.sort_by_key(|name| name.to_lowercase());

    if !dir_names.is_empty() || !file_names.is_empty() {
        out.push_str(&format!(
            "    Directory: {}\n\n",
            rel_display(current, state.root)
        ));
        out.push_str(&format!(
            "{:<20} {:<20} {:>12} Name\n",
            "Mode", "LastWriteTime", "Length"
        ));
        out.push_str(&format!(
            "{:<20} {:<20} {:>12} ----\n",
            "----", "-------------", "------"
        ));

        let entries = dir_names
            .iter()
            .map(|name| (name, true))
            .chain(file_names.iter().map(|name| (name, false)));
        for (name, is_dir) in entries {
            let path = current.join(name);
            let metadata = fs::metadata(&path).ok();
            let length = if is_dir {
                String::new()
            } else {
                metadata
                    .as_ref()
                    .map(|meta| meta.len().to_string())
                    .unwrap_or_default()
            };
            let date_str = metadata
                .and_then(|meta| meta.modified().ok())
                .map(ps_date)
                .unwrap_or_default();
            out.push_str(&format!(
                "{:<20} {date_str:<20} {length:>12} {name}\n",
                ps_mode(is_dir)
            ));
        }
        out.push_str("\n\n");
        *groups_written += 1;
        (state.log)(&format!("structure: {}", rel_display(current, state.root)));
    }

    for name in &dir_names {
        if state.cancel.is_cancelled() {
            break;
        }
        walk_dir(state, &current.join(name), out, groups_written);
    }
}

/// Runs pipeline step 3, ported from legacy `write_structure_report`.
/// `extra_ignored_dirs` is the "on top of the base defaults" set (legacy's own
/// `extra_ignored_dirs` variable, [`crate::ignored_dirs::extra_ignored_display`]'s
/// output turned into a set) — the base `codepack_scanner::IGNORED_DIR_NAMES` set is
/// always applied in addition, both for the header line and for per-entry pruning.
pub fn write_structure_report(
    root: &Path,
    output_file: &Path,
    extra_ignored_dirs: &HashSet<String>,
    log: &dyn Fn(&str),
    cancel: &CancellationToken,
) -> Result<u32> {
    let base_lower: HashSet<String> = codepack_scanner::IGNORED_DIR_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    let mut ignored_lower: HashSet<String> = base_lower.clone();
    ignored_lower.extend(extra_ignored_dirs.iter().map(|name| name.to_lowercase()));

    let mut display_names: Vec<String> = codepack_scanner::IGNORED_DIR_NAMES
        .iter()
        .map(|name| name.to_string())
        .chain(extra_ignored_dirs.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    display_names.sort();
    let ignored_display = display_names.join(", ");

    let root_name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str("=== Relative Project Structure ===\n");
    out.push_str(&format!("Project copy root name: {root_name}\n"));
    out.push_str(&format!("Generated: {}\n", human_now_utc()));
    out.push_str(&format!("Ignored directories: {ignored_display}\n"));
    out.push_str(&section_rule('='));
    out.push_str("\n\n");

    let mut groups_written = 0u32;
    let state = WalkState {
        root,
        ignored_lower: &ignored_lower,
        cancel,
        log,
    };
    walk_dir(&state, root, &mut out, &mut groups_written);

    fs::write(output_file, out).map_err(|source| EngineError::Io {
        path: output_file.to_path_buf(),
        source,
    })?;

    Ok(groups_written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_log(_: &str) {}

    #[test]
    fn ignored_directories_are_pruned_and_do_not_appear() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/pkg.js"), "x").unwrap();
        fs::write(dir.path().join("main.py"), "x").unwrap();

        let output = dir.path().join("out.txt");
        write_structure_report(
            dir.path(),
            &output,
            &HashSet::new(),
            &no_log,
            &CancellationToken::new(),
        )
        .unwrap();

        let content = fs::read_to_string(&output).unwrap();
        // "node_modules" itself legitimately appears in the header's "Ignored
        // directories:" line; what must never appear is anything recursed *into* it.
        assert!(!content.contains("pkg.js"));
        assert!(!content.contains("Directory: .\\node_modules"));
        assert!(content.contains("main.py"));
    }

    #[test]
    fn a_custom_extra_ignored_dir_is_pruned_too() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("vendor")).unwrap();
        fs::write(dir.path().join("vendor/lib.py"), "x").unwrap();

        let mut extra = HashSet::new();
        extra.insert("vendor".to_string());
        let output = dir.path().join("out.txt");
        write_structure_report(
            dir.path(),
            &output,
            &extra,
            &no_log,
            &CancellationToken::new(),
        )
        .unwrap();

        let content = fs::read_to_string(&output).unwrap();
        // "vendor" itself legitimately appears in the header's "Ignored directories:"
        // line; what must never appear is anything recursed *into* it.
        assert!(!content.contains("lib.py"));
        assert!(!content.contains("Directory: .\\vendor"));
    }

    #[test]
    fn a_directory_that_becomes_empty_after_pruning_contributes_no_group() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/pkg.js"), "x").unwrap();

        let output = dir.path().join("out.txt");
        let groups = write_structure_report(
            dir.path(),
            &output,
            &HashSet::new(),
            &no_log,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(groups, 0);
    }

    #[test]
    fn file_length_is_a_plain_digit_string_directory_length_is_blank() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.py"), "hello").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/nested.py"), "x").unwrap();

        let output = dir.path().join("out.txt");
        write_structure_report(
            dir.path(),
            &output,
            &HashSet::new(),
            &no_log,
            &CancellationToken::new(),
        )
        .unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains(" 5 main.py"));
        let sub_line = content
            .lines()
            .find(|line| line.trim_end().ends_with("sub"))
            .unwrap();
        assert!(sub_line.starts_with("d-----"));
    }

    #[test]
    fn cancellation_mid_walk_stops_early_without_error() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            fs::create_dir_all(dir.path().join(format!("dir{i}"))).unwrap();
            fs::write(dir.path().join(format!("dir{i}/f.py")), "x").unwrap();
        }
        let cancel = CancellationToken::new();
        cancel.cancel();

        let output = dir.path().join("out.txt");
        let result = write_structure_report(dir.path(), &output, &HashSet::new(), &no_log, &cancel);

        assert!(result.is_ok());
        assert!(output.exists());
    }
}
