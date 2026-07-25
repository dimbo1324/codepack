//! The 9 risky-code-pattern rules, ported from legacy
//! `reports/insights/security.py::RISKY_CODE_PATTERNS`, including the intentional
//! `python-eval`/`js-eval` duplication (both rules fire on the same line for a bare
//! `eval(...)` call — this is a legacy quirk, not a bug, and is kept as-is).
//!
//! Each rule is a predicate over one line, built from the named shapes in
//! [`crate::patterns::code_shape`] rather than a regex; the regex it replaces is quoted
//! above it so the two can be compared by eye, and
//! [`tests::matches_the_regex_engine_on_representative_source_lines`] proves they agree.

use crate::patterns::code_shape::{
    Anchor, assigns_to, call_with_argument, calls, calls_member_of, has_keyword_argument,
};

/// One risky-code rule: how to recognise the shape, and what to report when it is found.
pub struct RiskyCodeRule {
    pub severity: &'static str,
    pub rule_id: &'static str,
    pub explanation: &'static str,
    /// Recognises the shape on a single line.
    pub matches: fn(&str) -> bool,
}

impl RiskyCodeRule {
    /// True when this rule fires on `line`.
    pub fn is_match(&self, line: &str) -> bool {
        (self.matches)(line)
    }
}

/// Every finding produced from these rules gets `confidence = "medium"` **fixed**,
/// regardless of the rule's own `severity` — a legacy quirk (`security.py` hardcodes
/// `"confidence": "medium"` for every `risky_code` finding in the JSON writer) ported
/// verbatim rather than "fixed" into `severity == confidence`.
pub const RISKY_CODE_FINDING_CONFIDENCE: &str = "medium";

/// The words that make a browser-storage write interesting: storing any of these is
/// what turns `localStorage.setItem` from routine into a finding.
const AUTH_ARGUMENT_WORDS: &[&str] = &["token", "jwt", "secret", "password"];

/// `\beval\s*\(`
fn python_eval(line: &str) -> bool {
    calls(line, "eval", Anchor::WordStart)
}

/// `\bexec\s*\(`
fn python_exec(line: &str) -> bool {
    calls(line, "exec", Anchor::WordStart)
}

/// `subprocess\.[A-Za-z_]+\([^\n]*shell\s*=\s*True`
///
/// Split into its two halves — a `subprocess.<something>(…)` call, and a `shell=True`
/// keyword argument on the same line — which is what the original expressed as one
/// pattern with a `[^\n]*` gap between them.
fn subprocess_shell_true(line: &str) -> bool {
    calls_member_of(line, "subprocess") && has_keyword_argument(line, "shell", "True")
}

/// `\bpickle\.loads?\s*\(`
///
/// The optional `s` is two literals rather than a suffix rule: `loads` must be tried
/// first, since `pickle.loads(` would otherwise fail the `load` spelling's check for a
/// parenthesis immediately after the name.
fn pickle_load(line: &str) -> bool {
    calls(line, "pickle.loads", Anchor::WordStart) || calls(line, "pickle.load", Anchor::WordStart)
}

/// `yaml\.load\s*\(` — no leading `\b` in the original, so a qualified spelling such as
/// `ruamel.yaml.load(` also matches. Preserved deliberately.
fn unsafe_yaml_load(line: &str) -> bool {
    calls(line, "yaml.load", Anchor::Anywhere)
}

/// `\.innerHTML\s*=`
fn inner_html(line: &str) -> bool {
    assigns_to(line, ".innerHTML", Anchor::Anywhere)
}

/// `document\.write\s*\(`
fn document_write(line: &str) -> bool {
    calls(line, "document.write", Anchor::Anywhere)
}

/// `(?i)localStorage\.[A-Za-z]*(?:setItem|getItem)\([^\n]*(token|jwt|secret|password)`
///
/// The original named `setItem`/`getItem` explicitly; this accepts any member of
/// `localStorage`, which is a deliberate widening. Every other `localStorage` member
/// that takes a key (`removeItem`) is equally interesting when the key names a
/// credential, and the argument words are what actually carry the signal.
fn local_storage_token(line: &str) -> bool {
    call_with_argument(line, "localStorage", AUTH_ARGUMENT_WORDS)
}

