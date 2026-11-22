use super::*;
use std::{net::TcpListener, thread};

fn initialize() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}
    }})
}

fn bridge(url: &str) -> Bridge {
    Bridge::new(Config::new(url, Duration::from_secs(2)).unwrap()).unwrap()
}

fn exchange(bridge: &mut Bridge, messages: &[Value]) -> Vec<Value> {
    let input = messages
        .iter()
        .map(|value| format!("{value}\n"))
        .collect::<String>();
    let mut output = Vec::new();
    bridge.serve(input.as_bytes(), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn http(status: &str, body: &str, extra: &str) -> (String, thread::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
    let status = status.to_owned();
    let body = body.to_owned();
    let extra = extra.to_owned();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(
                        start.elapsed() < Duration::from_secs(4),
                        "bridge made no HTTP request"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept: {error}"),
            }
        };
        // Accepted sockets inherit nonblocking mode on Windows. The fixture
        // uses blocking reads with a timeout after the bounded accept loop.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = io::BufReader::new(&stream);
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            assert!(!line.is_empty());
            headers.push_str(&line.to_ascii_lowercase());
        }
        assert!(headers.starts_with("post /mcp http/1.1"));
        assert!(headers.contains("accept: application/json, text/event-stream"));
        assert!(headers.contains("mcp-protocol-version: 2025-11-25"));
        assert!(!headers.contains("origin:"));
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let mut input = vec![0; length];
        reader.read_exact(&mut input).unwrap();
        let request = serde_json::from_slice(&input).unwrap();
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}", body.len()).unwrap();
        request
    });
    (endpoint, handle)
}

#[test]
fn endpoints_are_restricted_to_loopback_and_mcp_path() {
    for endpoint in ["http://127.0.0.1:4748/mcp", "http://[::1]:4748/mcp"] {
        assert!(
            Config::new(endpoint, Duration::from_secs(1)).is_ok(),
            "{endpoint}"
        );
    }
    for endpoint in [
        "https://127.0.0.1:4748/mcp",
        "http://example.com:4748/mcp",
        "http://localhost:4748/mcp",
        "http://192.168.1.1:4748/mcp",
        "http://127.0.0.1:4748/",
        "http://user:pass@127.0.0.1:4748/mcp",
        "http://127.0.0.1:4748/mcp?key=value",
        "http://127.0.0.1:4748/mcp#fragment",
        "http://127.0.0.1:0/mcp",
    ] {
        assert!(
            Config::new(endpoint, Duration::from_secs(1)).is_err(),
            "{endpoint}"
        );
    }
}

#[test]
fn registry_metadata_matches_the_package_and_visible_ownership_marker() {
    let metadata: Value = serde_json::from_str(include_str!("../../server.json")).unwrap();
    assert_eq!(metadata["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        metadata["packages"][0]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        metadata["packages"][0]["identifier"],
        env!("CARGO_PKG_NAME")
    );
    assert_eq!(metadata["packages"][0]["transport"]["type"], "stdio");
    let marker = format!("mcp-name: {}", metadata["name"].as_str().unwrap());
    assert!(
        include_str!("../../README.md")
            .lines()
            .any(|line| line.starts_with("- MCP Registry name:") && line.contains(&marker))
    );
}

#[test]
fn local_handshake_and_ping_work_without_a_running_app_and_notifications_are_silent() {
    let mut bridge = bridge("http://127.0.0.1:1/mcp");
    let output = exchange(
        &mut bridge,
        &[
            initialize(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":"ping-id","method":"ping"}),
        ],
    );
    assert_eq!(output.len(), 2);
    assert_eq!(output[0]["result"]["serverInfo"]["name"], "gpui-agentation");
    assert_eq!(output[1]["id"], "ping-id");
    assert_eq!(output[1]["result"], json!({}));
}

#[test]
fn http_relay_preserves_tool_arguments_ids_and_results() {
    let expected = json!({"jsonrpc":"2.0","id":"reply","result":{"content":[{"type":"text","text":"saved"}],"isError":false}});
    let (endpoint, server) = http("200 OK", &expected.to_string(), "");
    let request = json!({"jsonrpc":"2.0","id":"reply","method":"tools/call","params":{"name":"annotation_reply","arguments":{"id":"note-1","message":"Checking spacing"}}});
    let output = exchange(&mut bridge(&endpoint), &[initialize(), request.clone()]);
    assert_eq!(server.join().unwrap(), request);
    assert_eq!(output[1], expected);
}

#[test]
fn unavailable_app_returns_an_actionable_error_without_ending_stdio_session() {
    let mut bridge = bridge("http://127.0.0.1:1/mcp");
    let output = exchange(
        &mut bridge,
        &[
            initialize(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":3,"method":"ping"}),
        ],
    );
    assert_eq!(output[1]["error"]["code"], -32001);
    assert!(
        output[1]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("start its debug build")
    );
    assert_eq!(output[2]["result"], json!({}));
}

#[test]
fn redirects_and_mismatched_response_ids_are_rejected() {
    for (status, body, headers) in [
        ("302 Found", "{}", "Location: http://example.com/mcp\r\n"),
        ("200 OK", r#"{"jsonrpc":"2.0","id":99,"result":{}}"#, ""),
        ("503 Unavailable", "{}", ""),
    ] {
        let (endpoint, server) = http(status, body, headers);
        let output = exchange(
            &mut bridge(&endpoint),
            &[
                initialize(),
                json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            ],
        );
        server.join().unwrap();
        assert_eq!(output[1]["id"], 2);
        assert_eq!(output[1]["error"]["code"], -32001);
    }
}

#[test]
fn framing_rejects_oversize_input_and_recovers_after_parse_errors() {
    let mut bridge = bridge("http://127.0.0.1:1/mcp");
    let mut output = Vec::new();
    bridge
        .serve(
            b"invalid\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n".as_slice(),
            &mut output,
        )
        .unwrap();
    let frames: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames[0]["error"]["code"], -32700);
    assert_eq!(frames[1]["id"], 7);
    assert!(
        bridge
            .serve(vec![b'x'; MAX_REQUEST + 3].as_slice(), Vec::new())
            .is_err()
    );
}

#[cfg(all(debug_assertions, feature = "native"))]
#[test]
fn bridge_discovers_and_reads_the_native_server() {
    use gpui_agentation::{Server, Store};

    let store = Store::memory();
    let session = store
        .ensure_session("Bridge smoke test", "gpui://bridge-test")
        .unwrap();
    let server = Server::start(store, 0).unwrap();
    let mut bridge = bridge(&format!("http://{}/mcp", server.address()));
    let frames = exchange(
        &mut bridge,
        &[
            initialize(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"annotation_sessions"}}),
        ],
    );
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[1]["id"], 2);
    assert_eq!(frames[1]["result"]["tools"].as_array().unwrap().len(), 6);
    assert_eq!(frames[2]["id"], 3);
    let sessions: Value =
        serde_json::from_str(frames[2]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(sessions[0]["id"], session.id);
}
