//! `codepack mcp` — the third entry point, for an agent rather than a person.
//!
//! ## What it is for
//!
//! Until this existed, handing a project to an assistant was a manual gesture: a human
//! exported a bundle and passed it over. The assistant could read what it was given and
//! nothing else — so when a file was missing from the bundle it had no way to ask why,
//! and its only options were to guess or to assume the file does not exist.
//!
//! This closes that loop without a single network call. An agent already running on
//! this machine speaks JSON-RPC over a pipe and can ask the same four questions a
//! person asks at the terminal: what would an export include, is there a secret here,
//! why is this one file missing, produce the bundle.
//!
//! ## Why it is not a new crate
//!
//! It was scoped as "a thin crate over `codepack-engine`", and reading the code changed
//! that. `preview`, `scan` and `explain` are not engine calls: they are this binary's
//! four-layer configuration resolution, the deliberate forcing of safe mode to `full`
//! for scanning, budget handling, and the report shapes other people's pipelines
//! already consume. A separate crate would have had to restate all of it, and the two
//! would have drifted — the first symptom being an agent contradicting the CLI about
//! the same project.
//!
//! So the transport lives here and calls the same builders the commands call. The whole
//! module is protocol plumbing; it decides nothing about exporting.
//!
//! ## Invariant I1 is untouched
//!
//! stdio, not HTTP. No dependency was added — `serde_json` was already here — so no
//! manifest gained a network client and the gate's `network isolation` step reads
//! exactly what it read before. A tool call runs locally and writes only where the
//! command it wraps would write.
//!
//! ## stdout is the protocol
//!
//! Nothing but JSON-RPC may ever reach stdout, which is the rule `--json` already lives
//! by, applied to a stream a machine parses continuously rather than once. Diagnostics
//! go to stderr. This is why the export tool runs quiet.

mod protocol;
mod tools;

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use protocol::{
    INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, PROTOCOL_VERSION, Request,
    Response, SERVER_NAME,
};

use crate::error::Result;
use crate::exit::Outcome;

/// Serves until stdin closes.
///
/// Closing stdin is how an MCP client shuts a server down, so end-of-input is a normal
/// exit and not an error. A server that treated it as one would make every clean
/// disconnect look like a crash in the client's logs.
pub(crate) fn run() -> Result<Outcome> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    // A broken pipe is how a client that has stopped listening looks from here, and it
    // is a normal end of session rather than a failure to report to nobody.
    match serve(&mut stdin.lock(), &mut stdout) {
        Ok(()) => Ok(Outcome::Success),
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(Outcome::Success),
        Err(error) => Err(crate::error::CliError::message(format!(
            "the MCP session ended: {error}"
        ))),
    }
}

/// The read/dispatch/write loop, over any pair of streams so it can be driven by a test
/// without a process.
fn serve(input: &mut dyn BufRead, output: &mut dyn Write) -> std::io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        // Blank lines between messages are not a protocol error; ignoring them costs
        // nothing and refusing them would break a client that pads its output.
        if trimmed.is_empty() {
            continue;
        }

        if let Some(response) = handle_line(trimmed) {
            output.write_all(response.to_line().as_bytes())?;
            // Flushed per message: a client is blocked waiting for this answer, and a
            // buffered response is indistinguishable from a hung server.
            output.flush()?;
        }
    }
}

/// One incoming line to at most one outgoing response. `None` means silence is the
/// correct answer, which is the case for every notification.
fn handle_line(line: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => {
            // The id is unknown for unparseable input, and the specification says to
            // answer with a null id rather than to stay silent — otherwise a client
            // that mis-sent one message waits forever.
            return Some(Response::failure(
                Value::Null,
                PARSE_ERROR,
                format!("could not parse the message: {error}"),
            ));
        }
    };

    if !request.is_valid_envelope() {
        let id = request.id.clone().unwrap_or(Value::Null);
        return Some(Response::failure(
            id,
            INVALID_REQUEST,
            "this transport speaks JSON-RPC 2.0",
        ));
    }

    if request.is_notification() {
        // `notifications/initialized`, `notifications/cancelled` and anything else a
        // client announces. Nothing here is long-running enough to interrupt between
        // messages — a tool call blocks the loop — so cancellation is acknowledged by
        // being ignored rather than pretended to.
        return None;
    }

    let id = request.id.clone().unwrap_or(Value::Null);
    Some(match request.method.as_str() {
        "initialize" => Response::success(id, initialize_result()),
        "ping" => Response::success(id, json!({})),
        "tools/list" => Response::success(id, json!({"tools": tools::catalogue()})),
        "tools/call" => match call_result(request.params.as_ref()) {
            Ok(result) => Response::success(id, result),
            Err(message) => Response::failure(id, INVALID_PARAMS, message),
        },
        other => Response::failure(
            id,
            METHOD_NOT_FOUND,
            format!("this server does not implement {other:?}"),
        ),
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        // Only tools. No resources, no prompts, no sampling: declaring a capability
        // this build does not implement would have clients calling into nothing.
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Ask codepack_preview what an export would contain before \
                         asking for one, and codepack_explain when a file you expected \
                         is missing from a bundle. Everything runs locally; nothing is \
                         uploaded."
    })
}