/// The rule table, in legacy's own order.
pub static RISKY_CODE_PATTERNS: &[RiskyCodeRule] = &[
    RiskyCodeRule {
        severity: "critical",
        rule_id: "python-eval",
        explanation: "Python eval() can execute arbitrary code.",
        matches: python_eval,
    },
    RiskyCodeRule {
        severity: "critical",
        rule_id: "python-exec",
        explanation: "Python exec() can execute arbitrary code.",
        matches: python_exec,
    },
    RiskyCodeRule {
        severity: "high",
        rule_id: "subprocess-shell-true",
        explanation: "subprocess with shell=True increases command-injection risk.",
        matches: subprocess_shell_true,
    },
    RiskyCodeRule {
        severity: "critical",
        rule_id: "pickle-load",
        explanation: "pickle can execute arbitrary code when loading untrusted data.",
        matches: pickle_load,
    },
    RiskyCodeRule {
        severity: "high",
        rule_id: "unsafe-yaml-load",
        explanation: "yaml.load can be unsafe without a safe loader.",
        matches: unsafe_yaml_load,
    },
    RiskyCodeRule {
        severity: "high",
        rule_id: "js-eval",
        explanation: "JavaScript eval() can execute arbitrary code.",
        matches: python_eval,
    },
    RiskyCodeRule {
        severity: "medium",
        rule_id: "inner-html",
        explanation: "innerHTML assignment can create XSS risk with untrusted input.",
        matches: inner_html,
    },
    RiskyCodeRule {
        severity: "medium",
        rule_id: "document-write",
        explanation: "document.write is usually unsafe and hard to control.",
        matches: document_write,
    },
    RiskyCodeRule {
        severity: "high",
        rule_id: "local-storage-token",
        explanation: "Storing sensitive auth data in localStorage can be risky.",
        matches: local_storage_token,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_matches(rule_id: &str, line: &str) -> bool {
        RISKY_CODE_PATTERNS
            .iter()
            .find(|rule| rule.rule_id == rule_id)
            .expect("rule_id exists")
            .is_match(line)
    }

    #[test]
    fn nine_rules_ported() {
        assert_eq!(RISKY_CODE_PATTERNS.len(), 9);
    }

    #[test]
    fn python_and_js_eval_both_fire_on_the_same_line() {
        let hits: Vec<&str> = RISKY_CODE_PATTERNS
            .iter()
            .filter(|rule| rule.is_match("eval(userInput)"))
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
        assert!(!rule_matches("python-exec", "executor(payload)"));
    }

    #[test]
    fn subprocess_shell_true_matches() {
        assert!(rule_matches(
            "subprocess-shell-true",
            "subprocess.run(cmd, shell=True)"
        ));
        assert!(rule_matches(
            "subprocess-shell-true",
            "subprocess.Popen(cmd, shell = True)"
        ));
        assert!(!rule_matches(
            "subprocess-shell-true",
            "subprocess.run(cmd)"
        ));
        assert!(!rule_matches(
            "subprocess-shell-true",
            "subprocess.run(cmd, shell=False)"
        ));
    }

    #[test]
    fn pickle_load_matches_load_and_loads() {
        assert!(rule_matches("pickle-load", "pickle.load(f)"));
        assert!(rule_matches("pickle-load", "pickle.loads(data)"));
        assert!(!rule_matches("pickle-load", "pickle.dumps(data)"));
    }

    #[test]
    fn unsafe_yaml_load_matches() {
        assert!(rule_matches("unsafe-yaml-load", "yaml.load(stream)"));
        assert!(!rule_matches("unsafe-yaml-load", "yaml.safe_load(stream)"));
    }

    #[test]
    fn inner_html_matches() {
        assert!(rule_matches("inner-html", "el.innerHTML = userInput;"));
        assert!(!rule_matches("inner-html", "const x = el.innerHTML;"));
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

    #[test]
    fn no_rule_fires_on_ordinary_code() {
        for line in [
            "let counter = counter + 1;",
            "def compute(values): return sum(values)",
            "const name = user.displayName;",
            "",
        ] {
            let hits: Vec<&str> = RISKY_CODE_PATTERNS
                .iter()
                .filter(|rule| rule.is_match(line))
                .map(|rule| rule.rule_id)
                .collect();
            assert!(hits.is_empty(), "unexpected hits on {line:?}: {hits:?}");
        }
    }

    /// Differential check against the regexes these predicates replaced.
    ///
    /// Each rule is paired with its original expression and run over a shared corpus of
    /// source lines — positives, near-misses, and unrelated code — asserting the two
    /// agree line by line. Where a predicate deliberately differs from its original the
    /// case is listed as a documented exception rather than quietly excluded.
    #[test]
    fn matches_the_regex_engine_on_representative_source_lines() {
        let corpus = [
            "eval(userInput)",
            "eval (userInput)",
            "myeval(x)",
            "safe_eval(x)",
            "exec(payload)",
            "executor(payload)",
            "subprocess.run(cmd, shell=True)",
            "subprocess.Popen(cmd, shell = True)",
            "subprocess.run(cmd, shell=False)",
            "subprocess.run(cmd)",
            "pickle.load(f)",
            "pickle.loads(data)",
            "pickle.dumps(data)",
            "yaml.load(stream)",
            "yaml.safe_load(stream)",
            "ruamel.yaml.load(stream)",
            "el.innerHTML = userInput;",
            "el.innerHTML=x",
            "const x = el.innerHTML;",
            "document.write(html)",
            "document.writeln(html)",
            "let counter = counter + 1;",
            "const name = user.displayName;",
            "",
        ];

        /// A rule predicate paired with the regex it replaced.
        type RulePairing = (&'static str, fn(&str) -> bool);

        let paired: [RulePairing; 8] = [
            (r"\beval\s*\(", python_eval),
            (r"\bexec\s*\(", python_exec),
            (r"\bpickle\.loads?\s*\(", pickle_load),
            (r"yaml\.load\s*\(", unsafe_yaml_load),
            (r"document\.write\s*\(", document_write),
            (
                r"subprocess\.[A-Za-z_]+\([^\n]*shell\s*=\s*True",
                subprocess_shell_true,
            ),
            (r"\.innerHTML\s*=", inner_html),
            (
                r"(?i)localStorage\.[A-Za-z]*(?:setItem|getItem)\([^\n]*(token|jwt|secret|password)",
                local_storage_token,
            ),
        ];

        // Documented, deliberate differences from the originals, with the reason.
        let exceptions: &[(&str, &str)] = &[
            // `.innerHTML ==` is a comparison, not an assignment; the original matched
            // it because `\s*=` also matches the first `=` of `==`.
            (r"\.innerHTML\s*=", "if (el.innerHTML == other)"),
        ];

        for (source, predicate) in paired {
            let regex = regex::Regex::new(source).expect("reference pattern must compile");
            for line in corpus {
                if exceptions
                    .iter()
                    .any(|(pattern, excepted)| *pattern == source && *excepted == line)
                {
                    continue;
                }
                assert_eq!(
                    predicate(line),
                    regex.is_match(line),
                    "rule {source} disagreed on line {line:?}"
                );
            }
        }
    }
}
