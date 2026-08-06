//! JSON-RPC 2.0, as the Model Context Protocol's stdio transport frames it.
//!
//! ## The framing
//!
//! One JSON object per line, both directions. Not the header-delimited framing the
//! Language Server Protocol uses — MCP's stdio transport is newline-delimited, and a
//! message may therefore never contain a raw newline. `serde_json::to_string` never
//! emits one, so that holds by construction rather than by care.
//!
//! ## Notifications get silence
//!
//! A request carries an `id` and must be answered exactly once; a notification has no
//! `id` and must **not** be answered at all. Replying to one is a protocol violation
//! that some clients treat as a fatal error, which is why [`Request::is_notification`]
//! exists rather than the loop guessing from the method name.
//!
//! ## `id` is not a number
//!
//! The specification allows a string or a number, and clients use both. It is carried
//! as an opaque [`serde_json::Value`] and echoed back untouched; parsing it into an
//! integer would break every client that numbers its requests with strings.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The MCP revision this build implements.
///
/// Sent back from `initialize`. A client asking for a different revision still gets
/// this one, which is what the specification prescribes: the server answers with the
/// version it will actually speak, and the client decides whether it can live with it.
/// Claiming to speak the client's version would be the dishonest alternative.
pub(crate) const PROTOCOL_VERSION: &str = "2025-06-18";

pub(crate) const SERVER_NAME: &str = "codepack";

// Standard JSON-RPC error codes. Named rather than inlined because the numbers carry
// no meaning at their call sites.
pub(crate) const PARSE_ERROR: i32 = -32700;
pub(crate) const INVALID_REQUEST: i32 = -32600;
pub(crate) const METHOD_NOT_FOUND: i32 = -32601;
pub(crate) const INVALID_PARAMS: i32 = -32602;

#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    #[serde(default)]
    pub jsonrpc: String,
    /// Absent for a notification.
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    pub(crate) fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Whether the envelope declares the version this transport speaks.
    ///
    /// Checked rather than assumed: a client that sends `"jsonrpc": "1.0"` is speaking
    /// a different protocol, and answering it as though it were 2.0 produces confusion
    /// further downstream instead of an error here.
    pub(crate) fn is_valid_envelope(&self) -> bool {
        self.jsonrpc == "2.0"
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseError {
    pub code: i32,
    pub message: String,
}

impl Response {
    pub(crate) fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn failure(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message: message.into(),
            }),
        }
    }

    /// One line, terminated — the unit this transport writes.
    pub(crate) fn to_line(&self) -> String {
        // A response that cannot be serialized would leave the client waiting forever,
        // so the fallback is a valid error object rather than a dropped message.
        match serde_json::to_string(self) {
            Ok(text) => format!("{text}\n"),
            Err(_) => format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{{\"code\":{},\"message\":\"the \
                 server could not serialize its own response\"}}}}\n",
                INVALID_REQUEST
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_without_an_id_is_a_notification() {
        let request: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .unwrap();
        assert!(request.is_notification());
        assert!(request.is_valid_envelope());
    }

    #[test]
    fn an_id_may_be_a_string_or_a_number_and_survives_untouched() {
        // Clients use both. Parsing either into an integer would break the other.
        for raw in [
            r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":"req-7","method":"ping"}"#,
        ] {
            let request: Request = serde_json::from_str(raw).unwrap();
            assert!(!request.is_notification());
            let echoed = Response::success(request.id.clone().unwrap(), Value::Null);
            let line = echoed.to_line();
            assert!(
                line.contains("\"id\":7") || line.contains("\"id\":\"req-7\""),
                "{line}"
            );
        }
    }

    #[test]
    fn a_wrong_protocol_version_in_the_envelope_is_rejected() {
        let request: Request =
            serde_json::from_str(r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#).unwrap();
        assert!(!request.is_valid_envelope());
    }

    #[test]
    fn a_response_line_is_one_line_and_ends_with_a_newline() {
        // The whole framing rests on this: an embedded newline would split one message
        // into two unparseable halves.
        let response =
            Response::success(Value::from(1), serde_json::json!({"text": "first\nsecond"}));
        let line = response.to_line();
        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1, "{line}");
    }

    #[test]
    fn an_error_response_carries_no_result_key_at_all() {
        // JSON-RPC forbids both being present, and a client checking for `result` by
        // key presence would otherwise read an error as a success.
        let line = Response::failure(Value::from(1), METHOD_NOT_FOUND, "nope").to_line();
        assert!(!line.contains("\"result\""), "{line}");
        assert!(line.contains("-32601"), "{line}");
    }
}