/// Turns `tools/call` parameters into a result.
///
/// `Err` here is a **protocol** failure — the parameters were not shaped like a tool
/// call at all. A tool that ran and failed comes back as `Ok` carrying `isError`, so
/// the model sees the message and can correct itself.
fn call_result(params: Option<&Value>) -> std::result::Result<Value, String> {
    let params = params.ok_or("tools/call needs params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("tools/call needs a tool `name`")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let outcome = tools::call(name, &arguments);
    let mut result = json!({
        "content": [{"type": "text", "text": outcome.text}],
        "isError": outcome.is_error
    });
    if let Some(structured) = outcome.structured
        && let Some(object) = result.as_object_mut()
    {
        object.insert("structuredContent".to_string(), structured);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(input: &str) -> Vec<Value> {
        let mut output = Vec::new();
        serve(&mut input.as_bytes(), &mut output).unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("every line must be one JSON object"))
            .collect()
    }

    #[test]
    fn initialize_reports_the_version_the_tools_and_the_server() {
        let responses = drive("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n");

        assert_eq!(responses.len(), 1);
        let result = &responses[0]["result"];
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert!(result["capabilities"]["tools"].is_object());
        // Capabilities this build does not implement must not be advertised.
        assert!(result["capabilities"].get("resources").is_none());
        assert!(result["capabilities"].get("prompts").is_none());
    }

    #[test]
    fn a_notification_is_answered_with_silence() {
        // Answering one is a protocol violation some clients treat as fatal.
        let responses = drive(
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n",
        );
        assert_eq!(responses.len(), 1, "the notification drew a reply");
        assert_eq!(responses[0]["id"], 1);
    }

    #[test]
    fn every_response_carries_the_id_it_was_asked_with() {
        let responses = drive(
            "{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"ping\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        );
        assert_eq!(responses[0]["id"], "a");
        assert_eq!(responses[1]["id"], 2);
    }

    #[test]
    fn tools_list_offers_the_four_tools() {
        let responses = drive("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n");
        let names: Vec<&str> = responses[0]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "codepack_preview",
                "codepack_scan",
                "codepack_explain",
                "codepack_export"
            ]
        );
    }

    #[test]
    fn an_unknown_method_is_a_protocol_error_not_a_crash() {
        let responses = drive("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/list\"}\n");
        assert_eq!(responses[0]["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn unparseable_input_answers_with_a_null_id_rather_than_silence() {
        // Silence would leave the client waiting forever on a message it mis-sent.
        let responses = drive("this is not json\n");
        assert_eq!(responses[0]["error"]["code"], PARSE_ERROR);
        assert_eq!(responses[0]["id"], Value::Null);
    }

    #[test]
    fn a_blank_line_between_messages_is_not_an_error() {
        let responses = drive("\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n");
        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn a_call_without_a_tool_name_is_a_parameter_error() {
        let responses =
            drive("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{}}\n");
        assert_eq!(responses[0]["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn a_tool_that_fails_comes_back_as_a_result_the_model_can_read() {
        let responses = drive(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":\
             {\"name\":\"codepack_preview\",\"arguments\":{\"project\":\"/nowhere/at/all\"}}}\n",
        );
        assert!(responses[0]["error"].is_null(), "{}", responses[0]);
        assert_eq!(responses[0]["result"]["isError"], true);
        assert!(responses[0]["result"]["content"][0]["text"].is_string());
    }

    #[test]
    fn a_tool_that_succeeds_carries_both_text_and_structured_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "print(1)\n").unwrap();
        let line = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":\
             {{\"name\":\"codepack_explain\",\"arguments\":\
             {{\"project\":{project},\"file\":\"main.py\"}}}}}}\n",
            project = serde_json::to_string(&dir.path().display().to_string()).unwrap()
        );

        let responses = drive(&line);
        let result = &responses[0]["result"];
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(result["structuredContent"]["verdict"], "included");
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("included")
        );
    }

    #[test]
    fn a_session_survives_a_bad_message_in_the_middle_of_it() {
        // A client that mis-sends one message must not lose the connection: the
        // failure is per-message, not per-session.
        let responses = drive(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\
             {oops\n\
             {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n",
        );
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["id"], 1);
        assert_eq!(responses[1]["error"]["code"], PARSE_ERROR);
        assert_eq!(responses[2]["id"], 2);
    }

    #[test]
    fn closing_the_input_ends_the_session_cleanly() {
        // How an MCP client shuts a server down. Treating it as an error would make
        // every clean disconnect look like a crash.
        assert!(drive("").is_empty());
    }
}
