use super::model::{is_managed_group, CopilotByokModel};
use super::store::{save_store, CopilotByokStore};
use super::sync;
use crate::error::AppError;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

const DEFAULT_CONTEXT_WINDOW: u64 = 128_000;
const DEFAULT_MAX_OUTPUT_TOKENS: u64 = 8_192;
const SUPPORTED_EDIT_TOOLS: [&str; 4] = [
    "find-replace",
    "multi-find-replace",
    "apply-patch",
    "code-rewrite",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokImportResult {
    pub target_id: String,
    pub imported_group_count: usize,
    pub imported_model_count: usize,
    pub reused_model_count: usize,
    pub skipped_group_count: usize,
    pub changed_target_count: usize,
    pub warnings: Vec<String>,
}

struct ParsedGroup {
    name: String,
    models: Vec<CopilotByokModel>,
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn value_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn value_bool(value: Option<&Value>, default: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(default)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    let Some(object) = value.and_then(Value::as_object) else {
        return BTreeMap::new();
    };

    object
        .iter()
        .filter_map(|(key, value)| {
            let rendered = match value {
                Value::String(value) => value.clone(),
                Value::Bool(value) => value.to_string(),
                Value::Number(value) => value.to_string(),
                _ => return None,
            };
            Some((key.clone(), rendered))
        })
        .collect()
}

fn infer_api_type(url: &str) -> String {
    if url.contains("/messages") {
        "messages".to_string()
    } else if url.contains("/responses") {
        "responses".to_string()
    } else {
        "chat-completions".to_string()
    }
}

fn deterministic_model_id(target_id: &str, group_name: &str, model_id: &str, url: &str) -> String {
    let digest = Sha256::digest(
        format!("{target_id}\0{group_name}\0{model_id}\0{url}").as_bytes(),
    );
    let encoded = format!("{digest:x}");
    format!("vscode-import:{}", &encoded[..24])
}

fn parse_model(
    target_id: &str,
    group_name: &str,
    group: &Map<String, Value>,
    value: &Value,
) -> Result<CopilotByokModel, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "model entry is not a JSON object".to_string())?;
    let model_id = value_string(object.get("id"))
        .ok_or_else(|| "model entry is missing id".to_string())?;
    let name = value_string(object.get("name")).unwrap_or_else(|| model_id.clone());
    let url = value_string(object.get("url"))
        .or_else(|| value_string(group.get("url")))
        .ok_or_else(|| format!("model {model_id} is missing url"))?;
    let api_key = value_string(object.get("apiKey"))
        .or_else(|| value_string(group.get("apiKey")))
        .ok_or_else(|| format!("model {model_id} is missing apiKey"))?;
    let api_type = value_string(object.get("apiType"))
        .or_else(|| value_string(group.get("apiType")))
        .unwrap_or_else(|| infer_api_type(&url));

    let mut edit_tools = string_array(object.get("editTools"));
    edit_tools.retain(|tool| SUPPORTED_EDIT_TOOLS.contains(&tool.as_str()));

    let mut request_headers = string_map(group.get("requestHeaders"));
    request_headers.extend(string_map(object.get("requestHeaders")));

    let mut model = CopilotByokModel {
        id: deterministic_model_id(target_id, group_name, &model_id, &url),
        model_id,
        name,
        url,
        api_key,
        api_type,
        enabled: true,
        tool_calling: value_bool(object.get("toolCalling"), false),
        vision: value_bool(object.get("vision"), false),
        thinking: value_bool(object.get("thinking"), false),
        streaming: value_bool(object.get("streaming"), true),
        context_window: value_u64(object.get("contextWindow"))
            .unwrap_or(DEFAULT_CONTEXT_WINDOW),
        max_input_tokens: value_u64(object.get("maxInputTokens")),
        max_output_tokens: value_u64(object.get("maxOutputTokens"))
            .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
        edit_tools,
        zero_data_retention_enabled: value_bool(
            object.get("zeroDataRetentionEnabled"),
            false,
        ),
        supports_reasoning_effort: string_array(object.get("supportsReasoningEffort")),
        reasoning_effort_format: value_string(object.get("reasoningEffortFormat")),
        request_headers,
        model_options: object
            .get("modelOptions")
            .cloned()
            .or_else(|| group.get("modelOptions").cloned())
            .unwrap_or_else(|| json!({})),
    };
    model.normalize();
    model.validate().map_err(|error| error.to_string())?;
    Ok(model)
}

fn parse_group(target_id: &str, value: &Value) -> Result<ParsedGroup, String> {
    let group = value
        .as_object()
        .ok_or_else(|| "provider group is not a JSON object".to_string())?;
    let name = value_string(group.get("name")).unwrap_or_else(|| "Custom Endpoint".to_string());
    let models = group
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name} has no static models array"))?;
    if models.is_empty() {
        return Err(format!("{name} has an empty models array"));
    }

    let parsed = models
        .iter()
        .map(|model| parse_model(target_id, &name, group, model))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedGroup {
        name,
        models: parsed,
    })
}

fn fingerprint(model: &CopilotByokModel) -> (String, String) {
    (model.model_id.to_ascii_lowercase(), model.url.to_ascii_lowercase())
}

