use std::sync::LazyLock;

use aho_corasick::AhoCorasick;

const LITERALS: &[&str] = &[
    "API_KEY",
    "API-KEY",
    "APIKEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASS",
    "PRIVATE_KEY",
    "PRIVATE-KEY",
    "PRIVATEKEY",
    "BEARER",
    "DATABASE_URL",
    "DATABASE-URL",
    "DATABASEURL",
    "JWT_SECRET",
    "JWT-SECRET",
    "JWTSECRET",
    "ACCESS_KEY",
    "ACCESS-KEY",
    "ACCESSKEY",
    "CLIENT_SECRET",
    "CLIENT-SECRET",
    "CLIENTSECRET",
    "AKIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "AIza",
    "xoxb-",
    "xoxa-",
    "xoxp-",
    "xoxr-",
    "xoxs-",
    "sk_live_",
    "rk_live_",
    "sk-",
    "sk-ant-",
    "-----BEGIN",
    "eyJ",
];

static AUTOMATON: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(LITERALS)
        .expect("fixed literal set builds into a valid Aho-Corasick automaton")
});

pub fn has_hit(line: &str) -> bool {
    AUTOMATON.is_match(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_on_keyword_roots_regardless_of_separator_style() {
        assert!(has_hit("api_key = value"));
        assert!(has_hit("API-KEY: value"));
        assert!(has_hit("apikey=value"));
    }

    #[test]
    fn hits_on_provider_prefixes() {
        assert!(has_hit("AKIAABCDEFGHIJKLMNOP"));
        assert!(has_hit("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(has_hit("sk-ant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(has_hit("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(has_hit("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig"));
    }

    #[test]
    fn case_insensitive_matching() {
        assert!(has_hit("bearer abcdefghijklmnopqrstuvwxyz"));
        assert!(has_hit("Secret: value"));
    }

    #[test]
    fn no_hit_on_unrelated_line() {
        assert!(!has_hit("let counter = counter + 1;"));
    }

    #[test]
    fn no_hit_on_telegram_shaped_line_documented_scope_limit() {
        assert!(!has_hit("123456789:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }
}
