//! Errors this crate raises.
//!
//! Every variant is safe to show a user and safe to write to a log: none of them can
//! carry an API key, and the transport variant deliberately does not embed the response
//! body, because a provider's error payload can echo request content back.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("no API key is stored for provider {provider:?}")]
    NoKey { provider: String },

    #[error("the OS credential store refused the operation: {message}")]
    KeyStore { message: String },

    #[error("provider {provider:?} is not supported")]
    UnknownProvider { provider: String },

    /// The send was refused before any network call. Carrying the reason as a typed
    /// value rather than a string keeps the UI able to explain *which* guard fired.
    #[error("refused to send: {0}")]
    Refused(#[from] Refusal),

    #[error("could not read the bundle at {path}: {source}")]
    Bundle {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A transport or protocol failure. The provider's own response body is **not**
    /// included: an error payload frequently quotes the request back, and the request
    /// is the user's source code.
    #[error("{provider} request failed: {kind}")]
    Transport { provider: String, kind: String },

    #[error("{provider} returned HTTP {status}")]
    Status { provider: String, status: u16 },

    #[error("{provider} returned a response this client could not parse")]
    Malformed { provider: String },
}

/// Why a send was refused. These are the guards that run *before* anything leaves the
/// machine, so each one is a promise the product makes rather than an error condition.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Refusal {
    #[error(
        "the bundle contains {count} critical security finding(s). Re-export with a \
         stricter safety mode, or send anyway only if you have checked each one."
    )]
    CriticalFindings { count: u64 },

    #[error("the AI integration is switched off in settings")]
    Disabled,

    #[error("the bundle has no AI context to send")]
    EmptyContext,
}
