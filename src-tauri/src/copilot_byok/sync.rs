use super::model::{is_managed_group, CopilotByokModel};
use super::store::{CopilotByokCustomTarget, CopilotByokStore};
use super::vscode::{discover_vscode_targets, VsCodeProfileTarget};
use crate::error::AppError;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokSyncResult {
    pub target_ids: Vec<String>,
    pub managed_model_count: usize,
    pub changed_target_count: usize,
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.cc-switch.bak")
}

fn ensure_regular_target(path: &Path) -> Result<(), AppError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput(format!(
                "Refusing to modify symlinked Copilot BYOK config: {}",
                path.display()
            )));
        }
        if !metadata.is_file() {
            return Err(AppError::InvalidInput(format!(
                "Copilot BYOK target is not a regular file: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn ensure_file_size(path: &Path) -> Result<(), AppError> {
    let metadata = fs::metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(AppError::InvalidInput(format!(
            "Copilot BYOK config exceeds {} MiB: {}",
            MAX_CONFIG_BYTES / 1024 / 1024,
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn read_language_model_groups(path: &Path) -> Result<Vec<Value>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    ensure_regular_target(path)?;
    ensure_file_size(path)?;

    let text = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = json5::from_str(&text).map_err(|error| {
        AppError::Config(format!(
            "Failed to parse VS Code language model config {}: {error}",
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

fn create_backup_once(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }
    let backup = backup_path(path);
    if backup.exists() {
        ensure_regular_target(&backup)?;
        ensure_file_size(&backup)?;
        return Ok(());
    }
    fs::copy(path, &backup).map_err(|error| AppError::io(&backup, error))?;
    Ok(())
}

fn write_groups(path: &Path, groups: &[Value]) -> Result<(), AppError> {
    ensure_regular_target(path)?;
    create_backup_once(path)?;
    crate::config::write_json_file(path, &Value::Array(groups.to_vec()))
}

pub(crate) fn merge_managed_groups(
    existing: Vec<Value>,
    models: &[CopilotByokModel],
) -> Vec<Value> {
    let mut merged: Vec<Value> = existing
        .into_iter()
        .filter(|group| !is_managed_group(group))
        .collect();
    merged.extend(
        models
            .iter()
            .filter(|model| model.enabled)
            .map(CopilotByokModel::to_language_model_group),
    );
    merged
}

fn custom_target_path(target: &CopilotByokCustomTarget) -> PathBuf {
    PathBuf::from(&target.language_models_path)
}

fn target_path_map(
    store: &CopilotByokStore,
    discovered: &[VsCodeProfileTarget],
) -> HashMap<String, PathBuf> {
    let mut paths = HashMap::new();
    for target in discovered {
        paths.insert(target.id.clone(), target.path());
    }
    for target in &store.custom_targets {
        paths.insert(target.id.clone(), custom_target_path(target));
    }
    paths
}

pub fn effective_selected_target_ids(
    store: &CopilotByokStore,
    discovered: &[VsCodeProfileTarget],
) -> Vec<String> {
    if store.targets_initialized {
        return store.selected_target_ids.clone();
    }

    discovered
        .iter()
        .find(|target| target.id == "stable:default")
        .or_else(|| discovered.first())
        .map(|target| vec![target.id.clone()])
        .unwrap_or_default()
}

fn resolve_target_paths(
    store: &CopilotByokStore,
    requested_ids: &[String],
) -> Result<Vec<(String, PathBuf)>, AppError> {
    let discovered = discover_vscode_targets()?;
    let paths = target_path_map(store, &discovered);
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();

    for id in requested_ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let path = paths.get(id).cloned().ok_or_else(|| {
            AppError::InvalidInput(format!("Unknown or unavailable Copilot BYOK target: {id}"))
        })?;
        resolved.push((id.clone(), path));
    }
    Ok(resolved)
}

pub fn sync_store(store: &CopilotByokStore) -> Result<CopilotByokSyncResult, AppError> {
    let discovered = discover_vscode_targets()?;
    let target_ids = effective_selected_target_ids(store, &discovered);
    if target_ids.is_empty() {
        return Err(AppError::InvalidInput(
            "No VS Code profile is selected for Copilot BYOK sync".to_string(),
        ));
    }

    let targets = resolve_target_paths(store, &target_ids)?;
    let mut changed_target_count = 0;
    for (_, path) in &targets {
        let existing = read_language_model_groups(path)?;
        let merged = merge_managed_groups(existing.clone(), &store.models);
        if merged != existing {
            write_groups(path, &merged)?;
            changed_target_count += 1;
        }
    }

    Ok(CopilotByokSyncResult {
        target_ids,
        managed_model_count: store.models.iter().filter(|model| model.enabled).count(),
        changed_target_count,
    })
}

pub fn remove_managed_groups(
    store: &CopilotByokStore,
    requested_ids: Option<Vec<String>>,
) -> Result<CopilotByokSyncResult, AppError> {
    let discovered = discover_vscode_targets()?;
    let target_ids = requested_ids
        .filter(|ids| !ids.is_empty())
        .unwrap_or_else(|| effective_selected_target_ids(store, &discovered));
    let targets = resolve_target_paths(store, &target_ids)?;
    let mut changed_target_count = 0;

    for (_, path) in &targets {
        if !path.exists() {
            continue;
        }
        let existing = read_language_model_groups(path)?;
        let unmanaged: Vec<Value> = existing
            .iter()
            .filter(|group| !is_managed_group(group))
            .cloned()
            .collect();
        if unmanaged == existing {
            continue;
        }

        if unmanaged.is_empty() && !backup_path(path).exists() {
            fs::remove_file(path).map_err(|error| AppError::io(path, error))?;
        } else {
            write_groups(path, &unmanaged)?;
        }
        changed_target_count += 1;
    }

    Ok(CopilotByokSyncResult {
        target_ids,
        managed_model_count: 0,
        changed_target_count,
    })
}

pub fn restore_backup(store: &CopilotByokStore, target_id: &str) -> Result<bool, AppError> {
    let targets = resolve_target_paths(store, &[target_id.to_string()])?;
    let (_, path) = targets
        .into_iter()
        .next()
        .ok_or_else(|| AppError::InvalidInput("Copilot BYOK target is required".to_string()))?;
    let backup = backup_path(&path);

    if backup.exists() {
        if path.exists() {
            ensure_regular_target(&path)?;
        }
        ensure_regular_target(&backup)?;
        ensure_file_size(&backup)?;
        let contents = fs::read(&backup).map_err(|error| AppError::io(&backup, error))?;
        crate::config::atomic_write(&path, &contents)?;
        return Ok(true);
    }

    let result = remove_managed_groups(store, Some(vec![target_id.to_string()]))?;
    Ok(result.changed_target_count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_preserves_unmanaged_groups() {
        let existing = vec![
            json!({"name": "User model", "vendor": "customendpoint"}),
            json!({"name": "CC Switch: Old", "vendor": "customendpoint"}),
        ];
        let merged = merge_managed_groups(existing, &[]);
        assert_eq!(
            merged,
            vec![json!({
                "name": "User model",
                "vendor": "customendpoint"
            })]
        );
    }

    fn target(id: &str, edition: super::super::vscode::VsCodeEdition) -> VsCodeProfileTarget {
        VsCodeProfileTarget {
            id: id.to_string(),
            edition,
            edition_name: id.to_string(),
            profile_id: None,
            profile_name: "Default".to_string(),
            is_default: true,
            user_dir: format!("/{id}"),
            language_models_path: format!("/{id}/chatLanguageModels.json"),
            config_exists: false,
            backup_exists: false,
        }
    }

    #[test]
    fn effective_selection_prefers_stable_default_before_initialization() {
        let store = CopilotByokStore::default();
        let targets = vec![
            target(
                "insiders:default",
                super::super::vscode::VsCodeEdition::Insiders,
            ),
            target(
                "stable:default",
                super::super::vscode::VsCodeEdition::Stable,
            ),
        ];
        assert_eq!(
            effective_selected_target_ids(&store, &targets),
            vec!["stable:default"]
        );
    }

    #[test]
    fn effective_selection_preserves_explicit_empty_selection() {
        let store = CopilotByokStore {
            targets_initialized: true,
            ..CopilotByokStore::default()
        };
        let targets = vec![target(
            "stable:default",
            super::super::vscode::VsCodeEdition::Stable,
        )];
        assert!(effective_selected_target_ids(&store, &targets).is_empty());
    }
}
