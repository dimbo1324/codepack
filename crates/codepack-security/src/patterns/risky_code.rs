//! The 9 risky-code-pattern rules, ported verbatim from legacy
//! `reports/insights/security.py::RISKY_CODE_PATTERNS`, including the intentional
//! `python-eval`/`js-eval` regex duplication (both rules fire on the same line for a
//! bare `eval(...)` call — this is a legacy quirk, not a bug, and is kept as-is).

use std::sync::LazyLock;

use regex::Regex;

pub struct RiskyCodeRule {
    pub severity: &'static str,
    pub rule_id: &'static str,
    pub explanation: &'static str,
    pub regex: Regex,
}

/// Every finding produced from these rules gets `confidence = "medium"` **fixed**,
/// regardless of the rule's own `severity` — a legacy quirk (`security.py` hardcodes
/// `"confidence": "medium"` for every `risky_code` finding in the JSON writer) ported
/// verbatim rather than "fixed" into `severity == confidence`.
pub const RISKY_CODE_FINDING_CONFIDENCE: &str = "medium";

pub static RISKY_CODE_PATTERNS: LazyLock<Vec<RiskyCodeRule>> = LazyLock::new(|| {
    vec![
        RiskyCodeRule {
            severity: "critical",
            rule_id: "python-eval",
            explanation: "Python eval() can execute arbitrary code.",
            regex: Regex::new(r"\beval\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "critical",
            rule_id: "python-exec",
            explanation: "Python exec() can execute arbitrary code.",
            regex: Regex::new(r"\bexec\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "high",
            rule_id: "subprocess-shell-true",
            explanation: "subprocess with shell=True increases command-injection risk.",
            regex: Regex::new(r"subprocess\.[A-Za-z_]+\([^\n]*shell\s*=\s*True")
                .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "critical",
            rule_id: "pickle-load",
            explanation: "pickle can execute arbitrary code when loading untrusted data.",
            regex: Regex::new(r"\bpickle\.loads?\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "high",
            rule_id: "unsafe-yaml-load",
            explanation: "yaml.load can be unsafe without a safe loader.",
            regex: Regex::new(r"yaml\.load\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "high",
            rule_id: "js-eval",
            explanation: "JavaScript eval() can execute arbitrary code.",
            regex: Regex::new(r"\beval\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "medium",
            rule_id: "inner-html",
            explanation: "innerHTML assignment can create XSS risk with untrusted input.",
            regex: Regex::new(r"\.innerHTML\s*=").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "medium",
            rule_id: "document-write",
            explanation: "document.write is usually unsafe and hard to control.",
            regex: Regex::new(r"document\.write\s*\(").expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
        RiskyCodeRule {
            severity: "high",
            rule_id: "local-storage-token",
            explanation: "Storing sensitive auth data in localStorage can be risky.",
            regex: Regex::new(
                r"(?i)localStorage\.[A-Za-z]*(?:setItem|getItem)\([^\n]*(token|jwt|secret|password)",
            )
            .expect("hand-written pattern literal is a valid regex, proven by test coverage"),
        },
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_rules_ported() {
        assert_eq!(RISKY_CODE_PATTERNS.len(), 9);
    }

    #[test]
    fn python_and_js_eval_both_fire_on_the_same_line() {
        let hits: Vec<&str> = RISKY_CODE_PATTERNS
            .iter()
            .filter(|rule| rule.regex.is_match("eval(userInput)"))
            .map(|rule| rule.rule_id)
            .collect();
        assert!(hits.contains(&"python-eval"));
        assert!(hits.contains(&"js-eval"));
        assert_eq!(
            hits.len(),
            2,
            "duplication is intentional, not deduplicated"
        );
    }

    #[test]
    fn python_exec_matches() {
        assert!(rule_matches("python-exec", "exec(payload)"));
    }

    #[test]
    fn subprocess_shell_true_matches() {
        assert!(rule_matches(
            "subprocess-shell-true",
            "subprocess.run(cmd, shell=True)"
        ));
        assert!(!rule_matches(
            "subprocess-shell-true",
            "subprocess.run(cmd)"
        ));
    }

    #[test]
    fn pickle_load_matches_load_and_loads() {
        assert!(rule_matches("pickle-load", "pickle.load(f)"));
        assert!(rule_matches("pickle-load", "pickle.loads(data)"));
    }

    #[test]
    fn unsafe_yaml_load_matches() {
        assert!(rule_matches("unsafe-yaml-load", "yaml.load(stream)"));
    }

    #[test]
    fn inner_html_matches() {
        assert!(rule_matches("inner-html", "el.innerHTML = userInput;"));
    }

    #[test]
    fn document_write_matches() {
        assert!(rule_matches("document-write", "document.write(html)"));
    }

    #[test]
    fn local_storage_token_matches_case_insensitively() {
        assert!(rule_matches(
            "local-storage-token",
            "localStorage.setItem('authToken', token)"
        ));
        assert!(rule_matches(
            "local-storage-token",
            "localStorage.getItem('JWT')"
        ));
        assert!(!rule_matches(
            "local-storage-token",
            "localStorage.setItem('theme', 'dark')"
        ));
    }

    fn rule_matches(rule_id: &str, line: &str) -> bool {
        RISKY_CODE_PATTERNS
            .iter()
            .find(|rule| rule.rule_id == rule_id)
            .expect("rule_id exists")
            .regex
            .is_match(line)
    }
}
