//! Tests for [`super`], the token matcher.
//!
//! Split out on 2026-07-27: `token_scan.rs` had grown to 825 lines, past the
//! project's own ~600-line limit (`.ai/project/12-domain-rules.md`), and the tests
//! were the larger half. The module's public surface is unchanged — it is a
//! directory module now, which callers never see.

use super::*;

fn spans(line: &str, pattern: &TokenPattern) -> Vec<(usize, usize)> {
    find_matches(line, pattern)
        .into_iter()
        .map(|found| (found.value_start, found.value_end))
        .collect()
}

fn matched<'a>(line: &'a str, pattern: &TokenPattern) -> Vec<&'a str> {
    find_matches(line, pattern)
        .into_iter()
        .map(|found| &line[found.value_start..found.value_end])
        .collect()
}

#[test]
fn aws_access_key_matches_exactly_sixteen_trailing_characters() {
    let key = "AKIA".to_string() + &"A".repeat(16);
    assert_eq!(matched(&key, &AWS_ACCESS_KEY_ID), vec![key.as_str()]);
    // One character short: no match at all.
    let short = "AKIA".to_string() + &"A".repeat(15);
    assert!(matched(&short, &AWS_ACCESS_KEY_ID).is_empty());
}

#[test]
fn aws_access_key_is_case_sensitive_in_both_prefix_and_body() {
    // Lowercase body characters are outside `[0-9A-Z]`, so a lowercase run cannot
    // reach the required length.
    let lower = "AKIA".to_string() + &"a".repeat(16);
    assert!(matched(&lower, &AWS_ACCESS_KEY_ID).is_empty());
    let wrong_prefix = "akia".to_string() + &"A".repeat(16);
    assert!(matched(&wrong_prefix, &AWS_ACCESS_KEY_ID).is_empty());
}

#[test]
fn pem_header_backtracks_past_the_greedy_uppercase_run() {
    // The regression this backtracking exists for: `[A-Z ]*` greedily eats
    // "RSA PRIVATE KEY" and the trailing literal must still find its text.
    assert!(is_match(
        "-----BEGIN RSA PRIVATE KEY-----",
        &PEM_PRIVATE_KEY
    ));
    assert!(is_match("-----BEGIN PRIVATE KEY-----", &PEM_PRIVATE_KEY));
    assert!(is_match(
        "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        &PEM_PRIVATE_KEY
    ));
    assert!(!is_match("-----BEGIN CERTIFICATE-----", &PEM_PRIVATE_KEY));
    assert!(!is_match(
        "-----BEGIN rsa PRIVATE KEY-----",
        &PEM_PRIVATE_KEY
    ));
}

#[test]
fn jwt_requires_all_three_segments() {
    assert!(is_match("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig", &JWT));
    assert!(!is_match("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0", &JWT));
    assert!(!is_match("eyJhbGciOiJIUzI1NiJ9", &JWT));
}

#[test]
fn aws_secret_reports_only_the_value_segment() {
    let value = "a".repeat(20) + &"B7c".repeat(6) + "de";
    assert_eq!(value.len(), 40);
    let line = format!("aws_secret_access_key={value}");

    assert_eq!(
        matched(&line, &AWS_SECRET_WITH_PREFIX),
        vec![value.as_str()]
    );
}

#[test]
fn aws_secret_accepts_every_documented_field_spelling() {
    let value = "a".repeat(20) + &"B7c".repeat(6) + "de";
    for line in [
        format!("aws_secret_access_key = {value}"),
        format!("AWS_SECRET_ACCESS_KEY={value}"),
        format!("aws.secret.access.key={value}"),
        format!("aws-secret-access-key={value}"),
    ] {
        assert_eq!(
            matched(&line, &AWS_SECRET_WITH_PREFIX),
            vec![value.as_str()],
            "should have matched: {line}"
        );
    }
    // The SDK spelling has no `aws` word, so the bare pattern covers it.
    let js = format!("  secretAccessKey: \"{value}\",");
    assert_eq!(matched(&js, &AWS_SECRET_BARE), vec![value.as_str()]);
}

