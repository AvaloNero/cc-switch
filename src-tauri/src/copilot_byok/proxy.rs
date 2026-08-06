use super::vscode::{self, VsCodeEdition, VsCodeProfileTarget};
use crate::error::AppError;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Json};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use bytes::Bytes;
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const STORE_FILE: &str = "copilot-byok-proxy.json";
const STORE_VERSION: u32 = 1;
const GROUP_NAME: &str = "CC Switch Proxy";
const MODEL_ID: &str = "cc-switch-current";
const DEFAULT_LISTEN_PORT: u16 = 15_735;
const DEFAULT_CONTEXT_WINDOW: u64 = 262_144;
const DEFAULT_MAX_OUTPUT_TOKENS: u64 = 32_768;
const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;

static OPERATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SERVER_RUNTIME: Lazy<Mutex<Option<ServerRuntime>>> = Lazy::new(|| Mutex::new(None));
static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

struct ServerRuntime {
    port: u16,
    shutdown: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

fn default_true() -> bool {
    true
}

fn default_context_window() -> u64 {
    DEFAULT_CONTEXT_WINDOW
}

fn default_max_output_tokens() -> u64 {
    DEFAULT_MAX_OUTPUT_TOKENS
}

fn default_listen_port() -> u16 {
    DEFAULT_LISTEN_PORT
}

fn store_version() -> u32 {
    STORE_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokModel {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: String,
    pub model_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default = "default_true")]
    pub tool_calling: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default = "default_true")]
    pub thinking: bool,
    #[serde(default = "default_true")]
    pub streaming: bool,
}

impl CopilotByokModel {
    fn normalize(&mut self) {
        self.id = self.id.trim().to_string();
        if self.id.is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
        self.name = self.name.trim().to_string();
        self.endpoint = self.endpoint.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.model_id = self.model_id.trim().to_string();
        self.request_headers = self
            .request_headers
            .iter()
            .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
            .filter(|(name, _)| !name.is_empty())
            .collect();
        if self.context_window == 0 {
            self.context_window = DEFAULT_CONTEXT_WINDOW;
        }
        if self.max_output_tokens == 0 {
            self.max_output_tokens = DEFAULT_MAX_OUTPUT_TOKENS;
        }
    }

