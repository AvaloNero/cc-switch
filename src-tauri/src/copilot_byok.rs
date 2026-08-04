use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const STORE_FILE: &str = "copilot-byok.json";
const MANAGED_PREFIX: &str = "CC Switch:";

fn default_true() -> bool {
    true
}
fn default_context_window() -> u64 {
    262_144
}
fn default_max_output_tokens() -> u64 {
    32_768
}
fn default_api_type() -> String {
    "chat-completions".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_api_type")]
    pub api_type: String,
    pub url: String,
    pub model_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub tool_calling: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default = "default_true")]
    pub thinking: bool,
    #[serde(default = "default_true")]
    pub streaming: bool,
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub model_options: Value,
}

impl CopilotByokModel {
    fn normalize(&mut self) {
        self.id = self.id.trim().to_string();
        if self.id.is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
        self.name = self.name.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.api_type = self.api_type.trim().to_ascii_lowercase();
        self.url = self.url.trim().to_string();
        self.model_id = self.model_id.trim().to_string();
        if self.context_window == 0 {
            self.context_window = default_context_window();
        }
        if self.max_output_tokens == 0 {
            self.max_output_tokens = default_max_output_tokens();
        }
    }

    fn validate(&self) -> Result<(), AppError> {
        if self.name.is_empty() {
            return Err(AppError::Config("Copilot BYOK model name is required".to_string()));
        }
        if self.api_key.is_empty() {
            return Err(AppError::Config("Copilot BYOK API key is required".to_string()));
        }
        if self.model_id.is_empty() {
            return Err(AppError::Config("Copilot BYOK model id is required".to_string()));
        }
        let parsed = url::Url::parse(&self.url)
            .map_err(|e| AppError::Config(format!("Invalid Copilot BYOK endpoint URL: {e}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AppError::Config(
                "Copilot BYOK endpoint must use http or https".to_string(),
            ));
        }
        if !matches!(
            self.api_type.as_str(),
            "chat-completions" | "responses" | "messages"
        ) {
            return Err(AppError::Config(format!(
                "Unsupported Copilot BYOK API type: {}",
                self.api_type
            )));
        }
        Ok(())
    }