#[test]
fn aws_secret_without_its_field_name_is_not_a_match() {
    // 40 base64 characters alone are indistinguishable from a hash or build id;
    // matching them bare is what would cost precision (invariant I9).
    let value = "a".repeat(20) + &"B7c".repeat(6) + "de";
    assert!(matched(&value, &AWS_SECRET_WITH_PREFIX).is_empty());
    assert!(matched(&value, &AWS_SECRET_BARE).is_empty());
    assert!(matched(&format!("build_id = {value}"), &AWS_SECRET_BARE).is_empty());
}

#[test]
fn aws_secret_value_longer_than_forty_is_covered_completely() {
    // A span covering only the first 40 characters would leave the tail unmasked.
    let value = "a".repeat(20) + &"B7c".repeat(8) + "de";
    assert!(value.len() > 40);
    let line = format!("aws_secret_access_key={value}");
    assert_eq!(
        matched(&line, &AWS_SECRET_WITH_PREFIX),
        vec![value.as_str()]
    );
}

#[test]
fn telegram_token_requires_the_digit_run_and_exact_body_length() {
    let token = "123456789:".to_string() + &"a".repeat(35);
    assert!(is_match(&token, &TELEGRAM_BOT_TOKEN));
    // Seven digits is below the minimum.
    assert!(!is_match(
        &("1234567:".to_string() + &"a".repeat(35)),
        &TELEGRAM_BOT_TOKEN
    ));
    // 34 body characters is below the exact length.
    assert!(!is_match(
        &("123456789:".to_string() + &"a".repeat(34)),
        &TELEGRAM_BOT_TOKEN
    ));
}

#[test]
fn matches_are_non_overlapping_and_left_to_right() {
    let first = "AKIA".to_string() + &"A".repeat(16);
    let second = "AKIA".to_string() + &"B".repeat(16);
    let line = format!("{first} and {second}");
    assert_eq!(
        matched(&line, &AWS_ACCESS_KEY_ID),
        vec![first.as_str(), second.as_str()]
    );
}

#[test]
fn an_unbounded_run_stops_at_the_first_character_outside_its_class() {
    let key = "sk-ant-".to_string() + &"a".repeat(25);
    let line = format!("{key} trailing text");
    assert_eq!(matched(&line, &ANTHROPIC_API_KEY), vec![key.as_str()]);
}

#[test]
fn empty_input_never_matches_and_never_loops() {
    assert!(matched("", &AWS_ACCESS_KEY_ID).is_empty());
    assert!(matched("", &JWT).is_empty());
    assert!(matched("", &PEM_PRIVATE_KEY).is_empty());
}

#[test]
fn non_ascii_bytes_terminate_a_run_without_panicking() {
    // Runs walk raw bytes, so a multi-byte character must simply fail the class
    // test rather than slice a character in half.
    let line = format!("AKIA{}", "Ф".repeat(20));
    assert!(matched(&line, &AWS_ACCESS_KEY_ID).is_empty());
    let mixed = "sk-ant-".to_string() + &"a".repeat(25) + "Ф";
    assert_eq!(
        matched(&mixed, &ANTHROPIC_API_KEY),
        vec![("sk-ant-".to_string() + &"a".repeat(25)).as_str()]
    );
}