    fn validate(&self) -> Result<(), AppError> {
        if self.name.is_empty() {
            return Err(AppError::InvalidInput(
                "Copilot proxy provider name is required".to_string(),
            ));
        }
        if self.api_key.is_empty() {
            return Err(AppError::InvalidInput(
                "Copilot proxy provider API key is required".to_string(),
            ));
        }
        if self.model_id.is_empty() {
            return Err(AppError::InvalidInput(
                "Copilot proxy upstream model id is required".to_string(),
            ));
        }

        let url = url::Url::parse(&self.endpoint).map_err(|error| {
            AppError::InvalidInput(format!("Invalid Copilot proxy endpoint: {error}"))
        })?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::InvalidInput(
                "Copilot proxy endpoint must be an absolute HTTP or HTTPS URL".to_string(),
            ));
        }

        for (name, value) in &self.request_headers {
            HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
                AppError::InvalidInput(format!("Invalid request header name {name}: {error}"))
            })?;
            HeaderValue::from_str(value).map_err(|error| {
                AppError::InvalidInput(format!("Invalid request header value for {name}: {error}"))
            })?;

            let lower_name = name.to_ascii_lowercase();
            if matches!(
                lower_name.as_str(),
                "authorization"
                    | "host"
                    | "content-length"
                    | "transfer-encoding"
                    | "connection"
                    | "proxy-authorization"
            ) {
                return Err(AppError::InvalidInput(format!(
                    "Request header {name} is managed by CC Switch and cannot be overridden"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomTarget {
    id: String,
    name: String,
    language_models_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Store {
    #[serde(default = "store_version")]
    version: u32,
    #[serde(default)]
    gateway_token: String,
    #[serde(default = "default_listen_port")]
    listen_port: u16,
    #[serde(default)]
    integration_enabled: bool,
    #[serde(default)]
    current_provider_id: Option<String>,
    #[serde(default)]
    providers: Vec<CopilotByokModel>,
    #[serde(default)]
    selected_target_ids: Vec<String>,
    #[serde(default)]
    targets_initialized: bool,
    #[serde(default)]
    custom_targets: Vec<CustomTarget>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            gateway_token: uuid::Uuid::new_v4().to_string(),
            listen_port: DEFAULT_LISTEN_PORT,
            integration_enabled: false,
            current_provider_id: None,
            providers: Vec::new(),
            selected_target_ids: Vec::new(),
            targets_initialized: false,
            custom_targets: Vec::new(),
        }
    }
}

impl Store {
    fn normalize(&mut self) -> Result<(), AppError> {
        self.version = STORE_VERSION;
        if self.gateway_token.trim().is_empty() {
            self.gateway_token = uuid::Uuid::new_v4().to_string();
        }
        if self.listen_port == 0 {
            self.listen_port = DEFAULT_LISTEN_PORT;
        }

        let mut provider_ids = HashSet::new();
        for provider in &mut self.providers {
            provider.normalize();
            provider.validate()?;
            if !provider_ids.insert(provider.id.clone()) {
                return Err(AppError::Config(format!(
                    "Duplicate Copilot proxy provider id: {}",
                    provider.id
                )));
            }
        }

        let current_available = self.current_provider_id.as_deref().is_some_and(|id| {
            self.providers
                .iter()
                .any(|provider| provider.id == id && provider.enabled)
        });
        if !current_available {
            self.current_provider_id = self
                .providers
                .iter()
                .find(|provider| provider.enabled)
                .map(|provider| provider.id.clone());
        }
        if self.current_provider_id.is_none() {
            self.integration_enabled = false;
        }

        self.selected_target_ids.sort();
        self.selected_target_ids.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokTargetState {
    pub id: String,
    pub source: String,
    pub edition: Option<VsCodeEdition>,
    pub edition_name: Option<String>,
    pub profile_id: Option<String>,
    pub profile_name: String,
    pub is_default: bool,
    pub language_models_path: String,
    pub config_exists: bool,
    pub backup_exists: bool,
    pub selected: bool,
    pub managed_group_count: usize,
    pub read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokState {
    pub models: Vec<CopilotByokModel>,
    pub targets: Vec<CopilotByokTargetState>,
    pub selected_target_ids: Vec<String>,
    pub current_provider_id: Option<String>,
    pub integration_enabled: bool,
    pub server_running: bool,
    pub listen_port: u16,
    pub proxy_url: String,
    pub fixed_model_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokSyncResult {
    pub target_ids: Vec<String>,
    pub managed_model_count: usize,
    pub changed_target_count: usize,
    pub integration_enabled: bool,
    pub server_running: bool,
}

fn operation_guard() -> Result<MutexGuard<'static, ()>, AppError> {
    OPERATION_LOCK
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))
}

fn store_path() -> PathBuf {
    crate::config::get_app_config_dir().join(STORE_FILE)
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.cc-switch.bak")
}

fn save_store(store: &Store) -> Result<(), AppError> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    let value = serde_json::to_value(store).map_err(|source| AppError::JsonSerialize { source })?;
    crate::config::write_json_file(&path, &value)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| AppError::io(&path, error))?;
    }
    Ok(())
}

fn load_store() -> Result<Store, AppError> {
    let path = store_path();
    if !path.exists() {
        let store = Store::default();
        save_store(&store)?;
        return Ok(store);
    }

    let text = fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    let mut store: Store = serde_json::from_str(&text).map_err(|error| {
        AppError::Config(format!("Failed to parse {}: {error}", path.display()))
    })?;
    store.normalize()?;
    save_store(&store)?;
    Ok(store)
}

fn ensure_regular_file(path: &Path) -> Result<(), AppError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput(format!(
                "Refusing to modify symlinked VS Code BYOK config: {}",
                path.display()
            )));
        }
        if !metadata.is_file() {
            return Err(AppError::InvalidInput(format!(
                "VS Code BYOK target is not a regular file: {}",
                path.display()
            )));
        }
        if metadata.len() > MAX_CONFIG_BYTES {
            return Err(AppError::InvalidInput(format!(
                "VS Code BYOK config exceeds {} MiB: {}",
                MAX_CONFIG_BYTES / 1024 / 1024,
                path.display()
            )));
        }
    }
    Ok(())
}

fn read_groups(path: &Path) -> Result<Vec<Value>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    ensure_regular_file(path)?;
    let text = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = json5::from_str(&text).map_err(|error| {
        AppError::Config(format!(
            "Failed to parse VS Code BYOK config {}: {error}",
            path.display()
        ))
    })?;
    value.as_array().cloned().ok_or_else(|| {
        AppError::Config(format!(
            "VS Code BYOK config must be a JSON array: {}",
            path.display()
        ))
    })
}

