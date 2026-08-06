use super::model::CopilotByokModel;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const STORE_FILE: &str = "copilot-byok.json";
const STORE_VERSION: u32 = 1;
const MAX_TARGETS: usize = 64;

fn default_store_version() -> u32 {
    STORE_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokCustomTarget {
    pub id: String,
    pub name: String,
    pub language_models_path: String,
}

impl CopilotByokCustomTarget {
    pub fn from_path(path: impl AsRef<Path>, name: Option<String>) -> Result<Self, AppError> {
        let path = path.as_ref();
        validate_language_models_path(path)?;
        let normalized = path.to_string_lossy().to_string();
        Ok(Self {
            id: custom_target_id(&normalized),
            name: name
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "Custom VS Code profile".to_string()),
            language_models_path: normalized,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokStore {
    #[serde(default = "default_store_version")]
    pub version: u32,
    #[serde(default)]
    pub selected_target_ids: Vec<String>,
    #[serde(default)]
    pub custom_targets: Vec<CopilotByokCustomTarget>,
    #[serde(default)]
    pub models: Vec<CopilotByokModel>,
}

impl Default for CopilotByokStore {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            selected_target_ids: Vec::new(),
            custom_targets: Vec::new(),
            models: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyStore {
    #[serde(default)]
    config_path: Option<String>,
    #[serde(default)]
    models: Vec<CopilotByokModel>,
}

fn custom_target_id(path: &str) -> String {
    let digest = Sha256::digest(path.as_bytes());
    format!("custom:{:x}", digest)[..19].to_string()
}

pub fn validate_language_models_path(path: &Path) -> Result<(), AppError> {
    if path.file_name().and_then(|value| value.to_str()) != Some("chatLanguageModels.json") {
        return Err(AppError::InvalidInput(
            "Custom Copilot BYOK target must end with chatLanguageModels.json".to_string(),
        ));
    }
    if !path.is_absolute() {
        return Err(AppError::InvalidInput(
            "Custom Copilot BYOK target must be an absolute path".to_string(),
        ));
    }
    Ok(())
}

fn normalize_store(store: &mut CopilotByokStore) -> Result<(), AppError> {
    store.version = STORE_VERSION;

    let mut selected = HashSet::new();
    store.selected_target_ids.retain(|id| {
        let trimmed = id.trim();
        !trimmed.is_empty() && selected.insert(trimmed.to_string())
    });
    for id in &mut store.selected_target_ids {
        *id = id.trim().to_string();
    }

    let mut target_ids = HashSet::new();
    store.custom_targets.retain(|target| {
        !target.id.trim().is_empty() && target_ids.insert(target.id.trim().to_string())
    });
    for target in &mut store.custom_targets {
        target.id = target.id.trim().to_string();
        target.name = target.name.trim().to_string();
        target.language_models_path = target.language_models_path.trim().to_string();
        validate_language_models_path(Path::new(&target.language_models_path))?;
    }

    if store.selected_target_ids.len() > MAX_TARGETS || store.custom_targets.len() > MAX_TARGETS {
        return Err(AppError::InvalidInput(format!(
            "Copilot BYOK supports at most {MAX_TARGETS} selected or custom targets"
        )));
    }

    let mut model_ids = HashSet::new();
    for model in &mut store.models {
        model.normalize();
        model.validate()?;
        if !model_ids.insert(model.id.clone()) {
            return Err(AppError::InvalidInput(format!(
                "Duplicate Copilot BYOK model id: {}",
                model.id
            )));
        }
    }
    Ok(())
}

pub(crate) fn parse_store_value(value: Value) -> Result<CopilotByokStore, AppError> {
    let mut store = if value.get("version").is_some()
        || value.get("selectedTargetIds").is_some()
        || value.get("customTargets").is_some()
    {
        serde_json::from_value(value).map_err(|error| {
            AppError::Config(format!("Failed to parse Copilot BYOK store: {error}"))
        })?
    } else {
        let legacy: LegacyStore = serde_json::from_value(value).map_err(|error| {
            AppError::Config(format!(
                "Failed to parse legacy Copilot BYOK store: {error}"
            ))
        })?;
        let mut migrated = CopilotByokStore {
            models: legacy.models,
            ..CopilotByokStore::default()
        };
        if let Some(path) = legacy
            .config_path
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            let custom = CopilotByokCustomTarget::from_path(&path, Some("Migrated target".into()))?;
            migrated.selected_target_ids.push(custom.id.clone());
            migrated.custom_targets.push(custom);
        }
        migrated
    };

    normalize_store(&mut store)?;
    Ok(store)
}

pub fn store_path() -> PathBuf {
    crate::config::get_app_config_dir().join(STORE_FILE)
}

pub fn load_store() -> Result<CopilotByokStore, AppError> {
    let path = store_path();
    if !path.exists() {
        return Ok(CopilotByokStore::default());
    }
    let text = fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    let value = serde_json::from_str(&text).map_err(|error| AppError::json(&path, error))?;
    parse_store_value(value)
}

pub fn save_store(store: &CopilotByokStore) -> Result<(), AppError> {
    let mut normalized = store.clone();
    normalize_store(&mut normalized)?;
    let path = store_path();
    crate::config::write_json_file(&path, &normalized)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| AppError::io(&path, error))?;
    }
    Ok(())
}

pub fn set_selected_targets(target_ids: Vec<String>) -> Result<CopilotByokStore, AppError> {
    let mut store = load_store()?;
    store.selected_target_ids = target_ids;
    save_store(&store)?;
    load_store()
}

pub fn add_custom_target(path: String, name: Option<String>) -> Result<CopilotByokStore, AppError> {
    let custom = CopilotByokCustomTarget::from_path(path, name)?;
    let mut store = load_store()?;
    if let Some(existing) = store
        .custom_targets
        .iter_mut()
        .find(|target| target.id == custom.id)
    {
        *existing = custom.clone();
    } else {
        store.custom_targets.push(custom.clone());
    }
    if !store.selected_target_ids.contains(&custom.id) {
        store.selected_target_ids.push(custom.id);
    }
    save_store(&store)?;
    load_store()
}

pub fn remove_custom_target(target_id: &str) -> Result<CopilotByokStore, AppError> {
    let mut store = load_store()?;
    store.custom_targets.retain(|target| target.id != target_id);
    store.selected_target_ids.retain(|id| id != target_id);
    save_store(&store)?;
    load_store()
}

pub fn upsert_model(mut model: CopilotByokModel) -> Result<CopilotByokStore, AppError> {
    model.normalize();
    model.validate()?;
    let mut store = load_store()?;
    if let Some(existing) = store.models.iter_mut().find(|item| item.id == model.id) {
        *existing = model;
    } else {
        store.models.push(model);
    }
    save_store(&store)?;
    load_store()
}

pub fn delete_model(model_id: &str) -> Result<CopilotByokStore, AppError> {
    let mut store = load_store()?;
    store.models.retain(|model| model.id != model_id);
    save_store(&store)?;
    load_store()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrates_legacy_single_path_store() {
        let temp = tempfile::tempdir().expect("temp directory");
        let path = temp.path().join("chatLanguageModels.json");
        let store = parse_store_value(json!({
            "configPath": path,
            "models": []
        }))
        .expect("migrate store");

        assert_eq!(store.version, STORE_VERSION);
        assert_eq!(store.custom_targets.len(), 1);
        assert_eq!(
            store.selected_target_ids,
            vec![store.custom_targets[0].id.clone()]
        );
    }

    #[test]
    fn rejects_custom_non_language_model_file() {
        let result = CopilotByokCustomTarget::from_path("/tmp/settings.json", None);
        assert!(result.is_err());
    }

    #[test]
    fn selected_target_ids_are_deduplicated() {
        let store = parse_store_value(json!({
            "version": 1,
            "selectedTargetIds": ["stable:default", "stable:default"],
            "customTargets": [],
            "models": []
        }))
        .expect("parse store");
        assert_eq!(store.selected_target_ids, vec!["stable:default"]);
    }
}
