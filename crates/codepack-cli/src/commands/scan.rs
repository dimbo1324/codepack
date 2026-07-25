//! `codepack scan` — the security scanner on its own.
//!
//! This is the command a CI pipeline runs on every push: it answers "does this project
//! contain secrets?" without producing a bundle. It is also the command that gives exit
//! code 3 its meaning.

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_security::{FindingKind, ScanResult, SecurityOptions, scan_project};
use serde::Serialize;

use crate::cli::ScanArgs;
use crate::commands::{self, ProjectContext};
use crate::error::Result;
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct ScanReport {
    pub project: String,
    /// Always `full`: a scan deliberately looks at everything the ignore rules include,
    /// not at what an export would keep. Reported so the output is self-explaining.
    pub safe_mode: String,
    pub scanned_files: usize,
    pub summary: Summary,
    pub findings: Vec<ReportedFinding>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub sensitive_files: usize,
    pub potential_secrets: usize,
    pub risky_code: usize,
    pub total_findings: usize,
    /// Findings at `critical` severity. Broken out because this is the number the exit
    /// code is derived from, and a consumer should not have to recount it.
    pub critical: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportedFinding {
    pub kind: &'static str,
    pub severity: String,
    pub confidence: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub rule: String,
    /// Already redacted by `codepack-security` before it ever reaches here — invariant
    /// I3 forbids a raw secret value in any output, and that includes this one.
    pub message: String,
}

pub(crate) fn run(args: &ScanArgs, format: Format) -> Result<Outcome> {
    let context = commands::prepare(&args.project)?;
    let report = build(&context)?;

    if format.is_json() {
        output::emit_json("scan", &report)?;
    } else {
        print_human(&report);
    }

    Ok(if report.summary.critical > 0 {
        Outcome::CriticalSecretsFound
    } else {
        Outcome::Success
    })
}

fn build(context: &ProjectContext) -> Result<ScanReport> {
    let cancel = CancellationToken::new();

    // Scanned with safe mode forced to `full`, meaning "exclude nothing on safety
    // grounds" — so the scan sees every file the ignore rules include, including the
    // ones an export would drop.
    //
    // This is the whole point of the command. Scanning the file set an export *would*
    // produce makes `scan` answer "is my bundle clean?", which is always yes: safe mode
    // removed the dangerous files before the scanner ever looked. A `.env` full of live
    // credentials sitting in the repository would be reported as "No findings", which
    // is worse than useless in the pre-commit and CI gate this command exists to be.
    // `preview` already answers the "what would I share?" question.
    //
    // Ignore rules still apply: `node_modules` and build output are not the user's
    // secrets to answer for, and scanning them would bury real findings in noise.
    let scan_config = Config {
        safe_export_mode: "full".to_string(),
        // A diff mode would narrow the scan to changed files; a secret that was
        // committed last week is still a secret today.
        diff_export_mode: "all".to_string(),
        // The budget drops low-value files, which has nothing to do with whether they
        // hold credentials.
        token_budget: 0,
        // The text-file size limit makes `SecurityOptions` skip a file whole rather
        // than truncate it, and the skip is invisible in the report. Two of the five
        // presets enable it (1 MB and 2 MB), so `scan --preset chatgpt` would answer
        // "No findings" for a credential sitting in a large file. A size limit is a
        // statement about what is worth *shipping*, never about what is worth looking
        // at.
        text_file_size_limit_enabled: false,
        ..context.config.clone()
    };

    let plan = codepack_engine::plan_export(
        &context.root,
        &scan_config,
        &std::collections::HashMap::new(),
        None,
        &cancel,
    )?;
    let relative_files: Vec<std::path::PathBuf> = plan
        .export_plan
        .included_files
        .iter()
        .map(|file| to_relative_path(&file.relative_path))
        .collect();

    let options = SecurityOptions::from(&scan_config);
    let result = scan_project(
        &context.root,
        &relative_files,
        options.max_bytes_per_file,
        &cancel,
    )?;

    Ok(assemble(context, relative_files.len(), &result))
}

fn assemble(context: &ProjectContext, scanned_files: usize, result: &ScanResult) -> ScanReport {
    let critical = result
        .findings
        .iter()
        .filter(|finding| finding.severity == "critical")
        .count();

    ScanReport {
        project: context.root.display().to_string(),
        safe_mode: "full".to_string(),
        scanned_files,
        summary: Summary {
            sensitive_files: result.summary.sensitive_files,
            potential_secrets: result.summary.potential_secrets,
            risky_code: result.summary.risky_code,
            total_findings: result.summary.total_findings,
            critical,
        },
        findings: result
            .findings
            .iter()
            .map(|finding| ReportedFinding {
                kind: kind_label(finding.kind),
                severity: finding.severity.clone(),
                confidence: finding.confidence.clone(),
                file: finding.file.clone(),
                line: finding.line,
                rule: finding.rule.clone(),
                message: finding.message.clone(),
            })
            .collect(),
    }
}

fn kind_label(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::SensitiveFile => "sensitive_file",
        FindingKind::PotentialSecret => "potential_secret",
        FindingKind::RiskyCode => "risky_code",
    }
}