fn create_backup_once(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }
    let backup = backup_path(path);
    if backup.exists() {
        ensure_regular_file(&backup)?;
        return Ok(());
    }
    fs::copy(path, &backup).map_err(|error| AppError::io(&backup, error))?;
    Ok(())
}

fn write_groups(path: &Path, groups: &[Value]) -> Result<(), AppError> {
    ensure_regular_file(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    create_backup_once(path)?;
    crate::config::write_json_file(path, &Value::Array(groups.to_vec()))
}

fn is_managed_group(value: &Value) -> bool {
    value.get("vendor").and_then(Value::as_str) == Some("customendpoint")
        && value.get("name").and_then(Value::as_str) == Some(GROUP_NAME)
}

fn current_provider(store: &Store) -> Option<&CopilotByokModel> {
    let id = store.current_provider_id.as_deref()?;
    store
        .providers
        .iter()
        .find(|provider| provider.id == id && provider.enabled)
}

fn managed_group(store: &Store) -> Value {
    let provider = current_provider(store);
    json!({
        "name": GROUP_NAME,
        "vendor": "customendpoint",
        "apiKey": store.gateway_token,
        "apiType": "chat-completions",
        "models": [{
            "id": MODEL_ID,
            "name": "CC Switch Current",
            "url": format!(
                "http://127.0.0.1:{}/copilot/v1/chat/completions",
                store.listen_port
            ),
            "contextWindow": provider
                .map(|provider| provider.context_window)
                .unwrap_or(DEFAULT_CONTEXT_WINDOW),
            "maxOutputTokens": provider
                .map(|provider| provider.max_output_tokens)
                .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
            "toolCalling": provider.map(|provider| provider.tool_calling).unwrap_or(true),
            "vision": provider.map(|provider| provider.vision).unwrap_or(false),
            "thinking": provider.map(|provider| provider.thinking).unwrap_or(true),
            "streaming": provider.map(|provider| provider.streaming).unwrap_or(true)
        }]
    })
}

fn target_path_map(
    store: &Store,
    detected: &[VsCodeProfileTarget],
) -> HashMap<String, PathBuf> {
    let mut paths = HashMap::new();
    for target in detected {
        paths.insert(target.id.clone(), target.path());
    }
    for target in &store.custom_targets {
        paths.insert(
            target.id.clone(),
            PathBuf::from(&target.language_models_path),
        );
    }
    paths
}

fn effective_selected_target_ids(
    store: &Store,
    detected: &[VsCodeProfileTarget],
) -> Vec<String> {
    if store.targets_initialized {
        return store.selected_target_ids.clone();
    }
    detected
        .iter()
        .find(|target| target.id == "stable:default")
        .or_else(|| detected.first())
        .map(|target| vec![target.id.clone()])
        .unwrap_or_default()
}

fn available_selected_target_ids(
    store: &Store,
    detected: &[VsCodeProfileTarget],
) -> Vec<String> {
    let available: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(store.custom_targets.iter().map(|target| target.id.clone()))
        .collect();
    effective_selected_target_ids(store, detected)
        .into_iter()
        .filter(|id| available.contains(id))
        .collect()
}

fn resolve_target_paths(
    store: &Store,
    requested_ids: &[String],
) -> Result<Vec<(String, PathBuf)>, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let paths = target_path_map(store, &detected);
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();
    for id in requested_ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let path = paths.get(id).cloned().ok_or_else(|| {
            AppError::InvalidInput(format!("Unknown or unavailable VS Code target: {id}"))
        })?;
        resolved.push((id.clone(), path));
    }
    Ok(resolved)
}

fn sync_vscode(store: &Store) -> Result<CopilotByokSyncResult, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let target_ids = available_selected_target_ids(store, &detected);
    if target_ids.is_empty() {
        return Err(AppError::InvalidInput(
            "No VS Code profile is selected for Copilot proxy integration".to_string(),
        ));
    }

    let targets = resolve_target_paths(store, &target_ids)?;
    let desired_group = managed_group(store);
    let mut changed_target_count = 0;
    for (_, path) in &targets {
        let existing = read_groups(path)?;
        let mut merged: Vec<Value> = existing
            .iter()
            .filter(|group| !is_managed_group(group))
            .cloned()
            .collect();
        merged.push(desired_group.clone());
        if merged != existing {
            write_groups(path, &merged)?;
            changed_target_count += 1;
        }
    }

    Ok(CopilotByokSyncResult {
        target_ids,
        managed_model_count: usize::from(store.current_provider_id.is_some()),
        changed_target_count,
        integration_enabled: store.integration_enabled,
        server_running: SERVER_RUNNING.load(Ordering::SeqCst),
    })
}

