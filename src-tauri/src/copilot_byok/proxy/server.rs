use super::{current_provider, load_store, Store};
use crate::error::AppError;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Json};
use axum::http::{header, HeaderMap, HeaderName, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use bytes::Bytes;
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const ROUTE: &str = "/copilot/v1/chat/completions";
const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;

static RUNTIME: Lazy<Mutex<Option<Runtime>>> = Lazy::new(|| Mutex::new(None));
static RUNNING: AtomicBool = AtomicBool::new(false);

struct Runtime {
    port: u16,
    shutdown: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

pub(super) fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

pub(super) fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}{ROUTE}")
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let body = json!({
        "error": {
            "message": message.into(),
            "type": "cc_switch_copilot_proxy_error"
        }
    });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from("proxy error")))
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    )
}

async fn handle(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
    let store = match load_store() {
        Ok(store) => store,
        Err(error) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };

    if bearer_token(&headers) != Some(store.gateway_token.as_str()) {
        return error_response(StatusCode::UNAUTHORIZED, "Invalid CC Switch gateway token");
    }

    let Some(provider) = current_provider(&store) else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "No enabled Copilot proxy provider is selected",
        );
    };

    let Some(object) = body.as_object_mut() else {
        return error_response(StatusCode::BAD_REQUEST, "Request body must be a JSON object");
    };
    object.insert("model".to_string(), Value::String(provider.model_id.clone()));

    let client = crate::proxy::http_client::get();
    let mut request = client
        .post(&provider.endpoint)
        .bearer_auth(&provider.api_key)
        .json(&body);
    for (name, value) in &provider.request_headers {
        request = request.header(name.as_str(), value.as_str());
    }

    let upstream = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("Upstream request failed: {error}"),
            )
        }
    };

    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let stream = upstream
        .bytes_stream()
        .map(|item| item.map(Bytes::from).map_err(std::io::Error::other));
    let mut response = Response::builder().status(status);
    for (name, value) in &upstream_headers {
        if !is_hop_by_hop(name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(stream))
        .unwrap_or_else(|error| error_response(StatusCode::BAD_GATEWAY, error.to_string()))
}

pub(super) async fn stop() -> Result<(), AppError> {
    let runtime = {
        let mut slot = RUNTIME
            .lock()
            .map_err(|error| AppError::Lock(error.to_string()))?;
        slot.take()
    };

    if let Some(Runtime {
        shutdown,
        mut handle,
        ..
    }) = runtime
    {
        let _ = shutdown.send(());
        if tokio::time::timeout(std::time::Duration::from_secs(3), &mut handle)
            .await
            .is_err()
        {
            handle.abort();
            let _ = handle.await;
        }
    }
    RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

pub(super) async fn ensure(port: u16) -> Result<(), AppError> {
    let already_running = RUNTIME
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))?
        .as_ref()
        .is_some_and(|runtime| runtime.port == port && is_running());
    if already_running {
        return Ok(());
    }

    stop().await?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| {
            AppError::Config(format!(
                "Failed to bind Copilot proxy on 127.0.0.1:{port}: {error}"
            ))
        })?;
    let (shutdown, shutdown_rx) = oneshot::channel();
    let app = Router::new()
        .route(ROUTE, post(handle))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES));

    RUNNING.store(true, Ordering::SeqCst);
    let handle = tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        RUNNING.store(false, Ordering::SeqCst);
        if let Err(error) = result {
            log::error!("Copilot BYOK proxy server stopped unexpectedly: {error}");
        }
    });

    let mut slot = RUNTIME
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))?;
    *slot = Some(Runtime {
        port,
        shutdown,
        handle,
    });
    Ok(())
}

pub(super) async fn activate(store: &Store) -> Result<(), AppError> {
    if current_provider(store).is_none() {
        return Err(AppError::InvalidInput(
            "Select an enabled Copilot proxy provider first".to_string(),
        ));
    }
    ensure(store.listen_port).await
}