/// Differential check against the regexes these patterns replaced.
///
/// The transcriptions above are readable, but readability is not equivalence. Each
/// case pairs a pattern with the original expression and a set of inputs chosen to
/// straddle its boundaries — correct length, one short, one long, wrong case, wrong
/// alphabet, embedded in surrounding text — and asserts the two agree on both the
/// yes/no answer and the reported span.
#[test]
fn matches_the_regex_engine_across_a_generated_corpus() {
    let alnum = "aB3";
    let cases: Vec<(&str, &TokenPattern, Vec<String>)> = vec![
        (
            r"AKIA[0-9A-Z]{16}",
            &AWS_ACCESS_KEY_ID,
            vec![
                format!("AKIA{}", "A".repeat(16)),
                format!("AKIA{}", "A".repeat(15)),
                format!("AKIA{}", "A".repeat(17)),
                format!("AKIA{}", "a".repeat(16)),
                format!("prefix AKIA{} suffix", "Z9".repeat(8)),
                "AKIA".to_string(),
                String::new(),
            ],
        ),
        (
            r"github_pat_[A-Za-z0-9_]{82}",
            &GITHUB_FINE_GRAINED_TOKEN,
            vec![
                format!("github_pat_{}", "a".repeat(82)),
                format!("github_pat_{}", "a".repeat(81)),
                format!("github_pat_{}", "-".repeat(82)),
            ],
        ),
        (
            r"AIza[0-9A-Za-z\-_]{35}",
            &GOOGLE_API_KEY,
            vec![
                format!("AIza{}", alnum.repeat(12)),
                format!("AIza{}", "a".repeat(34)),
                format!("AIza{}", "-_a".repeat(12)),
            ],
        ),
        (
            r"sk-ant-[A-Za-z0-9_-]{20,}",
            &ANTHROPIC_API_KEY,
            vec![
                format!("sk-ant-{}", "a".repeat(20)),
                format!("sk-ant-{}", "a".repeat(19)),
                format!("sk-ant-{}", "a".repeat(60)),
                format!("sk-ant-{} tail", "a".repeat(30)),
            ],
        ),
        (
            r"sk-[A-Za-z0-9]{20,}",
            &OPENAI_API_KEY,
            vec![
                format!("sk-{}", "a".repeat(20)),
                format!("sk-{}", "a".repeat(19)),
                format!("sk-{}-tail", "a".repeat(25)),
            ],
        ),
        (
            r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
            &JWT,
            vec![
                "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig".to_string(),
                "eyJa.b.c".to_string(),
                "eyJa.b".to_string(),
                "eyJ.b.c".to_string(),
                "prefix eyJa.b.c suffix".to_string(),
            ],
        ),
        (
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            &PEM_PRIVATE_KEY,
            vec![
                "-----BEGIN RSA PRIVATE KEY-----".to_string(),
                "-----BEGIN PRIVATE KEY-----".to_string(),
                "-----BEGIN ENCRYPTED PRIVATE KEY-----".to_string(),
                "-----BEGIN CERTIFICATE-----".to_string(),
                "x -----BEGIN EC PRIVATE KEY----- y".to_string(),
            ],
        ),
        (
            r"\d{8,10}:[A-Za-z0-9_-]{35}",
            &TELEGRAM_BOT_TOKEN,
            vec![
                format!("12345678:{}", "a".repeat(35)),
                format!("1234567890:{}", "a".repeat(35)),
                format!("1234567:{}", "a".repeat(35)),
                format!("12345678:{}", "a".repeat(34)),
                format!("call 123456789:{} now", "a".repeat(35)),
            ],
        ),
    ];

    for (source, pattern, inputs) in cases {
        let regex = regex::Regex::new(source).expect("reference pattern must compile");
        for input in inputs {
            let ours = spans(&input, pattern);
            let theirs: Vec<(usize, usize)> = regex
                .find_iter(&input)
                .map(|found| (found.start(), found.end()))
                .collect();
            assert_eq!(
                ours, theirs,
                "pattern {source} disagreed on input {input:?}"
            );
        }
    }
}

/// The context-anchored AWS rule, checked the same way but against its own regex,
/// comparing the reported *value* span rather than the whole match.
#[test]
fn aws_secret_value_span_matches_the_regex_capture_group() {
    let regex = regex::Regex::new(
        r#"(?i)(?:aws[_.\-]?)?secret[_.\-]?access[_.\-]?key["']?[ \t]*[:=][ \t]*["']?([A-Za-z0-9/+=]{40,})"#,
    )
    .expect("reference pattern must compile");

    let value = "a".repeat(20) + &"B7c".repeat(6) + "de";
    let long_value = "a".repeat(20) + &"B7c".repeat(9);
    let inputs = vec![
        format!("aws_secret_access_key={value}"),
        format!("AWS_SECRET_ACCESS_KEY = {value}"),
        format!("  secretAccessKey: \"{value}\","),
        format!("aws.secret.access.key={long_value}"),
        format!("secret_access_key='{value}'"),
        format!("build_id = {value}"),
        format!("aws_secret_access_key={}", "a".repeat(39)),
    ];

    for input in inputs {
        let expected: Vec<(usize, usize)> = regex
            .captures_iter(&input)
            .filter_map(|caps| caps.get(1))
            .map(|group| (group.start(), group.end()))
            .collect();

        // Either registration may carry the match; the scanner tries both, so the
        // union is what must agree with the capture group.
        let mut ours = spans(&input, &AWS_SECRET_WITH_PREFIX);
        if ours.is_empty() {
            ours = spans(&input, &AWS_SECRET_BARE);
        }
        assert_eq!(ours, expected, "AWS rule disagreed on input {input:?}");
    }
}