fn remove_managed_groups(store: &Store, requested_ids: &[String]) -> Result<usize, AppError> {
    let targets = resolve_target_paths(store, requested_ids)?;
    let mut changed = 0;
    for (_, path) in targets {
        if !path.exists() {
            continue;
        }
        let existing = read_groups(&path)?;
        let unmanaged: Vec<Value> = existing
            .iter()
            .filter(|group| !is_managed_group(group))
            .cloned()
            .collect();
        if unmanaged == existing {
            continue;
        }
        if unmanaged.is_empty() && !backup_path(&path).exists() {
            fs::remove_file(&path).map_err(|error| AppError::io(&path, error))?;
        } else {
            write_groups(&path, &unmanaged)?;
        }
        changed += 1;
    }
    Ok(changed)
}

fn inspect_target(path: &Path) -> (usize, Option<String>) {
    match read_groups(path) {
        Ok(groups) => (
            groups.iter().filter(|group| is_managed_group(group)).count(),
            None,
        ),
        Err(error) => (0, Some(error.to_string())),
    }
}

fn build_state(store: Store) -> Result<CopilotByokState, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let selected_target_ids = available_selected_target_ids(&store, &detected);
    let selected: HashSet<String> = selected_target_ids.iter().cloned().collect();
    let mut targets = Vec::new();

    for target in detected {
        let path = target.path();
        let (managed_group_count, read_error) = inspect_target(&path);
        targets.push(CopilotByokTargetState {
            selected: selected.contains(&target.id),
            id: target.id,
            source: "detected".to_string(),
            edition: Some(target.edition),
            edition_name: Some(target.edition_name),
            profile_id: target.profile_id,
            profile_name: target.profile_name,
            is_default: target.is_default,
            language_models_path: target.language_models_path,
            config_exists: path.exists(),
            backup_exists: backup_path(&path).exists(),
            managed_group_count,
            read_error,
        });
    }

    for target in &store.custom_targets {
        let path = PathBuf::from(&target.language_models_path);
        let (managed_group_count, read_error) = inspect_target(&path);
        targets.push(CopilotByokTargetState {
            id: target.id.clone(),
            source: "custom".to_string(),
            edition: None,
            edition_name: None,
            profile_id: None,
            profile_name: target.name.clone(),
            is_default: false,
            language_models_path: target.language_models_path.clone(),
            config_exists: path.exists(),
            backup_exists: backup_path(&path).exists(),
            selected: selected.contains(&target.id),
            managed_group_count,
            read_error,
        });
    }

    targets.sort_by(|left, right| {
        right
            .selected
            .cmp(&left.selected)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.profile_name.cmp(&right.profile_name))
    });

    Ok(CopilotByokState {
        models: store.providers,
        targets,
        selected_target_ids,
        current_provider_id: store.current_provider_id,
        integration_enabled: store.integration_enabled,
        server_running: SERVER_RUNNING.load(Ordering::SeqCst),
        listen_port: store.listen_port,
        proxy_url: format!(
            "http://127.0.0.1:{}/copilot/v1/chat/completions",
            store.listen_port
        ),
        fixed_model_id: MODEL_ID.to_string(),
    })
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

async fn proxy_handler(headers: HeaderMap, Json(mut body): Json<Value>) -> Response {
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

async fn stop_server() -> Result<(), AppError> {
    let runtime = {
        let mut slot = SERVER_RUNTIME
            .lock()
            .map_err(|error| AppError::Lock(error.to_string()))?;
        slot.take()
    };

    if let Some(ServerRuntime {
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
    SERVER_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

async fn ensure_server(port: u16) -> Result<(), AppError> {
    let already_running = SERVER_RUNTIME
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))?
        .as_ref()
        .is_some_and(|runtime| runtime.port == port && SERVER_RUNNING.load(Ordering::SeqCst));
    if already_running {
        return Ok(());
    }

    stop_server().await?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| {
            AppError::Config(format!(
                "Failed to bind Copilot proxy on 127.0.0.1:{port}: {error}"
            ))
        })?;
    let (shutdown, shutdown_rx) = oneshot::channel();
    let app = Router::new()
        .route("/copilot/v1/chat/completions", post(proxy_handler))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES));

    SERVER_RUNNING.store(true, Ordering::SeqCst);
    let handle = tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        SERVER_RUNNING.store(false, Ordering::SeqCst);
        if let Err(error) = result {
            log::error!("Copilot BYOK proxy server stopped unexpectedly: {error}");
        }
    });

    let mut slot = SERVER_RUNTIME
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))?;
    *slot = Some(ServerRuntime {
        port,
        shutdown,
        handle,
    });
    Ok(())
}

