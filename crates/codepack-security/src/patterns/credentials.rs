//! Structural detection of credentials embedded in URLs and HTTP auth headers.
//!
//! **Finding 2, 2026-07-27 audit.** Two shapes the rest of the crate could not see:
//! a password inside `scheme://user:password@host`, and `Basic`/`Digest` HTTP
//! authentication — `Bearer` was already covered
//! ([`crate::patterns::keyword_scan::find_bearer_tokens`]), the other two schemes from
//! the same RFC were not.
//!
//! Both matchers work on **structure**, not on entropy or an alphabet-scoring
//! heuristic. Q18 (`docs/decisions/open-questions.md`) recorded that a short,
//! word-shaped password (`hunter2`) cannot be told apart from a hostname *by its
//! shape* — but its *position* in the URL identifies it regardless of shape, which is
//! what [`find_url_credentials`] matches on. A seven-character password is therefore
//! found exactly as reliably as a forty-character one.

use crate::patterns::keyword_scan::{
    BEARER_MIN_TOKEN_LEN, is_bearer_token_char, is_word_bounded, skip_spaces,
};

/// The password span inside every `scheme://user:password@host` occurrence on `line`.
///
/// Deliberately does **not** restrict which scheme precedes `://` to a connection-string
/// allow-list: the structural gate below is what keeps this precise, not a list of
/// known schemes, so it catches `https://user:pass@host` exactly as well as
/// `postgres://user:pass@host`.
///
/// The authority component — the text between `://` and the first `/`, `?`, `#`, or
/// whitespace — is scanned for a `user:password@` shape. This is what correctly
/// rejects `http://host/a:b@c`: there the `@` sits in the *path*, not the authority,
/// because a `/` appears first, so no span is produced. Extracting only the substring
/// before that boundary is what makes the rejection structural rather than a special
/// case bolted on afterward.
pub fn find_url_credentials(line: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;

    while let Some(offset) = line[cursor..].find("://") {
        let after_scheme = cursor + offset + 3;

        let authority_end = line[after_scheme..]
            .find(|c: char| c == '/' || c == '?' || c == '#' || c.is_whitespace())
            .map(|relative| after_scheme + relative)
            .unwrap_or(line.len());
        let authority = &line[after_scheme..authority_end];

        // The rightmost '@' is the userinfo/host separator per the URL grammar: a
        // password may itself contain '@' (percent-encoded in a real URL, but this
        // scanner reads raw text), so the *last* '@' in the authority is the reliable
        // choice — an earlier one can only belong to the userinfo, never to the host.
        if let Some(at_relative) = authority.rfind('@') {
            let userinfo = &authority[..at_relative];
            if let Some(colon_relative) = userinfo.find(':') {
                let start = after_scheme + colon_relative + 1;
                let end = after_scheme + at_relative;
                if end > start {
                    spans.push((start, end));
                }
            }
        }

        // Always past "://" itself (never past authority_end, which can equal
        // after_scheme when the authority is empty), so the next search cannot re-find
        // the same "://" and loop forever.
        cursor = after_scheme;
    }

    spans
}

/// The `Authorization` schemes whose value starts with a bare token this scanner can
/// recognise. `BEARER` is listed for completeness of this function's own contract, but
/// in practice a `Bearer` line never reaches this rule in the scan cascade — it was
/// already reported at `high` confidence by the keyword cascade
/// (`crate::patterns::keyword::has_secret_with_value`) before this function is called;
/// see `scan::collect_secret_hits`'s ordering.
const AUTH_SCHEMES: &[&str] = &["BEARER", "BASIC", "DIGEST"];