/// The plan stores backslash-joined relative paths regardless of platform; rebuild a
/// real path rather than handing the string to `Path::new`, which would treat the whole
/// thing as one component on Unix.
fn to_relative_path(relative: &str) -> std::path::PathBuf {
    relative.split('\\').collect()
}

fn print_human(report: &ScanReport) {
    output::line(format!("Project:   {}", report.project));
    output::line(format!(
        "Scanned:   {} file(s) in {} mode",
        report.scanned_files, report.safe_mode
    ));
    output::line("");

    if report.findings.is_empty() {
        output::line("No findings.");
        return;
    }

    output::line(format!(
        "{} finding(s): {} sensitive file(s), {} potential secret(s), {} risky code",
        report.summary.total_findings,
        report.summary.sensitive_files,
        report.summary.potential_secrets,
        report.summary.risky_code
    ));
    if report.summary.critical > 0 {
        output::line(format!("{} of them are critical.", report.summary.critical));
    }
    output::line("");

    for finding in &report.findings {
        let location = match finding.line {
            Some(line) => format!("{}:{}", finding.file, line),
            None => finding.file.clone(),
        };
        output::line(format!(
            "  [{}] {} ({}) — {}",
            finding.severity, location, finding.rule, finding.message
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ResolutionTrace;

    fn context_with(root: &std::path::Path, config: Config) -> ProjectContext {
        ProjectContext {
            root: root.to_path_buf(),
            config,
            trace: ResolutionTrace::default(),
        }
    }

    #[test]
    fn a_text_file_size_limit_does_not_hide_a_secret_from_the_scan() {
        // Reachable through `--preset chatgpt`, a user profile, or `.codepack.toml`.
        // The scanner skips an over-limit file entirely, so without the override this
        // reported "No findings" for a file that plainly holds a credential — a silent
        // false negative in the command a CI gate runs.
        let dir = tempfile::tempdir().unwrap();
        let mut contents = "# padding
"
        .repeat(120_000);
        contents.push_str(
            "AWS_SECRET_ACCESS_KEY = \"wJalrXUtnFEMIK7MDENGbPxRfiCYEXAMPLEKEY\"
",
        );
        std::fs::write(dir.path().join("big.py"), &contents).unwrap();
        assert!(
            contents.len() > 1024 * 1024,
            "the file must exceed the limit"
        );

        let config = Config {
            text_file_size_limit_enabled: true,
            max_text_file_mb: 1,
            ..Config::default()
        };
        let report = build(&context_with(dir.path(), config)).unwrap();

        assert!(
            report.summary.total_findings > 0,
            "the credential must be reported despite the size limit"
        );
    }
}