async fn activate_store(store: &Store) -> Result<CopilotByokSyncResult, AppError> {
    if current_provider(store).is_none() {
        return Err(AppError::InvalidInput(
            "Select an enabled Copilot proxy provider first".to_string(),
        ));
    }
    ensure_server(store.listen_port).await?;
    match sync_vscode(store) {
        Ok(result) => Ok(result),
        Err(error) => {
            let _ = stop_server().await;
            Err(error)
        }
    }
}

fn selected_ids_for_store(store: &Store) -> Result<Vec<String>, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    Ok(available_selected_target_ids(store, &detected))
}

pub async fn get_state() -> Result<CopilotByokState, AppError> {
    let store = {
        let _guard = operation_guard()?;
        load_store()?
    };

    if store.integration_enabled && current_provider(&store).is_some() {
        if let Err(error) = activate_store(&store).await {
            log::warn!("Failed to restore Copilot proxy integration: {error}");
        }
    }
    build_state(store)
}

pub async fn set_targets(target_ids: Vec<String>) -> Result<CopilotByokState, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        let detected = vscode::discover_vscode_targets()?;
        let valid_ids: HashSet<String> = detected
            .iter()
            .map(|target| target.id.clone())
            .chain(store.custom_targets.iter().map(|target| target.id.clone()))
            .collect();
        if let Some(invalid) = target_ids.iter().find(|id| !valid_ids.contains(*id)) {
            return Err(AppError::InvalidInput(format!(
                "Unknown or unavailable VS Code target: {invalid}"
            )));
        }

        let previous: HashSet<String> = available_selected_target_ids(&store, &detected)
            .into_iter()
            .collect();
        let next: HashSet<String> = target_ids.iter().cloned().collect();
        let removed: Vec<String> = previous.difference(&next).cloned().collect();
        if !removed.is_empty() {
            remove_managed_groups(&store, &removed)?;
        }

        store.targets_initialized = true;
        store.selected_target_ids = next.into_iter().collect();
        store.selected_target_ids.sort();
        if store.selected_target_ids.is_empty() {
            store.integration_enabled = false;
        }
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    } else {
        stop_server().await?;
    }
    build_state(store)
}

pub async fn add_custom_target(
    path: String,
    name: Option<String>,
) -> Result<CopilotByokState, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        let path = PathBuf::from(path.trim());
        if !path.is_absolute() {
            return Err(AppError::InvalidInput(
                "Custom VS Code target path must be absolute".to_string(),
            ));
        }
        if path.file_name().and_then(|value| value.to_str()) != Some("chatLanguageModels.json") {
            return Err(AppError::InvalidInput(
                "Custom target must point to chatLanguageModels.json".to_string(),
            ));
        }
        ensure_regular_file(&path)?;
        if store
            .custom_targets
            .iter()
            .any(|target| PathBuf::from(&target.language_models_path) == path)
        {
            return Err(AppError::InvalidInput(
                "This custom VS Code target already exists".to_string(),
            ));
        }

        let id = format!("custom:{}", uuid::Uuid::new_v4());
        store.custom_targets.push(CustomTarget {
            id: id.clone(),
            name: name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Custom VS Code Profile")
                .to_string(),
            language_models_path: path.to_string_lossy().to_string(),
        });
        store.targets_initialized = true;
        store.selected_target_ids.push(id);
        store.normalize()?;
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    }
    build_state(store)
}

pub async fn remove_custom_target(target_id: &str) -> Result<CopilotByokState, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        if store
            .custom_targets
            .iter()
            .any(|target| target.id == target_id)
        {
            remove_managed_groups(&store, &[target_id.to_string()])?;
        }
        store.custom_targets.retain(|target| target.id != target_id);
        store.selected_target_ids.retain(|id| id != target_id);
        if store.selected_target_ids.is_empty() {
            store.integration_enabled = false;
        }
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    } else {
        stop_server().await?;
    }
    build_state(store)
}

