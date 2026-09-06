//! Reports ignored advisories that no longer match anything in the dependency graph, or
//! whose justification is overdue for a second look.
//!
//! `deny.toml` carries a list of advisories accepted by owner decision, each because its
//! upstream has no safe upgrade. Nothing made that list expire: it would go on suppressing
//! an advisory long after the dependency that caused it was updated or dropped, and the
//! first sign would be nobody noticing a real one (audit No. 33).
//!
//! This does not fail the build. An entry that has stopped matching, or whose revisit date
//! has passed, is housekeeping, not a defect, and turning a dependency update into a red
//! gate would teach people to delete the check rather than the entry. It prints what needs
//! attention, so the list prunes and refreshes itself the next time somebody looks.
//!
//! It reads `cargo deny`'s own JSON diagnostics rather than guessing whether an entry still
//! matches: an entry is stale exactly when `cargo deny` says it matched nothing, and asking
//! the tool that owns the question is the only answer that stays right as the graph moves.
//!
//! ## The revisit date (audit 2026-09-07, S-3)
//!
//! A justification can be factually true the day it is written and false a season later —
//! this file's own GTK3 entries said "the Windows-only build does not even compile this",
//! which stopped being true the moment Linux packaging shipped, and nothing noticed for
//! several commits. `every entry's reason` is expected to end with `Revisit: YYYY-MM-DD`;
//! an entry past that date, or missing one entirely, is reported the same way an unmatched
//! entry is — a nudge, not a build failure, because the fix is rereading a sentence, not
//! rewriting a dependency graph.

use std::path::Path;
use std::process::Command;

use codepack_core::time::UtcDateTime;

/// One `deny.toml` ignore entry: its advisory id and, when present, its free-text reason.
///
/// A bare-string entry (`"RUSTSEC-..."`, no reason) is valid cargo-deny and yields
/// `reason: None` — it simply cannot carry a revisit date, and is reported as missing
/// one below, the same as a table entry whose reason forgot it.
struct IgnoredAdvisory {
    id: String,
    reason: Option<String>,
}

/// Prints any advisory in `deny.toml`'s ignore list that matched nothing, and any whose
/// justification has no revisit date or whose date has passed.
///
/// Never fails the gate — see the module docs. A `cargo deny` that cannot run at all is
/// reported and skipped for the match check, because the `deny` step itself has already
/// had its say about that by the time this runs; the revisit-date check needs no
/// `cargo deny` invocation at all and always runs.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let declared = declared_ignores(root)?;
    if declared.is_empty() {
        println!("ignored advisories: none declared.");
        return Ok(());
    }

    report_stale_matches(root, &declared);
    report_overdue_revisits(&declared);
    Ok(())
}

fn report_stale_matches(root: &Path, declared: &[IgnoredAdvisory]) {
    let output = Command::new("cargo")
        // `--log-level info` is what makes the per-advisory diagnostics appear at all.
        // Without it `--format json` emits only a summary object, and every entry looks
        // unmatched — which is exactly the wrong answer, since it invites deleting
        // suppressions that are still doing their job.
        .args([
            "deny",
            "--format",
            "json",
            "--log-level",
            "info",
            "check",
            "advisories",
        ])
        .current_dir(root)
        .output();

    let Ok(output) = output else {
        println!(
            "ignored advisories: cargo-deny is not available, so {} entr(ies) were not \
             re-checked against the graph.",
            declared.len()
        );
        return;
    };

    // Diagnostics go to stderr, one JSON object per line.
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    let matched: Vec<&str> = declared
        .iter()
        .map(|entry| entry.id.as_str())
        .filter(|id| diagnostics.contains(id))
        .collect();
    let unmatched: Vec<&str> = declared
        .iter()
        .map(|entry| entry.id.as_str())
        .filter(|id| !diagnostics.contains(id))
        .collect();

    if unmatched.is_empty() {
        println!(
            "ignored advisories: all {} still apply to the current graph.",
            declared.len()
        );
        return;
    }

    println!(
        "ignored advisories: {} of {} no longer match anything in the graph and can be \
         removed from deny.toml:",
        unmatched.len(),
        declared.len()
    );
    for id in &unmatched {
        println!("  {id}");
    }
    println!("  ({} still apply.)", matched.len());
}

/// Flags an entry whose reason has no `Revisit: YYYY-MM-DD`, or whose date is today or
/// earlier — the mechanism the module doc describes: a justification that was true when
/// written is not re-checked by anything unless something asks it to expire.
fn report_overdue_revisits(declared: &[IgnoredAdvisory]) {
    let today = UtcDateTime::now();
    let mut missing = Vec::new();
    let mut overdue = Vec::new();

    for entry in declared {
        match entry.reason.as_deref().and_then(revisit_date) {
            Some(date) if date <= (today.year, today.month, today.day) => {
                overdue.push((entry.id.as_str(), date));
            }
            Some(_) => {}
            None => missing.push(entry.id.as_str()),
        }
    }

    if missing.is_empty() && overdue.is_empty() {
        return;
    }
    println!("ignored advisories: revisit dates need attention:");
    for id in &missing {
        println!("  {id}: no `Revisit: YYYY-MM-DD` in its reason");
    }
    for (id, (year, month, day)) in &overdue {
        println!("  {id}: revisit date {year:04}-{month:02}-{day:02} has passed");
    }
}

