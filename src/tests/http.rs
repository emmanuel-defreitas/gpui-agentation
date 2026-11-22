use super::*;

#[tokio::test]
async fn api_should_support_the_human_agent_feedback_round_trip() {
    let store = Store::memory();
    let server = Server::start(store, 0).unwrap();
    let base = format!("http://{}", server.address());
    let client = reqwest::Client::new();
    let session: Value = client
        .post(format!("{base}/sessions"))
        .json(&json!({"appName": "Test", "url": "gpui://test/explore"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session_id = session["id"].as_str().unwrap();
    let note: Value = client.post(format!("{base}/sessions/{session_id}/annotations")).json(&json!({"comment": "More padding", "elementPath": "titlebar", "element": "Title bar", "x": 50, "y": 10, "boundingBox": {"x": 0, "y": 0, "width": 800, "height": 54}, "sourceFile": "shell.rs:1:1"})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let id = note["id"].as_str().unwrap();
    let pending: Value = client
        .get(format!("{base}/pending"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pending["count"], 1);
    client
        .patch(format!("{base}/annotations/{id}"))
        .json(&json!({"status": "acknowledged", "comment": "Updated spacing request"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("{base}/annotations/{id}/thread"))
        .json(&json!({"role": "agent", "message": "Updated padding"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .patch(format!("{base}/annotations/{id}"))
        .json(&json!({"status": "resolved"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let session: Value = client
        .get(format!("{base}/sessions/{session_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        session["annotations"][0]["thread"][0]["message"],
        "Updated padding"
    );
    assert_eq!(session["annotations"][0]["status"], "resolved");
    let markdown = client
        .get(format!("{base}/sessions/{session_id}/markdown"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(markdown.contains("Updated spacing request"));
    assert_eq!(session["annotations"][0]["id"], note["id"]);
    assert_eq!(session["annotations"][0]["index"], note["index"]);
    client
        .delete(format!("{base}/annotations/{id}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(
        client
            .get(format!("{base}/annotations/{id}"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn events_should_replay_and_filter_by_session() {
    let store = Store::memory();
    let home = store.ensure_session("Test", "/").unwrap();
    store.ensure_session("Test", "/explore").unwrap();
    let server = Server::start(store, 0).unwrap();
    let mut response = reqwest::Client::new()
        .get(format!(
            "http://{}/sessions/{}/events",
            server.address(),
            home.id
        ))
        .header("Last-Event-ID", "0")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let chunk = tokio::time::timeout(Duration::from_secs(2), response.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let text = String::from_utf8(chunk.to_vec()).unwrap();
    assert!(text.contains("event: session.created"));
    assert!(text.contains(&home.id));
    assert!(!text.contains("/explore"));
}

#[tokio::test]
async fn server_should_reject_browser_origins_invalid_hosts_and_invalid_mutations() {
    let server = Server::start(Store::memory(), 0).unwrap();
    let base = format!("http://{}", server.address());
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{base}/sessions"))
            .header("Origin", "https://example.com")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(format!("{base}/sessions"))
            .header("Host", "example.com")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .patch(format!("{base}/annotations/missing"))
            .json(&json!({"status": "made-up"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        client
            .post(format!("{base}/sessions"))
            .json(&json!({"appName": "", "url": "/"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        client
            .post(format!("{base}/sessions"))
            .header("Content-Type", "application/json")
            .body(" ".repeat(40000))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn dropping_server_should_close_sse_connections_and_release_the_port() {
    let server = Server::start(Store::memory(), 0).unwrap();
    let address = server.address();
    let _stream = reqwest::get(format!("http://{address}/events"))
        .await
        .unwrap();
    drop(server);
    let server = Server::start(Store::memory(), address.port()).unwrap();
    assert_eq!(server.address(), address);
}
