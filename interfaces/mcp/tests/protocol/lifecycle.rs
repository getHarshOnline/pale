//! Connection negotiation through the production stdio executable.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Connection {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Connection {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pale-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.input, "{message}").unwrap();
        self.input.flush().unwrap();
    }

    fn request(&mut self, message: Value) -> Value {
        let id = message["id"].clone();
        self.send(message);
        let mut line = String::new();
        assert_ne!(self.output.read_line(&mut line).unwrap(), 0);
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], id);
        response
    }

    fn initialize(&mut self, version: &str) -> Value {
        self.request(json!({
            "jsonrpc":"2.0", "id":1, "method":"initialize",
            "params": {
                "protocolVersion":version, "capabilities":{},
                "clientInfo":{"name":"protocol-test", "version":"1"}
            }
        }))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn inventory(metadata: bool) -> Value {
    let body = json!({
        "name":"compute_path_inventory",
        "arguments":{"style":"posix_relative", "paths":["src/./lib.rs"]}
    });
    json!({"jsonrpc":"2.0", "id":3, "method":"tools/call",
        "params":if metadata { super::params(body) } else { body }})
}

#[test]
fn initialized_clients_discover_and_call_the_same_tool_as_stateless_clients() {
    let expected = super::request(inventory(true))["result"]["structuredContent"].clone();
    for version in ["2025-06-18", "2025-11-25", "2026-07-28"] {
        let mut connection = Connection::new();
        let initialized = connection.initialize(version);
        assert_eq!(initialized["result"]["protocolVersion"], version);
        assert_eq!(initialized["result"]["serverInfo"]["name"], "pale-mcp");
        assert!(initialized["result"]["capabilities"]["tools"].is_object());
        connection.send(json!({"jsonrpc":"2.0", "method":"notifications/initialized"}));
        let listed = connection.request(json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/list", "params":{}
        }));
        assert!(listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "compute_path_inventory"));
        let called = connection.request(inventory(false));
        assert_eq!(called["result"]["structuredContent"], expected);
        assert_ne!(called["result"]["isError"], true);
        for response in [&listed, &called] {
            assert_eq!(
                response["result"].get("resultType").is_some(),
                version == "2026-07-28"
            );
        }
    }
}

#[test]
fn initialization_fallback_uses_the_agreed_version_and_cannot_be_replaced() {
    let mut connection = Connection::new();
    assert_eq!(
        connection.initialize("2099-01-01")["result"]["protocolVersion"],
        "2026-07-28"
    );
    assert_eq!(connection.initialize("2025-06-18")["error"]["code"], -32600);
    let called = connection.request(inventory(false));
    assert_eq!(called["result"]["resultType"], "complete");
    assert_ne!(called["result"]["isError"], true);
}

#[test]
fn initialized_clients_cannot_hide_invalid_explicit_metadata() {
    let mut connection = Connection::new();
    assert!(connection.initialize("2025-11-25").get("error").is_none());
    for metadata in [
        json!({"io.modelcontextprotocol/protocolVersion":7}),
        json!({"io.modelcontextprotocol/clientCapabilities":{}}),
        json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28", "io.modelcontextprotocol/clientCapabilities":[]}),
    ] {
        let response = connection.request(json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/list", "params":{"_meta":metadata}
        }));
        assert_eq!(response["error"]["code"], -32602);
    }
    let mut unsupported = inventory(true);
    unsupported["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2099-01-01");
    assert_eq!(connection.request(unsupported)["error"]["code"], -32022);
    assert_eq!(
        connection.request(inventory(true))["result"]["resultType"],
        "complete"
    );
    assert!(connection.request(inventory(false))["result"]
        .get("resultType")
        .is_none());
}

#[test]
fn stateless_requests_and_preinit_ping_do_not_initialize_the_connection() {
    let mut connection = Connection::new();
    assert_eq!(
        connection.request(json!({"jsonrpc":"2.0", "id":0, "method":"ping"}))["result"],
        json!({})
    );
    assert_eq!(
        connection.request(inventory(false))["error"]["code"],
        -32602
    );
    assert!(connection.request(inventory(true)).get("error").is_none());
    assert_eq!(
        connection.request(inventory(false))["error"]["code"],
        -32602
    );
}
