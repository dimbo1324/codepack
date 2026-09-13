//! Proves that this build can actually complete a TLS handshake with the provider.
//!
//! ## Why this test exists
//!
//! Because for seven weeks it did not, and nothing noticed. From 2026-07-27 until
//! 2026-09-13 this crate's `ureq` was configured with `native-tls-no-default`, chosen to
//! keep a CDLA-licensed Mozilla root list out of the dependency graph. That feature adds
//! the native-TLS *dependency* but `ureq` gates provider *selection* on
//! `cfg(feature = "native-tls")`, so no TLS provider was active at all and every HTTPS
//! request panicked:
//!
//! ```text
//! uri scheme is https, provider is Rustls but feature is not enabled: rustls
//! ```
//!
//! Everything else was tested. The plan, the guards, the key store, the response parsing,
//! the refusals — all covered, all passing, all meaningless, because the one step between
//! them never ran. The stage's own status line recorded the gap honestly ("живого запроса
//! к провайдеру так и не было") and the gap was the defect.
//!
//! ## Why it is `#[ignore]`d
//!
//! It opens a real connection to a real host, so it does not belong in a gate that must
//! pass on an offline machine. Run it explicitly:
//!
//! ```text
//! cargo test -p codepack-ai-api --test tls_handshake -- --ignored
//! ```
//!
//! ## Why it needs no API key
//!
//! It deliberately sends *no* credential. The provider answers `401`, and a `401` is
//! proof of exactly what is under test: DNS resolved, the socket opened, the certificate
//! chain validated against this machine's trust store, and the request reached the
//! application layer. Whether the key is any good is a different question, and not one a
//! test should need a real key to ask.

use std::time::Duration;

use ureq::tls::{Certificate, RootCerts, TlsConfig};

/// The same endpoint the client uses, so the certificate chain under test is the one that
/// matters rather than a stand-in.
const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// The same roots the client loads. Duplicated from `providers::anthropic` on purpose:
/// this test asserts that *this configuration* completes a handshake, so sharing the
/// function would let a change break the client and the test together and still pass.
fn platform_roots() -> RootCerts {
    let loaded = rustls_native_certs::load_native_certs();
    let certs: Vec<Certificate<'static>> = loaded
        .certs
        .iter()
        .map(|der| Certificate::from_der(der.as_ref()).to_owned())
        .collect();
    RootCerts::new_with_certs(&certs)
}

#[test]
#[ignore = "opens a real network connection; run explicitly or in the weekly job"]
fn the_configured_tls_stack_completes_a_handshake_with_the_provider() {
    let result = ureq::post(ENDPOINT)
        .config()
        .timeout_global(Some(Duration::from_secs(30)))
        .tls_config(TlsConfig::builder().root_certs(platform_roots()).build())
        .build()
        .send_json(serde_json::json!({}));

    match result {
        // Unauthorised, or a complaint about the empty body: either way the bytes
        // travelled over a validated TLS connection, which is the whole claim.
        Err(ureq::Error::StatusCode(status)) => {
            assert!(
                (400..500).contains(&status),
                "expected the provider to reject an unauthenticated request, got {status}"
            );
        }
        Ok(_) => panic!("an unauthenticated request must not succeed"),
        Err(error) => panic!(
            "the TLS handshake did not complete: {error}\n\n\
             This is the failure mode that went unnoticed from 2026-07-27 to 2026-09-13. \
             Check the `ureq` feature set in the root Cargo.toml — provider selection is \
             gated on a feature, and enabling the transport dependency alone is not \
             enough."
        ),
    }
}

/// The roots have to come from somewhere, and an empty list is a silent handshake
/// failure rather than a loud one.
///
/// Not `#[ignore]`d: reading the operating system's certificate store touches no network
/// and works on every machine the gate runs on, so this half can be checked always.
#[test]
fn the_operating_systems_trust_store_yields_certificates() {
    let loaded = rustls_native_certs::load_native_certs();
    assert!(
        !loaded.certs.is_empty(),
        "no root certificates were read from this machine's trust store; \
         every HTTPS request would fail to validate. Errors: {:?}",
        loaded.errors
    );
}