/// Finds every `<scheme> <token>` span for the schemes in [`AUTH_SCHEMES`], using the
/// exact alphabet and length floor `Bearer` already uses
/// ([`is_bearer_token_char`]/[`BEARER_MIN_TOKEN_LEN`]) so the three schemes cannot
/// silently diverge in what counts as a token.
///
/// Real `Digest` authentication is a comma-separated parameter list
/// (`Digest username="...", realm="...", response="..."`), not a single token, and
/// this is honest about not parsing that list: every field is individually quoted and
/// short, so a typical Digest header clears no length floor here and is not caught.
/// What this does catch is the less common but real case of a client or proxy logging
/// an unquoted value (a bare `response=<hex>` hash) after the scheme name.
pub fn find_http_auth_tokens(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();

    for scheme in AUTH_SCHEMES {
        let mut cursor = 0usize;
        while cursor < line.len() {
            let Some(offset) = line[cursor..].to_ascii_uppercase().find(scheme) else {
                break;
            };
            let start = cursor + offset;
            let after_keyword = start + scheme.len();

            if !is_word_bounded(line, start, after_keyword) {
                cursor = start + 1;
                continue;
            }

            // `\s+`: at least one space, matching Bearer's own grammar.
            let token_start = skip_spaces(bytes, after_keyword);
            if token_start == after_keyword {
                cursor = after_keyword;
                continue;
            }

            let mut token_end = token_start;
            while token_end < bytes.len() && is_bearer_token_char(bytes[token_end]) {
                token_end += 1;
            }

            if token_end - token_start >= BEARER_MIN_TOKEN_LEN {
                spans.push((start, token_end));
                cursor = token_end;
            } else {
                cursor = after_keyword;
            }
        }
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans_of<'a>(line: &'a str, spans: &[(usize, usize)]) -> Vec<&'a str> {
        spans.iter().map(|&(s, e)| &line[s..e]).collect()
    }

    #[test]
    fn finds_the_password_between_colon_and_at_sign() {
        let line = "db.connect('postgres://admin:hunter2fakepass@host/db?sslmode=require')";
        let spans = find_url_credentials(line);
        assert_eq!(spans_of(line, &spans), vec!["hunter2fakepass"]);
    }

    #[test]
    fn a_short_password_is_found_exactly_as_reliably_as_a_long_one() {
        // Q18's whole point: shape cannot tell "hunter2" from a hostname, position can.
        let line = "postgres://admin:hunter2@host/db";
        assert_eq!(spans_of(line, &find_url_credentials(line)), vec!["hunter2"]);
    }

    #[test]
    fn works_for_any_scheme_not_just_database_connection_strings() {
        let line = "https://deploy:s3cr3t-fake-token@ci.example.com/hook";
        assert_eq!(
            spans_of(line, &find_url_credentials(line)),
            vec!["s3cr3t-fake-token"]
        );
    }

    #[test]
    fn a_path_that_merely_looks_like_userinfo_is_not_a_match() {
        // The '@' here is in the path, not the authority: a '/' appears first.
        assert!(find_url_credentials("http://host/a:b@c").is_empty());
    }

    #[test]
    fn no_password_no_match() {
        assert!(find_url_credentials("https://readonly@cdn.example.com/assets").is_empty());
        assert!(find_url_credentials("https://example.com/path").is_empty());
    }

    #[test]
    fn no_scheme_separator_no_match() {
        assert!(find_url_credentials("just some text with a : and an @ in it").is_empty());
    }

    #[test]
    fn multiple_urls_on_one_line_are_all_found() {
        let line =
            "primary=postgres://a:onefakepass@host1/db mirror=mysql://b:twofakepass@host2/db";
        assert_eq!(
            spans_of(line, &find_url_credentials(line)),
            vec!["onefakepass", "twofakepass"]
        );
    }

    #[test]
    fn an_empty_authority_does_not_loop_forever() {
        assert!(find_url_credentials("file:///etc/passwd").is_empty());
    }

    #[test]
    fn finds_basic_auth_token() {
        let line = "headers = {'Authorization': 'Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk'}";
        let spans = find_http_auth_tokens(line);
        assert_eq!(
            spans_of(line, &spans),
            vec!["Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3Jk"]
        );
    }

    #[test]
    fn finds_digest_auth_token() {
        // Honest about the limit: a real Digest header is
        // `Digest username="...", realm="...", response="..."` — every field
        // individually quoted, so no single run after the keyword clears the length
        // floor, and this function does not attempt to parse the whole parameter list
        // (see the module doc). What it does catch is a client or proxy that logs the
        // response hash unquoted, which real deployments do.
        let line = "Authorization: Digest response=1234567890abcdef1234567890abcdef";
        assert!(!find_http_auth_tokens(line).is_empty());
    }

    #[test]
    fn a_realistic_quoted_digest_header_is_not_caught_by_this_heuristic() {
        // Documented limitation, not a silent gap: every field is short and quoted, so
        // no run after "Digest" clears BEARER_MIN_TOKEN_LEN. Scanning the full
        // comma-separated parameter list is future work, not attempted here.
        let line = r#"Authorization: Digest username="fakeuser", realm="example.com""#;
        assert!(find_http_auth_tokens(line).is_empty());
    }

    #[test]
    fn short_word_after_basic_is_prose_not_a_match() {
        assert!(find_http_auth_tokens("Basic training starts Monday").is_empty());
    }

    #[test]
    fn bearer_is_still_recognised_by_this_function_too() {
        // Not the rule that actually fires in the scan cascade for a Bearer line (the
        // keyword cascade wins first), but this function's own contract must hold.
        let line = "Authorization: Bearer abcdefghijklmnopqrstuvwxyz";
        assert!(!find_http_auth_tokens(line).is_empty());
    }

    #[test]
    fn glued_to_a_longer_word_is_not_a_match() {
        assert!(find_http_auth_tokens("BasicAuthenticationHandler").is_empty());
    }

    #[test]
    fn empty_input_is_handled() {
        assert!(find_url_credentials("").is_empty());
        assert!(find_http_auth_tokens("").is_empty());
    }
}
