use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const MANAGED_PREFIX: &str = "CC Switch:";

fn default_true() -> bool {
    true
}

fn default_api_type() -> String {
    "chat-completions".to_string()
}

fn default_context_window() -> u64 {
    262_144
}

fn default_max_output_tokens() -> u64 {
    32_768
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokModel {
    #[serde(default)]
    pub id: String,
    pub model_id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_api_type")]
    pub api_type: String,
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
    #[serde(default)]
    pub max_input_tokens: Option<u64>,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default)]
    pub supports_reasoning_effort: Vec<String>,
    #[serde(default)]
    pub reasoning_effort_format: Option<String>,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub model_options: Value,
}

impl CopilotByokModel {
    pub fn normalize(&mut self) {
        self.id = self.id.trim().to_string();
        if self.id.is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
        self.model_id = self.model_id.trim().to_string();
        self.name = self.name.trim().to_string();
        self.url = self.url.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.api_type = self.api_type.trim().to_ascii_lowercase();
        self.supports_reasoning_effort = self
            .supports_reasoning_effort
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .collect();
        self.supports_reasoning_effort.sort();
        self.supports_reasoning_effort.dedup();
        self.reasoning_effort_format = self
            .reasoning_effort_format
            .take()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        if self.context_window == 0 {
            self.context_window = default_context_window();
        }
        if self.max_output_tokens == 0 {
            self.max_output_tokens = default_max_output_tokens();
        }
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if self.model_id.is_empty() {
            return Err(AppError::InvalidInput(
                "Copilot BYOK model id is required".to_string(),
            ));
        }
        if self.name.is_empty() {
            return Err(AppError::InvalidInput(
                "Copilot BYOK display name is required".to_string(),
            ));
        }

        let parsed = url::Url::parse(&self.url).map_err(|error| {
            AppError::InvalidInput(format!("Invalid Copilot BYOK endpoint URL: {error}"))
        })?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(AppError::InvalidInput(
                "Copilot BYOK endpoint must be an absolute HTTP(S) URL".to_string(),
            ));
        }
        if !matches!(
            self.api_type.as_str(),
            "chat-completions" | "responses" | "messages"
        ) {
            return Err(AppError::InvalidInput(format!(
                "Unsupported Copilot BYOK API type: {}",
                self.api_type
            )));
        }
        if let Some(format) = self.reasoning_effort_format.as_deref() {
            if !matches!(format, "chat-completions" | "responses" | "messages") {
                return Err(AppError::InvalidInput(format!(
                    "Unsupported reasoning effort format: {format}"
                )));
            }
        }
        for (name, value) in &self.request_headers {
            if name.trim().is_empty() || name.contains(['\r', '\n']) || value.contains(['\r', '\n']) {
                return Err(AppError::InvalidInput(
                    "Copilot BYOK request headers must not contain empty names or newlines"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn to_language_model_group(&self) -> Value {
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
        if let Some(max_input_tokens) = self.max_input_tokens {
            model["maxInputTokens"] = json!(max_input_tokens);
        }
        if !self.supports_reasoning_effort.is_empty() {
            model["supportedReasoningEfforts"] = json!(self.supports_reasoning_effort);
        }
        if let Some(format) = &self.reasoning_effort_format {
            model["reasoningEffortFormat"] = json!(format);
        }
        if !self.request_headers.is_empty() {
            model["requestHeaders"] = json!(self.request_headers);
        }
        if !self.model_options.is_null() && self.model_options != json!({}) {
            model["modelOptions"] = self.model_options.clone();
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

pub fn is_managed_group(value: &Value) -> bool {
    value.get("vendor").and_then(Value::as_str) == Some("customendpoint")
        && value
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.starts_with(MANAGED_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> CopilotByokModel {
        CopilotByokModel {
            id: String::new(),
            model_id: "kimi-k3".to_string(),
            name: "Kimi K3".to_string(),
            url: "https://api.example.com/v1/chat/completions".to_string(),
            api_key: "secret".to_string(),
            api_type: "chat-completions".to_string(),
            enabled: true,
            tool_calling: true,
            vision: false,
            thinking: true,
            streaming: true,
            context_window: 262_144,
            max_input_tokens: None,
            max_output_tokens: 32_768,
            supports_reasoning_effort: Vec::new(),
            reasoning_effort_format: None,
            request_headers: BTreeMap::new(),
            model_options: json!({}),
        }
    }

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
    fn validation_rejects_non_http_endpoint() {
        let mut value = model();
        value.url = "file:///tmp/model".to_string();
        assert!(value.validate().is_err());
    }

    #[test]
    fn generated_group_has_stable_ownership_marker() {
        let group = model().to_language_model_group();
        assert_eq!(group["name"], "CC Switch: Kimi K3");
        assert_eq!(group["vendor"], "customendpoint");
        assert_eq!(group["models"][0]["id"], "kimi-k3");
    }
}
