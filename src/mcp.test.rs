use super::*;

async fn rpc(client: &reqwest::Client, url: &str, method: &str, params: Value) -> Value {
    client
        .post(url)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn mcp_round_trip_shares_rest_state_and_preserves_security_guards() {
    let store = Store::memory();
    let session = store.ensure_session("MCP test", "gpui://test").unwrap();
    let note = store
        .add(
            &session.id,
            crate::AnnotationInput {
                comment: "Test MCP".into(),
                element_path: "button".into(),
                element: "Button".into(),
                x: 50.,
                y: 10.,
                bounding_box: crate::Rect::default(),
                source_file: None,
                gpui_configuration: Some(crate::GpuiConfiguration {
                    source: "src/button.rs:42:9".into(),
                    element_path: "button.primary".into(),
                    changes: vec![crate::GpuiStyleChange {
                        property: "Padding".into(),
                        before: "8px".into(),
                        after: "12px".into(),
                    }],
                    rust: "div()\n  .p(px(12.0))".into(),
                }),
                shape: None,
            },
        )
        .unwrap();
    let server = crate::Server::start(store.clone(), 0).unwrap();
    let url = format!("http://{}/mcp", server.address());
    let client = reqwest::Client::new();
    let init=rpc(&client,&url,"initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}})).await;
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let accepted = client
        .post(&url)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert!(accepted.bytes().await.unwrap().is_empty());
    let list = rpc(&client, &url, "tools/list", json!({})).await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 6);
    let pending = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_pending"}),
    )
    .await;
    let pending_text = pending["result"]["content"][0]["text"].as_str().unwrap();
    assert!(pending_text.contains(&note.id));
    assert!(pending_text.contains("gpuiConfiguration"));
    assert!(pending_text.contains(".p(px(12.0))"));
    let updated = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_update","arguments":{"id":note.id,"status":"acknowledged"}}),
    )
    .await;
    assert_eq!(updated["result"]["isError"], false);
    let replied = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_reply","arguments":{"id":note.id,"message":"Working on it"}}),
    )
    .await;
    assert_eq!(replied["result"]["isError"], false);
    assert_eq!(
        store.annotation(&note.id).unwrap().thread[0].message,
        "Working on it"
    );
    let markdown = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_markdown","arguments":{"sessionId":session.id}}),
    )
    .await;
    assert!(
        markdown["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Working on it")
    );
    assert!(
        markdown["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("GPUI configuration")
    );
    let invalid = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_update","arguments":{"id":note.id,"status":"invented"}}),
    )
    .await;
    assert_eq!(invalid["error"]["code"], -32602);
    let missing = rpc(
        &client,
        &url,
        "tools/call",
        json!({"name":"annotation_get","arguments":{"id":"missing"}}),
    )
    .await;
    assert_eq!(missing["result"]["isError"], true);
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    for (name, value) in [("origin", "https://evil.example"), ("host", "evil.example")] {
        assert_eq!(
            client
                .post(&url)
                .header(name, value)
                .json(&json!({}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        client
            .post(&url)
            .header("content-type", "application/json")
            .body("x".repeat(33 * 1024))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn mcp_rejects_invalid_transport_and_rpc_messages() {
    let server = crate::Server::start(Store::memory(), 0).unwrap();
    let url = format!("http://{}/mcp", server.address());
    let client = reqwest::Client::new();
    for (body, code) in [
        ("{", -32700),
        ("[]", -32600),
        (r#"{"jsonrpc":"2.0","method":"ping","id":null}"#, -32600),
    ] {
        let response: Value = client
            .post(&url)
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response["error"]["code"], code);
    }
    assert_eq!(
        client
            .post(&url)
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_ACCEPTABLE
    );
    assert_eq!(
        client
            .post(&url)
            .header("mcp-protocol-version", "unknown")
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        rpc(&client, &url, "unknown", json!({})).await["error"]["code"],
        -32601
    );
}

#[tokio::test]
async fn pausing_mcp_should_block_new_calls_but_preserve_rest_and_resume_on_the_same_port() {
    let store = Store::memory();
    let session = store
        .ensure_session("Toggle test", "gpui://toggle")
        .unwrap();
    let server = crate::Server::start(store, 0).unwrap();
    let address = server.address();
    let url = format!("http://{address}/mcp");
    let client = reqwest::Client::new();
    assert!(server.is_mcp_enabled());
    assert!(
        rpc(&client, &url, "ping", json!({}))
            .await
            .get("result")
            .is_some()
    );
    server.set_mcp_enabled(false);
    assert!(!server.is_mcp_enabled());
    let paused = client
        .post(&url)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}))
        .send()
        .await
        .unwrap();
    assert_eq!(paused.status(), StatusCode::SERVICE_UNAVAILABLE);
    // Existing browser-origin protection stays in front of the pause gate.
    assert_eq!(
        client
            .post(&url)
            .header("origin", "https://evil.example")
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let sessions: Value = client
        .get(format!("http://{address}/sessions"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sessions[0]["id"], session.id);
    server.set_mcp_enabled(true);
    assert!(server.is_mcp_enabled());
    assert_eq!(server.address(), address);
    let tools = rpc(&client, &url, "tools/list", json!({})).await;
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 6);
}