/// Pulls `(year, month, day)` out of `Revisit: YYYY-MM-DD` inside a free-text reason.
///
/// Deliberately not a general date parser: this reads exactly the one fixed-width shape
/// this file's own entries are written in, and a reason that spells the date any other
/// way is treated as not carrying one at all — which `report_overdue_revisits` then
/// reports, rather than silently accepting a shape the reader did not intend.
fn revisit_date(reason: &str) -> Option<(i64, u32, u32)> {
    let after = reason.split("Revisit:").nth(1)?;
    let candidate = after.trim().get(0..10)?;
    let mut parts = candidate.splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// Every advisory id (and reason, when present) in `deny.toml`'s `[advisories] ignore`
/// list.
///
/// Parsed as TOML rather than grepped: the entries are `{ id = "...", reason = "..." }`
/// tables since audit No. 33, and a regex over that shape is the kind of thing that keeps
/// working until the day somebody reformats the file.
fn declared_ignores(root: &Path) -> Result<Vec<IgnoredAdvisory>, String> {
    let path = root.join("deny.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let parsed: toml::Table = text
        .parse()
        .map_err(|error| format!("{} is not valid TOML: {error}", path.display()))?;

    let Some(ignore) = parsed
        .get("advisories")
        .and_then(|advisories| advisories.get("ignore"))
        .and_then(|ignore| ignore.as_array())
    else {
        return Ok(Vec::new());
    };

    Ok(ignore
        .iter()
        .filter_map(|entry| match entry {
            // Both spellings are valid cargo-deny: a bare id, or a table with a reason.
            toml::Value::String(id) => Some(IgnoredAdvisory {
                id: id.clone(),
                reason: None,
            }),
            toml::Value::Table(table) => {
                table
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .map(|id| IgnoredAdvisory {
                        id: id.to_string(),
                        reason: table
                            .get("reason")
                            .and_then(toml::Value::as_str)
                            .map(str::to_string),
                    })
            }
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root is two levels above this crate")
    }

    /// The real `deny.toml` parses, and every entry yields an id. A `reason` that swallowed
    /// the id would make this check silently examine nothing.
    #[test]
    fn every_entry_in_the_real_deny_toml_yields_an_id() {
        let declared = declared_ignores(workspace_root()).expect("deny.toml parses");
        assert!(
            !declared.is_empty(),
            "the project does ignore some advisories"
        );
        for entry in &declared {
            assert!(
                entry.id.starts_with("RUSTSEC-"),
                "{} is not an advisory id",
                entry.id
            );
        }
    }

    /// The real `deny.toml`'s own revisit dates are all in the future — this is the test
    /// that would catch the exact defect S-3 found: a justification nobody has looked at
    /// again since the day it stopped being true.
    #[test]
    fn every_entry_in_the_real_deny_toml_has_a_revisit_date_that_has_not_passed() {
        let declared = declared_ignores(workspace_root()).expect("deny.toml parses");
        let today = UtcDateTime::now();
        let today = (today.year, today.month, today.day);
        for entry in &declared {
            let date = entry
                .reason
                .as_deref()
                .and_then(revisit_date)
                .unwrap_or_else(|| panic!("{}: reason carries no `Revisit: YYYY-MM-DD`", entry.id));
            assert!(
                date > today,
                "{}: revisit date {date:?} has already passed",
                entry.id
            );
        }
    }

    /// Both spellings cargo-deny accepts are understood, so switching between them cannot
    /// quietly empty the list this step examines.
    #[test]
    fn both_a_bare_id_and_a_table_with_a_reason_are_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("deny.toml"),
            "[advisories]\nignore = [\n  \"RUSTSEC-0000-0001\",\n  \
             { id = \"RUSTSEC-0000-0002\", reason = \"because\" },\n]\n",
        )
        .unwrap();

        let declared = declared_ignores(dir.path()).unwrap();
        let ids: Vec<&str> = declared.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["RUSTSEC-0000-0001", "RUSTSEC-0000-0002"]);
        assert_eq!(declared[0].reason, None);
        assert_eq!(declared[1].reason.as_deref(), Some("because"));
    }

    /// A file with no ignore list is not an error: a project that suppresses nothing is
    /// the state this check would like everyone to reach.
    #[test]
    fn a_deny_toml_with_no_ignores_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("deny.toml"),
            "[bans]\nwildcards = \"deny\"\n",
        )
        .unwrap();
        assert!(declared_ignores(dir.path()).unwrap().is_empty());
    }

    /// The exact shape this file's entries are written in.
    #[test]
    fn revisit_date_reads_the_documented_shape() {
        assert_eq!(
            revisit_date("gdk: GTK3 binding, unmaintained. Revisit: 2026-12-01"),
            Some((2026, 12, 1))
        );
    }

    /// No `Revisit:` marker at all is "no date", not a parse error to propagate.
    #[test]
    fn revisit_date_is_none_when_the_marker_is_absent() {
        assert_eq!(
            revisit_date("archived upstream, reached only through tauri-utils"),
            None
        );
    }

    /// A month or day outside its valid range is rejected rather than silently accepted —
    /// a typo here should be visible, not treated as a real, far-future date.
    #[test]
    fn revisit_date_rejects_an_impossible_calendar_date() {
        assert_eq!(revisit_date("Revisit: 2026-13-01"), None);
        assert_eq!(revisit_date("Revisit: 2026-00-01"), None);
    }
}
