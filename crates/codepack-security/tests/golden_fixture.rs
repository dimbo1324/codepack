//! End-to-end golden test: one small synthetic project exercising every finding
//! category legacy `reports/insights/security.py` produced — sensitive files, all
//! four secret-confidence tiers, all nine risky-code rules (including the intentional
//! `python-eval`/`js-eval` duplication), and the self-protection exemption — verified
//! through the public `scan_project` entry point rather than any single detector
//! module in isolation.

use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_security::{FindingKind, scan_project};

fn write_file(root: &Path, relative: &str, content: &str) -> PathBuf {
    let full = root.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, content).unwrap();
    PathBuf::from(relative)
}

#[test]
fn scan_project_reproduces_the_full_legacy_finding_set() {
    let dir = tempfile::tempdir().unwrap();

    let env_file = write_file(dir.path(), ".env", "API_KEY=totally-fake-abc123\n");
    let id_rsa = write_file(dir.path(), "id_rsa", "ssh-rsa AAAAB3NzaC1yc2EAAAADAQAB\n");
    let credentials = write_file(dir.path(), "credentials.json", "{}\n");

    let app_py = write_file(
        dir.path(),
        "app.py",
        concat!(
            "-----BEGIN RSA PRIVATE KEY-----\n",
            "PASSWORD = \"hunter2long\"\n",
            "token: \n",
            "we should rotate the client_secret manually\n",
            "eval(user_input)\n",
            "exec(payload)\n",
            "subprocess.run(cmd, shell=True)\n",
            "pickle.loads(data)\n",
            "yaml.load(stream)\n",
            "# see SECRET_PATTERNS and redact_secrets for details\n",
        ),
    );

    let app_js = write_file(
        dir.path(),
        "app.js",
        concat!(
            "el.innerHTML = userInput;\n",
            "document.write(html);\n",
            "localStorage.setItem('authToken', token);\n",
        ),
    );

    let files = vec![env_file, id_rsa, credentials, app_py, app_js];
    let result = scan_project(dir.path(), &files, None, &CancellationToken::new()).unwrap();

    // --- Sensitive files: three distinct files, matching legacy's severity rule. ---
    let sensitive: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::SensitiveFile)
        .collect();
    assert_eq!(
        sensitive.len(),
        3,
        "expected exactly 3 sensitive-file findings"
    );
    assert!(
        sensitive
            .iter()
            .any(|f| f.file.ends_with(".env") && f.severity == "critical")
    );
    assert!(
        sensitive
            .iter()
            .any(|f| f.file.ends_with("id_rsa") && f.severity == "high")
    );
    assert!(
        sensitive
            .iter()
            .any(|f| f.file.ends_with("credentials.json") && f.severity == "high")
    );
    assert!(sensitive.iter().all(|f| f.rule == "sensitive_filename"));
    assert!(sensitive.iter().all(|f| f.confidence == "high"));

    // --- Potential secrets: all four confidence tiers present. ---
    let secrets: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::PotentialSecret)
        .collect();
    for tier in ["critical", "high", "medium", "low"] {
        assert!(
            secrets.iter().any(|f| f.confidence == tier),
            "missing a {tier}-confidence potential-secret finding"
        );
    }
    assert!(
        secrets
            .iter()
            .any(|f| f.confidence == "critical" && f.rule == "secret_like_line")
    );

    // --- Self-protection: the comment mentioning SECRET_PATTERNS/redact_secrets must
    // not itself produce a potential-secret finding. ---
    assert!(
        !secrets
            .iter()
            .any(|f| f.message.contains("SECRET_PATTERNS") || f.message.contains("redact_secrets"))
    );

    // --- Risky code: all nine rules fire, including the intentional duplication. ---
    let risky_rules: std::collections::HashSet<&str> = result
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::RiskyCode)
        .map(|f| f.rule.as_str())
        .collect();
    for rule in [
        "python-eval",
        "python-exec",
        "subprocess-shell-true",
        "pickle-load",
        "unsafe-yaml-load",
        "js-eval",
        "inner-html",
        "document-write",
        "local-storage-token",
    ] {
        assert!(risky_rules.contains(rule), "missing risky-code rule {rule}");
    }
    assert!(
        result
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::RiskyCode)
            .all(|f| f.confidence == "medium"),
        "every risky-code finding must carry the fixed legacy confidence of medium"
    );

    assert_eq!(result.summary.total_findings, result.findings.len());
    assert_eq!(result.summary.sensitive_files, sensitive.len());
    assert_eq!(result.summary.potential_secrets, secrets.len());
}
