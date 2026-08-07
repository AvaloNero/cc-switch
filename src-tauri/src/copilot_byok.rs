mod import;
mod model;
mod store;
mod sync;
mod vscode;

pub use import::CopilotByokImportResult;
pub use model::CopilotByokGroup;
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
pub struct CopilotByokState {
    pub groups: Vec<CopilotByokGroup>,
    pub targets: Vec<CopilotByokTargetState>,
    pub selected_target_ids: Vec<String>,
    pub managed_model_count: usize,
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
    let available_ids: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(store.custom_targets.iter().map(|target| target.id.clone()))
        .collect();
    let mut selected_target_ids = sync::effective_selected_target_ids(&store, &detected);
    selected_target_ids.retain(|id| available_ids.contains(id));
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
        managed_model_count: store
            .groups
            .iter()
            .map(CopilotByokGroup::enabled_model_count)
            .sum(),
        groups: store.groups,
        targets,
        selected_target_ids,
    })
}

fn sync_if_selected(store: &CopilotByokStore) -> Result<(), AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let available_ids: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(store.custom_targets.iter().map(|target| target.id.clone()))
        .collect();
    let selected_target_ids: Vec<String> = sync::effective_selected_target_ids(store, &detected)
        .into_iter()
        .filter(|id| available_ids.contains(id))
        .collect();
    if selected_target_ids.is_empty() {
        return Ok(());
    }

    let mut available_store = store.clone();
    available_store.targets_initialized = true;
    available_store.selected_target_ids = selected_target_ids;
    sync::sync_store(&available_store)?;
    Ok(())
}

fn commit_and_build(
    current: &CopilotByokStore,
    updated: &CopilotByokStore,
    overrides: sync::TransactionOverrides,
) -> Result<CopilotByokState, AppError> {
    sync::commit_store_update(current, updated, overrides, true)?;
    build_state(store::load_store()?)
}

pub fn get_state() -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(store::load_store()?)
}

pub fn sync_selected_on_startup() -> Result<(), AppError> {
    let _guard = operation_guard()?;
    let store = store::load_store()?;
    sync_if_selected(&store)
}

/// 返回会话用量导入所需的 BYOK 目录。调用方只使用供应商与模型元数据，
/// 不会复制或记录 API Key。
pub(crate) fn usage_catalog() -> Result<Vec<CopilotByokGroup>, AppError> {
    let _guard = operation_guard()?;
    Ok(store::load_store()?.groups)
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

    let mut updated = current.clone();
    updated.targets_initialized = true;
    updated.selected_target_ids = target_ids;
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn add_custom_target(path: String, name: Option<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let custom = CopilotByokCustomTarget::from_path(path, name)?;
    let custom_key = store::path_identity_key(Path::new(&custom.language_models_path));
    if vscode::discover_vscode_targets()?
        .iter()
        .any(|target| store::path_identity_key(&target.path()) == custom_key)
    {
        return Err(AppError::InvalidInput(
            "This VS Code profile is already available as a detected sync target".to_string(),
        ));
    }
    let mut updated = current.clone();
    updated.targets_initialized = true;
    if let Some(existing) = updated
        .custom_targets
        .iter_mut()
        .find(|target| target.id == custom.id)
    {
        *existing = custom.clone();
    } else {
        updated.custom_targets.push(custom.clone());
    }
    if !updated.selected_target_ids.contains(&custom.id) {
        updated.selected_target_ids.push(custom.id);
    }
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn remove_custom_target(target_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let mut updated = current.clone();
    updated
        .custom_targets
        .retain(|target| target.id != target_id);
    updated.selected_target_ids.retain(|id| id != target_id);
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn upsert_group(mut group: CopilotByokGroup) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    group.normalize();
    group.validate()?;
    let current = store::load_store()?;
    let mut updated = current.clone();
    if let Some(existing) = updated.groups.iter_mut().find(|item| item.id == group.id) {
        *existing = group;
    } else {
        updated.groups.push(group);
    }
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn delete_group(group_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let mut updated = current.clone();
    updated.groups.retain(|group| group.id != group_id);
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn reorder_groups(group_ids: Vec<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let mut updated = current.clone();
    store::apply_group_order(&mut updated.groups, &group_ids)?;
    commit_and_build(&current, &updated, sync::TransactionOverrides::default())
}

pub fn import_models(target_id: &str) -> Result<CopilotByokImportResult, AppError> {
    let _guard = operation_guard()?;
    import::import_from_target(store::load_store()?, target_id)
}

pub fn sync() -> Result<CopilotByokSyncResult, AppError> {
    let _guard = operation_guard()?;
    let store = store::load_store()?;
    sync::sync_store(&store)
}

pub fn restore_backup(target_id: &str) -> Result<bool, AppError> {
    let _guard = operation_guard()?;
    let current = store::load_store()?;
    let detected = vscode::discover_vscode_targets()?;
    let (_, target_path) = sync::resolve_target_paths(&current, &[target_id.to_string()])?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "Unknown or unavailable Copilot BYOK target: {target_id}"
            ))
        })?;
    let target_identity = store::path_identity_key(&target_path);
    let alias_ids: HashSet<String> = detected
        .iter()
        .filter(|target| store::path_identity_key(&target.path()) == target_identity)
        .map(|target| target.id.clone())
        .chain(
            current
                .custom_targets
                .iter()
                .filter(|target| {
                    store::path_identity_key(Path::new(&target.language_models_path))
                        == target_identity
                })
                .map(|target| target.id.clone()),
        )
        .collect();
    let selected = sync::effective_selected_target_ids(&current, &detected);
    let was_selected = selected.iter().any(|id| alias_ids.contains(id));
    let mut updated = current.clone();
    updated.targets_initialized = true;
    updated
        .selected_target_ids
        .retain(|id| !alias_ids.contains(id));
    let overrides = sync::TransactionOverrides {
        restore_targets: [target_id.to_string()].into_iter().collect(),
        ..sync::TransactionOverrides::default()
    };
    let result = sync::commit_store_update(&current, &updated, overrides, true)?;
    Ok(was_selected || result.changed_target_count > 0)
}
