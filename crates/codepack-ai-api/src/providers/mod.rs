//! One module per provider. Adding a second one means adding a file here and a line in
//! [`resolve`] — nothing else in the crate, and nothing outside it, needs to change.

pub mod anthropic;

use crate::provider::AiProvider;
use codepack_ai::error::AiError;

/// Every provider this build supports.
pub fn all() -> Vec<Box<dyn AiProvider>> {
    vec![Box::new(anthropic::Anthropic)]
}

/// The provider registered under `id`.
pub fn resolve(id: &str) -> Result<Box<dyn AiProvider>, AiError> {
    match id {
        anthropic::ID => Ok(Box::new(anthropic::Anthropic)),
        other => Err(AiError::UnknownProvider {
            provider: other.to_string(),
        }),
    }
}

/// The provider used when configuration names none.
pub const DEFAULT_PROVIDER: &str = anthropic::ID;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_provider_resolves() {
        assert!(resolve(DEFAULT_PROVIDER).is_ok());
    }

    #[test]
    fn an_unknown_provider_is_named_in_the_error_rather_than_silently_defaulted() {
        // Falling back to the default would send the user's code to a vendor they did
        // not choose.
        // `Box<dyn AiProvider>` is not `Debug`, so the success arm cannot be unwrapped
        // for its error — match instead of asserting through `unwrap_err`.
        let Err(error) = resolve("not-a-provider") else {
            panic!("an unknown provider must not resolve");
        };
        assert!(matches!(error, AiError::UnknownProvider { .. }));
        assert!(error.to_string().contains("not-a-provider"));
    }

    #[test]
    fn every_registered_provider_resolves_by_its_own_id() {
        for provider in all() {
            assert!(resolve(provider.id()).is_ok(), "{}", provider.id());
        }
    }

    #[test]
    fn the_providers_here_are_exactly_the_ones_config_will_accept() {
        // The ids are duplicated on purpose: `Config` must normalize the stored value
        // and cannot depend on this crate (the dependency points `ai-api → core`). A
        // split like that is only safe while something fails when it drifts — a setting
        // naming a provider this build does not implement would resolve to nothing.
        //
        // The same guard `codepack-ai::handoff` keeps over the local-agent list, for the
        // same reason and in the same shape.
        let here: Vec<&str> = all().iter().map(|provider| provider.id()).collect();
        assert_eq!(
            here,
            codepack_core::config::AI_API_PROVIDERS.to_vec(),
            "providers here and Config's AI_API_PROVIDERS have drifted"
        );
        assert!(resolve(codepack_core::config::DEFAULT_AI_API_PROVIDER).is_ok());
    }
}