pub async fn upsert_model(mut model: CopilotByokModel) -> Result<CopilotByokState, AppError> {
    model.normalize();
    model.validate()?;
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        if let Some(existing) = store
            .providers
            .iter_mut()
            .find(|provider| provider.id == model.id)
        {
            *existing = model;
        } else {
            store.providers.push(model);
        }
        store.normalize()?;
        if store.current_provider_id.is_none() {
            let selected = selected_ids_for_store(&store)?;
            if !selected.is_empty() {
                remove_managed_groups(&store, &selected)?;
            }
        }
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    } else if store.current_provider_id.is_none() {
        stop_server().await?;
    }
    build_state(store)
}

pub async fn delete_model(model_id: &str) -> Result<CopilotByokState, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        store.providers.retain(|provider| provider.id != model_id);
        store.normalize()?;
        if store.current_provider_id.is_none() {
            let selected = selected_ids_for_store(&store)?;
            if !selected.is_empty() {
                remove_managed_groups(&store, &selected)?;
            }
        }
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    } else if store.current_provider_id.is_none() {
        stop_server().await?;
    }
    build_state(store)
}

pub async fn sync(provider_id: Option<String>) -> Result<CopilotByokSyncResult, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        if let Some(provider_id) = provider_id {
            if !store
                .providers
                .iter()
                .any(|provider| provider.id == provider_id && provider.enabled)
            {
                return Err(AppError::InvalidInput(format!(
                    "Unknown or disabled Copilot proxy provider: {provider_id}"
                )));
            }
            store.current_provider_id = Some(provider_id);
        }
        if current_provider(&store).is_none() {
            return Err(AppError::InvalidInput(
                "Select an enabled Copilot proxy provider first".to_string(),
            ));
        }
        if selected_ids_for_store(&store)?.is_empty() {
            return Err(AppError::InvalidInput(
                "Select at least one VS Code profile first".to_string(),
            ));
        }
        store.integration_enabled = true;
        save_store(&store)?;
        store
    };
    activate_store(&store).await
}

pub async fn remove_managed_models(
    requested_ids: Option<Vec<String>>,
) -> Result<CopilotByokSyncResult, AppError> {
    let (store, target_ids, changed_target_count) = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        let target_ids = match requested_ids.filter(|ids| !ids.is_empty()) {
            Some(ids) => ids,
            None => selected_ids_for_store(&store)?,
        };
        let changed = remove_managed_groups(&store, &target_ids)?;
        store.integration_enabled = false;
        save_store(&store)?;
        (store, target_ids, changed)
    };

    stop_server().await?;
    Ok(CopilotByokSyncResult {
        target_ids,
        managed_model_count: usize::from(store.current_provider_id.is_some()),
        changed_target_count,
        integration_enabled: false,
        server_running: false,
    })
}

pub async fn restore_backup(target_id: &str) -> Result<bool, AppError> {
    let (store, restored) = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        let targets = resolve_target_paths(&store, &[target_id.to_string()])?;
        let (_, path) = targets.into_iter().next().ok_or_else(|| {
            AppError::InvalidInput("VS Code target is required".to_string())
        })?;
        let backup = backup_path(&path);
        let restored = if backup.exists() {
            ensure_regular_file(&backup)?;
            if path.exists() {
                ensure_regular_file(&path)?;
            }
            let contents = fs::read(&backup).map_err(|error| AppError::io(&backup, error))?;
            crate::config::atomic_write(&path, &contents)?;
            true
        } else {
            remove_managed_groups(&store, &[target_id.to_string()])? > 0
        };

        store.targets_initialized = true;
        store.selected_target_ids.retain(|id| id != target_id);
        if store.selected_target_ids.is_empty() {
            store.integration_enabled = false;
        }
        save_store(&store)?;
        (store, restored)
    };

    if store.integration_enabled {
        activate_store(&store).await?;
    } else {
        stop_server().await?;
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_store_has_local_gateway_token() {
        let store = Store::default();
        assert!(!store.gateway_token.is_empty());
        assert_eq!(store.listen_port, DEFAULT_LISTEN_PORT);
    }

    #[test]
    fn provider_rejects_non_http_endpoint() {
        let provider = CopilotByokModel {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            endpoint: "file:///tmp/endpoint".to_string(),
            api_key: "secret".to_string(),
            model_id: "model".to_string(),
            enabled: true,
            request_headers: BTreeMap::new(),
            context_window: DEFAULT_CONTEXT_WINDOW,
            max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
            tool_calling: true,
            vision: false,
            thinking: true,
            streaming: true,
        };
        assert!(provider.validate().is_err());
    }

    #[test]
    fn managed_group_uses_fixed_model_id() {
        let store = Store::default();
        let group = managed_group(&store);
        assert_eq!(group["models"][0]["id"], MODEL_ID);
    }
}
