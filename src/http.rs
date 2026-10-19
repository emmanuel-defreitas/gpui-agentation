use std::{
    convert::Infallible,
    future::IntoFuture,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    thread::JoinHandle,
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event as SseEvent, KeepAlive},
    },
    routing::{get, post},
};
use futures_util::stream;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::{AnnotationInput, AnnotationPatch, Error, Store};

/// Owns a loopback-only HTTP server. Dropping it closes connections and joins its thread.
pub struct Server {
    address: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Bind 127.0.0.1, using an OS-assigned port when `port` is zero.
    pub fn start(store: Store, port: u16) -> Result<Self, Error> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (stop, stopped) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("gpui-agentation".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                        return;
                    };
                    tokio::select! {
                        _ = axum::serve(listener, router(store)).into_future() => {},
                        _ = stopped => {},
                    }
                });
                runtime.shutdown_timeout(Duration::from_secs(1));
            })?;
        Ok(Self {
            address,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Actual listener address, including the assigned port.
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn router(store: Store) -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { Json(json!({"ok": true, "name": "gpui-agentation"})) }),
        )
        .route("/status", get(status))
        .route("/sessions", get(sessions).post(create_session))
        .route("/sessions/:id", get(session))
        .route("/sessions/:id/annotations", post(add))
        .route("/sessions/:id/pending", get(session_pending))
        .route("/sessions/:id/markdown", get(markdown))
        .route("/sessions/:id/events", get(session_events))
        .route(
            "/annotations/:id",
            get(annotation).patch(update).delete(delete),
        )
        .route("/annotations/:id/thread", post(reply))
        .route("/pending", get(pending))
        .route("/events", get(events))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn(local_clients))
        .with_state(store)
}

async fn local_clients(request: axum::extract::Request, next: Next) -> Response {
    let host = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    if request.headers().contains_key("origin") || !matches!(host, "127.0.0.1" | "localhost") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "native local clients only"})),
        )
            .into_response();
    }
    next.run(request).await
}

struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            Error::NotFound => (StatusCode::NOT_FOUND, "feedback object not found".into()),
            Error::Invalid(message) => (StatusCode::BAD_REQUEST, message),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "feedback storage unavailable".into(),
            ),
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}

async fn run<T: serde::Serialize + Send + 'static>(
    operation: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<Json<T>, ApiError> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|_| ApiError(Error::Poisoned))?
        .map(Json)
        .map_err(Into::into)
}

async fn status(State(store): State<Store>) -> Result<Json<Value>, ApiError> {
    run(move || Ok(json!({"name": "gpui-agentation", "version": 1, "revision": store.revision()?, "sessions": store.sessions()?.len()}))).await
}
async fn sessions(State(store): State<Store>) -> Result<Json<Vec<crate::Session>>, ApiError> {
    run(move || store.sessions()).await
}
async fn session(
    State(store): State<Store>,
    Path(id): Path<String>,
) -> Result<Json<crate::Session>, ApiError> {
    run(move || store.session(&id)).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NewSession {
    app_name: String,
    url: String,
}
async fn create_session(
    State(store): State<Store>,
    Json(input): Json<NewSession>,
) -> Result<Json<crate::Session>, ApiError> {
    run(move || store.ensure_session(&input.app_name, &input.url)).await
}
async fn add(
    State(store): State<Store>,
    Path(id): Path<String>,
    Json(input): Json<AnnotationInput>,
) -> Result<Json<crate::Annotation>, ApiError> {
    run(move || store.add(&id, input)).await
}
async fn annotation(
    State(store): State<Store>,
    Path(id): Path<String>,
) -> Result<Json<crate::Annotation>, ApiError> {
    run(move || store.annotation(&id)).await
}
async fn update(
    State(store): State<Store>,
    Path(id): Path<String>,
    Json(patch): Json<AnnotationPatch>,
) -> Result<Json<crate::Annotation>, ApiError> {
    run(move || store.update(&id, patch)).await
}
async fn delete(
    State(store): State<Store>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    run(move || {
        store.delete(&id)?;
        Ok(json!({"deleted": id}))
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    role: String,
    message: String,
}
async fn reply(
    State(store): State<Store>,
    Path(id): Path<String>,
    Json(input): Json<Reply>,
) -> Result<Json<crate::Annotation>, ApiError> {
    run(move || store.reply(&id, &input.role, &input.message)).await
}
async fn pending(State(store): State<Store>) -> Result<Json<Value>, ApiError> {
    run(move || {
        let notes = store.pending(None)?;
        Ok(json!({"count": notes.len(), "annotations": notes}))
    })
    .await
}
async fn session_pending(
    State(store): State<Store>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    run(move || {
        let notes = store.pending(Some(&id))?;
        Ok(json!({"count": notes.len(), "annotations": notes}))
    })
    .await
}
async fn markdown(State(store): State<Store>, Path(id): Path<String>) -> Result<String, ApiError> {
    Ok(store.markdown(&id)?)
}

async fn events(State(store): State<Store>, headers: HeaderMap) -> Result<Response, ApiError> {
    event_stream(store, None, headers)
}
async fn session_events(
    State(store): State<Store>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    store.session(&id)?;
    event_stream(store, Some(id), headers)
}

fn event_stream(
    store: Store,
    session: Option<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    // Subscribe before reading history; the cursor removes overlap without losing an event.
    let receiver = store.events.subscribe();
    let cursor = match headers.get("last-event-id") {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| Error::Invalid("invalid Last-Event-ID".into()))?,
        None => store.revision()?,
    };
    let history = store.replay(cursor)?;
    let state = (
        std::collections::VecDeque::from(history),
        receiver,
        cursor,
        session,
    );
    let stream = stream::unfold(
        state,
        |(mut history, mut receiver, mut cursor, session)| async move {
            loop {
                let event = match history.pop_front() {
                    Some(event) => event,
                    None => match receiver.recv().await {
                        Ok(event) => event,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let event = SseEvent::default()
                                .event("reset")
                                .data("reload sessions and reconnect without Last-Event-ID");
                            return Some((
                                Ok::<_, Infallible>(event),
                                (history, receiver, cursor, session),
                            ));
                        }
                        Err(_) => return None,
                    },
                };
                if event.id <= cursor {
                    continue;
                }
                cursor = event.id;
                if session.as_ref().is_some_and(|id| event.session_id != *id) {
                    continue;
                }
                let Ok(json) = serde_json::to_string(&event) else {
                    continue;
                };
                let output = SseEvent::default()
                    .id(event.id.to_string())
                    .event(event.kind)
                    .data(json);
                return Some((
                    Ok::<_, Infallible>(output),
                    (history, receiver, cursor, session),
                ));
            }
        },
    );
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

#[cfg(test)]
#[path = "http.test.rs"]
mod tests;
