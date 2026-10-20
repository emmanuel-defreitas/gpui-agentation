#![cfg(all(debug_assertions, feature = "native", feature = "mcp"))]

use gpui_agentation::{Server, Store};
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn published_bridge_discovers_and_reads_the_native_server() {
    let store = Store::memory();
    let session = store
        .ensure_session("Bridge smoke test", "gpui://bridge-test")
        .unwrap();
    let server = Server::start(store, 0).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_gpui-agentation"))
        .args(["--url", &format!("http://{}/mcp", server.address())])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"e2e","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"annotation_sessions"}}),
        ] {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frames: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[1]["id"], 2);
    assert_eq!(frames[1]["result"]["tools"].as_array().unwrap().len(), 6);
    assert_eq!(frames[2]["id"], 3);
    let sessions: Value =
        serde_json::from_str(frames[2]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(sessions[0]["id"], session.id);
}
