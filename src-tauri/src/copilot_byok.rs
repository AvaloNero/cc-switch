mod model;
mod store;
mod sync;
mod vscode;

pub use model::CopilotByokModel;
pub use sync::CopilotByokSyncResult;
pub use vscode::{VsCodeEdition, VsCodeProfileTarget};

use crate::error::AppError;
use model::is_managed_group;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use store::{CopilotByokCustomTarget, CopilotByokStore};

static OPERATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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
pub struct CopilotByokSecurityNotice {
    pub api_keys_are_written_to_vscode_config: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotByokState {
    pub models: Vec<CopilotByokModel>,
    pub targets: Vec<CopilotByokTargetState>,
    pub selected_target_ids: Vec<String>,
    pub managed_model_count: usize,
    pub security_notice: CopilotByokSecurityNotice,
}

fn operation_guard() -> Result<MutexGuard<'static, ()>, AppError> {
    OPERATION_LOCK
        .lock()
        .map_err(|error| AppError::Lock(error.to_string()))
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.cc-switch.bak")
}

fn inspect_path(path: &Path) -> (usize, Option<String>) {
    match sync::read_language_model_groups(path) {
        Ok(groups) => (
            groups
                .iter()
                .filter(|group| is_managed_group(group))
                .count(),
            None,
        ),
        Err(error) => (0, Some(error.to_string())),
    }
}

fn detected_target_state(
    target: VsCodeProfileTarget,
    selected_ids: &HashSet<String>,
) -> CopilotByokTargetState {
    let path = target.path();
    let (managed_group_count, read_error) = inspect_path(&path);
    CopilotByokTargetState {
        selected: selected_ids.contains(&target.id),
        id: target.id,
        source: "detected".to_string(),
        edition: Some(target.edition),
        edition_name: Some(target.edition_name),
        profile_id: target.profile_id,
        profile_name: target.profile_name,
        is_default: target.is_default,
        language_models_path: target.language_models_path,
        config_exists: target.config_exists,
        backup_exists: target.backup_exists,
        managed_group_count,
        read_error,
    }
}

fn custom_target_state(
    target: &CopilotByokCustomTarget,
    selected_ids: &HashSet<String>,
) -> CopilotByokTargetState {
    let path = PathBuf::from(&target.language_models_path);
    let (managed_group_count, read_error) = inspect_path(&path);
    CopilotByokTargetState {
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
        selected: selected_ids.contains(&target.id),
        managed_group_count,
        read_error,
    }
}

fn build_state(store: CopilotByokStore) -> Result<CopilotByokState, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let selected_target_ids = sync::effective_selected_target_ids(&store, &detected);
    let selected_ids: HashSet<String> = selected_target_ids.iter().cloned().collect();

    let mut targets: Vec<CopilotByokTargetState> = detected
        .into_iter()
        .map(|target| detected_target_state(target, &selected_ids))
        .collect();
    targets.extend(
        store
            .custom_targets
            .iter()
            .map(|target| custom_target_state(target, &selected_ids)),
    );
    targets.sort_by(|left, right| {
        right
            .selected
            .cmp(&left.selected)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.profile_name.cmp(&right.profile_name))
    });

    Ok(CopilotByokState {
        managed_model_count: store.models.iter().filter(|model| model.enabled).count(),
        models: store.models,
        targets,
        selected_target_ids,
        security_notice: CopilotByokSecurityNotice {
            api_keys_are_written_to_vscode_config: true,
            message: "VS Code Custom Endpoint BYOK stores the configured API key in chatLanguageModels.json. CC Switch restricts its own store file on Unix, but VS Code does not expose SecretStorage to external applications."
                .to_string(),
        },
    })
}

pub fn get_state() -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::load_store()?)
}

pub fn set_targets(target_ids: Vec<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let detected = vscode::discover_vscode_targets()?;
    let valid_ids: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(
            current
                .custom_targets
                .iter()
                .map(|target| target.id.clone()),
        )
        .collect();
    if let Some(invalid) = target_ids.iter().find(|id| !valid_ids.contains(*id)) {
        return Err(AppError::InvalidInput(format!(
            "Unknown or unavailable Copilot BYOK target: {invalid}"
        )));
    }
    build_state(store::set_selected_targets(target_ids)?)
}

pub fn add_custom_target(path: String, name: Option<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::add_custom_target(path, name)?)
}

pub fn remove_custom_target(target_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::remove_custom_target(target_id)?)
}

pub fn upsert_model(model: CopilotByokModel) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::upsert_model(model)?)
}

pub fn delete_model(model_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::delete_model(model_id)?)
}

pub fn sync() -> Result<CopilotByokSyncResult, AppError> {
    let _guard = operation_guard()?;
    let store = store::load_store()?;
    sync::sync_store(&store)
}

pub fn remove_managed_models(
    target_ids: Option<Vec<String>>,
) -> Result<CopilotByokSyncResult, AppError> {
    let _guard = operation_guard()?;
    let store = store::load_store()?;
    sync::remove_managed_groups(&store, target_ids)
}

pub fn restore_backup(target_id: &str) -> Result<bool, AppError> {
    let _guard = operation_guard()?;
    let store = store::load_store()?;
    sync::restore_backup(&store, target_id)
}
