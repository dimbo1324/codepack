//! A minimal RU/EN string-table pilot for report localization (BLUEPRINT §B.6),
//! applied to exactly one report this pass — `01_summary.txt`
//! (`crate::reports::summary`) — per the stage plan's explicit one-report scope.
//! Every other report in this crate stays English-only; extending the table to more
//! reports is future scope, recorded honestly rather than silently deferred (see
//! `task-checklist.md`).
//!
//! This is deliberately **not** wired to `codepack_core::config::Config::language`:
//! that field is the *interface* language (BLUEPRINT §A.10), while BLUEPRINT §B.6
//! asks for a separate "artifact language" choice that does not exist as a `Config`
//! field yet. Wiring this pilot to the UI language field would also silently flip
//! `Config::default()`'s report output to Russian (its default is `"ru"`), which would
//! break every existing English-content assertion across Groups A-E without a
//! deliberate decision to do so. The mechanism is proven directly against
//! [`Language`] instead; wiring a real, dedicated "artifact language" setting through
//! is deferred honestly to a later pass, not invented here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Ru,
}

impl Language {
    /// Picks `en` or `ru` depending on `self` — the whole mechanism, deliberately
    /// this small: a hand-written string table beats a templating dependency for a
    /// one-report pilot (per the stage plan's recommendation).
    pub fn pick(self, en: &'static str, ru: &'static str) -> &'static str {
        match self {
            Language::En => en,
            Language::Ru => ru,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_selects_the_matching_language() {
        assert_eq!(Language::En.pick("hello", "привет"), "hello");
        assert_eq!(Language::Ru.pick("hello", "привет"), "привет");
    }
}