    fn to_language_model_group(&self) -> Value {
        let mut model = json!({
            "id": self.model_id,
            "name": self.name,
            "url": self.url,
            "toolCalling": self.tool_calling,
            "vision": self.vision,
            "thinking": self.thinking,
            "streaming": self.streaming,
            "contextWindow": self.context_window,
            "maxOutputTokens": self.max_output_tokens,
        });
        if !self.model_options.is_null() && self.model_options != json!({}) {
            model["modelOptions"] = self.model_options.clone();
        }
        if !self.request_headers.is_empty() {
            model["requestHeaders"] = serde_json::to_value(&self.request_headers)
                .unwrap_or_else(|_| json!({}));
        }

        json!({
            "name": format!("{MANAGED_PREFIX} {}", self.name),
            "vendor": "customendpoint",
            "apiKey": self.api_key,
            "apiType": self.api_type,
            "models": [model],
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokStore {
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub models: Vec<CopilotByokModel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokState {
    pub store: CopilotByokStore,
    pub resolved_config_path: String,
    pub config_exists: bool,
    pub managed_model_count: usize,
}

fn store_path() -> PathBuf {
    crate::config::get_app_config_dir().join(STORE_FILE)
}

fn default_vscode_config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(crate::config::get_home_dir);
    base.join("Code").join("User").join("chatLanguageModels.json")
}

fn resolve_config_path(store: &CopilotByokStore) -> PathBuf {
    store
        .config_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_vscode_config_path)
}

pub fn load_store() -> Result<CopilotByokStore, AppError> {
    let path = store_path();
    if !path.exists() {
        return Ok(CopilotByokStore::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    serde_json::from_str(&text).map_err(|e| {
        AppError::Config(format!("Failed to parse {}: {e}", path.display()))
    })
}

fn save_store(store: &CopilotByokStore) -> Result<(), AppError> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    crate::config::write_json_file(
        &path,
        &serde_json::to_value(store).map_err(|e| AppError::JsonSerialize { source: e })?,
    )
}

fn read_language_models(path: &Path) -> Result<Vec<Value>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = json5::from_str(&text).map_err(|e| {
        AppError::Config(format!(
            "Failed to parse VS Code language model config {}: {e}",
            path.display()
        ))
    })?;
    value.as_array().cloned().ok_or_else(|| {
        AppError::Config(format!(
            "VS Code language model config must be a JSON array: {}",
            path.display()
        ))
    })
}

fn is_managed_group(value: &Value) -> bool {
    value.get("vendor").and_then(Value::as_str) == Some("customendpoint")
        && value
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.starts_with(MANAGED_PREFIX))
}

fn write_language_models(path: &Path, groups: Vec<Value>) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    if path.exists() {
        let backup = path.with_extension("json.cc-switch.bak");
        if !backup.exists() {
            fs::copy(path, &backup).map_err(|e| AppError::io(&backup, e))?;
        }
    }
    crate::config::write_json_file(path, &Value::Array(groups))
}

pub fn sync_store(store: &CopilotByokStore) -> Result<(), AppError> {
    let path = resolve_config_path(store);
    let mut groups = read_language_models(&path)?;
    groups.retain(|group| !is_managed_group(group));
    groups.extend(
        store
            .models
            .iter()
            .filter(|model| model.enabled)
            .map(CopilotByokModel::to_language_model_group),
    );
    write_language_models(&path, groups)
}

pub fn get_state() -> Result<CopilotByokState, AppError> {
    let store = load_store()?;
    let path = resolve_config_path(&store);
    Ok(CopilotByokState {
        managed_model_count: store.models.iter().filter(|model| model.enabled).count(),
        config_exists: path.exists(),
        resolved_config_path: path.to_string_lossy().to_string(),
        store,
    })
}

pub fn upsert_model(mut model: CopilotByokModel) -> Result<CopilotByokState, AppError> {
    model.normalize();
    model.validate()?;
    let mut store = load_store()?;
    if let Some(existing) = store.models.iter_mut().find(|item| item.id == model.id) {
        *existing = model;
    } else {
        store.models.push(model);
    }
    save_store(&store)?;
    sync_store(&store)?;
    get_state()
}

pub fn delete_model(id: &str) -> Result<CopilotByokState, AppError> {
    let mut store = load_store()?;
    store.models.retain(|model| model.id != id);
    save_store(&store)?;
    sync_store(&store)?;
    get_state()
}

pub fn set_config_path(path: Option<String>) -> Result<CopilotByokState, AppError> {
    let mut store = load_store()?;
    store.config_path = path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    save_store(&store)?;
    sync_store(&store)?;
    get_state()
}

pub fn sync() -> Result<CopilotByokState, AppError> {
    let store = load_store()?;
    sync_store(&store)?;
    get_state()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_group_detection_is_scoped() {
        assert!(is_managed_group(&json!({
            "name": "CC Switch: Kimi",
            "vendor": "customendpoint"
        })));
        assert!(!is_managed_group(&json!({
            "name": "My Kimi",
            "vendor": "customendpoint"
        })));
    }

    #[test]
    fn model_validation_rejects_non_http_url() {
        let mut model = CopilotByokModel {
            id: String::new(),
            name: "test".to_string(),
            api_key: "secret".to_string(),
            api_type: "chat-completions".to_string(),
            url: "file:///tmp/model".to_string(),
            model_id: "model".to_string(),
            enabled: true,
            tool_calling: true,
            vision: false,
            thinking: true,
            streaming: true,
            context_window: 1,
            max_output_tokens: 1,
            request_headers: BTreeMap::new(),
            model_options: json!({}),
        };
        model.normalize();
        assert!(model.validate().is_err());
    }
}