fn add_group_models(
    store: &mut CopilotByokStore,
    parsed: ParsedGroup,
) -> Result<(usize, usize), String> {
    let existing_fingerprints: HashSet<(String, String)> =
        store.models.iter().map(fingerprint).collect();
    let existing_names: HashSet<String> = store
        .models
        .iter()
        .map(|model| model.name.to_ascii_lowercase())
        .collect();
    let existing_ids: HashSet<String> = store.models.iter().map(|model| model.id.clone()).collect();

    let mut staged = Vec::new();
    let mut staged_names = HashSet::new();
    let mut staged_fingerprints = HashSet::new();
    let mut reused = 0;

    for mut model in parsed.models {
        let model_fingerprint = fingerprint(&model);
        if existing_fingerprints.contains(&model_fingerprint) {
            reused += 1;
            continue;
        }
        if !staged_fingerprints.insert(model_fingerprint) {
            return Err(format!("{} contains duplicate model endpoints", parsed.name));
        }
        if existing_ids.contains(&model.id) {
            return Err(format!("{} produced a conflicting internal model id", parsed.name));
        }

        let original_name = model.name.clone();
        let original_key = original_name.to_ascii_lowercase();
        if existing_names.contains(&original_key) || !staged_names.insert(original_key) {
            model.name = format!("{} · {}", parsed.name, original_name);
            let namespaced_key = model.name.to_ascii_lowercase();
            if existing_names.contains(&namespaced_key) || !staged_names.insert(namespaced_key) {
                return Err(format!(
                    "{} conflicts with an existing model display name",
                    parsed.name
                ));
            }
        }
        staged.push(model);
    }

    let imported = staged.len();
    store.models.extend(staged);
    Ok((imported, reused))
}

pub fn import_from_target(
    mut store: CopilotByokStore,
    target_id: &str,
) -> Result<CopilotByokImportResult, AppError> {
    let resolved = sync::resolve_target_paths(&store, &[target_id.to_string()])?;
    let (_, path) = resolved
        .into_iter()
        .next()
        .ok_or_else(|| AppError::InvalidInput("Copilot BYOK target is required".to_string()))?;
    let groups = sync::read_language_model_groups(&path)?;

    let mut accepted_indexes = HashSet::new();
    let mut imported_group_count = 0;
    let mut imported_model_count = 0;
    let mut reused_model_count = 0;
    let mut skipped_group_count = 0;
    let mut warnings = Vec::new();

    for (index, group) in groups.iter().enumerate() {
        if is_managed_group(group)
            || group.get("vendor").and_then(Value::as_str) != Some("customendpoint")
        {
            continue;
        }

        match parse_group(target_id, group)
            .and_then(|parsed| add_group_models(&mut store, parsed))
        {
            Ok((imported, reused)) => {
                accepted_indexes.insert(index);
                imported_group_count += 1;
                imported_model_count += imported;
                reused_model_count += reused;
            }
            Err(reason) => {
                skipped_group_count += 1;
                warnings.push(reason);
            }
        }
    }

    if accepted_indexes.is_empty() {
        return Ok(CopilotByokImportResult {
            target_id: target_id.to_string(),
            imported_group_count,
            imported_model_count,
            reused_model_count,
            skipped_group_count,
            changed_target_count: 0,
            warnings,
        });
    }

    store.targets_initialized = true;
    if !store.selected_target_ids.iter().any(|id| id == target_id) {
        store.selected_target_ids.push(target_id.to_string());
    }
    save_store(&store)?;

    let unmanaged: Vec<Value> = groups
        .into_iter()
        .enumerate()
        .filter_map(|(index, group)| (!accepted_indexes.contains(&index)).then_some(group))
        .collect();
    let converted = sync::merge_managed_groups(unmanaged, &store.models);
    sync::write_language_model_groups(&path, &converted)?;
    let sync_result = sync::sync_store(&store)?;

    Ok(CopilotByokImportResult {
        target_id: target_id.to_string(),
        imported_group_count,
        imported_model_count,
        reused_model_count,
        skipped_group_count,
        changed_target_count: sync_result.changed_target_count,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_custom_endpoint_fields() {
        let group = json!({
            "name": "Existing Kimi",
            "vendor": "customendpoint",
            "apiKey": "${input:chat.lm.secret.test}",
            "apiType": "responses",
            "models": [{
                "id": "kimi-k3",
                "name": "Kimi K3",
                "url": "https://api.example.com/v1/responses",
                "contextWindow": 262144,
                "maxOutputTokens": 32768,
                "toolCalling": true,
                "thinking": true,
                "editTools": ["apply-patch", "unsupported"],
                "supportsReasoningEffort": ["low", "high"],
                "reasoningEffortFormat": "responses",
                "zeroDataRetentionEnabled": true
            }]
        });

        let parsed = parse_group("stable:default", &group).expect("parse group");
        let model = &parsed.models[0];
        assert_eq!(model.api_key, "${input:chat.lm.secret.test}");
        assert_eq!(model.api_type, "responses");
        assert_eq!(model.edit_tools, vec!["apply-patch"]);
        assert_eq!(model.supports_reasoning_effort, vec!["high", "low"]);
        assert!(model.zero_data_retention_enabled);
    }

    #[test]
    fn rejects_group_when_any_model_cannot_be_represented() {
        let group = json!({
            "name": "Incomplete",
            "vendor": "customendpoint",
            "apiKey": "secret",
            "models": [{"id": "missing-url"}]
        });
        assert!(parse_group("stable:default", &group).is_err());
    }

    #[test]
    fn equivalent_existing_models_are_reused() {
        let group = parse_group(
            "stable:default",
            &json!({
                "name": "Existing",
                "vendor": "customendpoint",
                "apiKey": "secret",
                "models": [{
                    "id": "model-a",
                    "name": "Model A",
                    "url": "https://api.example.com/v1/chat/completions"
                }]
            }),
        )
        .expect("parse group");
        let mut store = CopilotByokStore::default();
        store.models.push(group.models[0].clone());

        let (imported, reused) = add_group_models(&mut store, group).expect("reuse group");
        assert_eq!(imported, 0);
        assert_eq!(reused, 1);
        assert_eq!(store.models.len(), 1);
    }
}
