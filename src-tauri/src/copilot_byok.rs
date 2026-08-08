mod import;
mod model;
mod store;
mod sync;
mod vscode;

pub use import::CopilotByokImportResult;
pub use model::CopilotByokGroup;
pub use sync::CopilotByokSyncResult;
pub use vscode::{VsCodeEdition, VsCodeProfileTarget};

use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use model::is_managed_group;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use store::{CopilotByokCustomTarget, CopilotByokStore};

static OPERATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
// Usage statistics intentionally use `copilot-byok` with one normalized
// provider row. Keep the portable BYOK catalog in its own provider namespace
// so statistics and configuration can never overwrite or parse each other.
const CATALOG_APP_TYPE: &str = "copilot-byok-catalog";
const LEGACY_CATALOG_APP_TYPE: &str = "copilot-byok";

fn group_to_provider(group: &CopilotByokGroup, sort_index: usize) -> Result<Provider, AppError> {
    Ok(Provider {
        id: group.id.clone(),
        name: group.name.clone(),
        settings_config: serde_json::to_value(group)
            .map_err(|error| AppError::JsonSerialize { source: error })?,
        website_url: group.website_url.clone(),
        category: Some("custom".to_string()),
        created_at: None,
        sort_index: Some(sort_index),
        notes: group.notes.clone(),
        meta: None,
        icon: group.icon.clone(),
        icon_color: group.icon_color.clone(),
        in_failover_queue: false,
    })
}

fn provider_to_group(provider: &Provider) -> Result<CopilotByokGroup, AppError> {
    let mut group: CopilotByokGroup = serde_json::from_value(provider.settings_config.clone())
        .map_err(|error| {
            AppError::Config(format!(
                "Failed to parse VS Code Copilot provider '{}': {error}",
                provider.id
            ))
        })?;
    group.id = provider.id.clone();
    group.name = provider.name.clone();
    group.website_url = provider.website_url.clone();
    group.notes = provider.notes.clone();
    group.icon = provider.icon.clone();
    group.icon_color = provider.icon_color.clone();
    group.normalize();
    group.validate()?;
    Ok(group)
}

fn migrate_legacy_database_catalog(db: &Database) -> Result<(), AppError> {
    let legacy = db.get_all_providers(LEGACY_CATALOG_APP_TYPE)?;
    let existing = db.get_all_providers(CATALOG_APP_TYPE)?;
    let mut next_sort = existing.len();

    for provider in legacy.values() {
        let Ok(group) = provider_to_group(provider) else {
            // The normalized statistics provider has a deliberately different
            // settings shape and remains in the live application namespace.
            continue;
        };
        if !existing.contains_key(&group.id) {
            db.save_provider(CATALOG_APP_TYPE, &group_to_provider(&group, next_sort)?)?;
            next_sort += 1;
        }
        db.delete_provider(LEGACY_CATALOG_APP_TYPE, &group.id)?;
    }
    Ok(())
}

fn load_catalog(db: &Database) -> Result<Vec<CopilotByokGroup>, AppError> {
    migrate_legacy_database_catalog(db)?;
    db.get_all_providers(CATALOG_APP_TYPE)?
        .values()
        .map(provider_to_group)
        .collect()
}

fn persist_catalog(db: &Database, groups: &[CopilotByokGroup]) -> Result<(), AppError> {
    let providers = groups
        .iter()
        .enumerate()
        .map(|(sort_index, group)| group_to_provider(group, sort_index))
        .collect::<Result<Vec<_>, _>>()?;
    db.replace_provider_catalog(CATALOG_APP_TYPE, &providers)
}

fn load_runtime_store(db: &Database) -> Result<CopilotByokStore, AppError> {
    let mut local = store::load_store()?;
    if !local.groups.is_empty() {
        let existing = db.get_all_providers(CATALOG_APP_TYPE)?;
        let mut next_sort = existing.len();
        for group in &local.groups {
            if !existing.contains_key(&group.id) {
                db.save_provider(CATALOG_APP_TYPE, &group_to_provider(group, next_sort)?)?;
                next_sort += 1;
            }
        }
        local.groups.clear();
        store::save_device_store(&local)?;
    }
    local.groups = load_catalog(db)?;
    store::normalize_store(&mut local)?;
    Ok(local)
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

/// Resolve the device-local VS Code profile targets currently selected for
/// Copilot management. Provider catalog data is portable, while these paths
/// intentionally remain local to this device.
pub(crate) fn selected_language_model_paths() -> Result<Vec<PathBuf>, AppError> {
    selected_resource_paths(sync::TargetResource::LanguageModels)
}

pub(crate) fn selected_prompt_homes() -> Result<Vec<PathBuf>, AppError> {
    selected_resource_paths(sync::TargetResource::PromptsHome)
}

pub(crate) fn selected_mcp_paths() -> Result<Vec<PathBuf>, AppError> {
    selected_resource_paths(sync::TargetResource::Mcp)
}

fn selected_resource_paths(resource: sync::TargetResource) -> Result<Vec<PathBuf>, AppError> {
    let store = store::load_store()?;
    let discovered = vscode::discover_vscode_targets()?;
    let selected_ids = sync::effective_selected_target_ids(&store, &discovered);
    sync::resolve_resource_paths_from_discovered(&store, &selected_ids, resource, &discovered)
        .map(|targets| targets.into_iter().map(|(_, path)| path).collect())
}

pub(crate) fn primary_profile_config_dir() -> Result<PathBuf, AppError> {
    selected_language_model_paths()?
        .into_iter()
        .find_map(|path| path.parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            AppError::Config(
                "No VS Code Copilot sync target is selected on this device".to_string(),
            )
        })
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
        language_models_path: target.resources.language_models_path,
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
    db: &Database,
    current: &CopilotByokStore,
    updated: &CopilotByokStore,
    overrides: sync::TransactionOverrides,
) -> Result<CopilotByokState, AppError> {
    sync::commit_store_update(current, updated, overrides, true)?;
    build_state(load_runtime_store(db)?)
}

pub fn get_state(db: &Database) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    build_state(load_runtime_store(db)?)
}

pub fn sync_selected_on_startup(db: &Database) -> Result<(), AppError> {
    sync_if_configured(db)
}

/// Sync Copilot BYOK when this device has at least one available selected
/// target. Global synchronization calls this tolerant variant so users who do
/// not use VS Code Copilot do not block unrelated provider, MCP, or Skill
/// projections. Once a target is configured, real synchronization failures
/// are still propagated to the caller.
pub(crate) fn sync_if_configured(db: &Database) -> Result<(), AppError> {
    let _guard = operation_guard()?;
    let store = load_runtime_store(db)?;
    sync_if_selected(&store)
}

/// 返回会话用量导入所需的 BYOK 目录。调用方只使用供应商与模型元数据，
/// 不会复制或记录 API Key。
pub(crate) fn usage_catalog(db: &Database) -> Result<Vec<CopilotByokGroup>, AppError> {
    let _guard = operation_guard()?;
    load_catalog(db)
}

pub fn set_targets(db: &Database, target_ids: Vec<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
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
    commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    )
}

pub fn add_custom_target(
    db: &Database,
    path: String,
    name: Option<String>,
) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
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
    commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    )
}

pub fn remove_custom_target(db: &Database, target_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
    let mut updated = current.clone();
    updated
        .custom_targets
        .retain(|target| target.id != target_id);
    updated.selected_target_ids.retain(|id| id != target_id);
    commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    )
}

pub fn upsert_group(
    db: &Database,
    mut group: CopilotByokGroup,
) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    group.normalize();
    group.validate()?;
    let current = load_runtime_store(db)?;
    let previous_groups = current.groups.clone();
    let mut updated = current.clone();
    if let Some(existing) = updated.groups.iter_mut().find(|item| item.id == group.id) {
        *existing = group;
    } else {
        updated.groups.push(group);
    }
    persist_catalog(db, &updated.groups)?;
    match commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    ) {
        Ok(state) => Ok(state),
        Err(error) => {
            persist_catalog(db, &previous_groups).map_err(|rollback_error| {
                AppError::Config(format!(
                    "{error}; failed to roll back VS Code Copilot catalog: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

pub fn delete_group(db: &Database, group_id: &str) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
    let previous_groups = current.groups.clone();
    let mut updated = current.clone();
    updated.groups.retain(|group| group.id != group_id);
    persist_catalog(db, &updated.groups)?;
    match commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    ) {
        Ok(state) => Ok(state),
        Err(error) => {
            persist_catalog(db, &previous_groups).map_err(|rollback_error| {
                AppError::Config(format!(
                    "{error}; failed to roll back VS Code Copilot catalog: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

pub fn reorder_groups(db: &Database, group_ids: Vec<String>) -> Result<CopilotByokState, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
    let previous_groups = current.groups.clone();
    let mut updated = current.clone();
    store::apply_group_order(&mut updated.groups, &group_ids)?;
    persist_catalog(db, &updated.groups)?;
    match commit_and_build(
        db,
        &current,
        &updated,
        sync::TransactionOverrides::default(),
    ) {
        Ok(state) => Ok(state),
        Err(error) => {
            persist_catalog(db, &previous_groups).map_err(|rollback_error| {
                AppError::Config(format!(
                    "{error}; failed to roll back VS Code Copilot catalog: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

pub fn import_models(db: &Database, target_id: &str) -> Result<CopilotByokImportResult, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
    let prepared = import::prepare_import_from_target(current.clone(), target_id)?;
    if prepared.result.imported_group_count == 0 {
        return Ok(prepared.result);
    }

    persist_catalog(db, &prepared.updated_store.groups)?;
    match sync::commit_store_update(
        &prepared.original_store,
        &prepared.updated_store,
        prepared.overrides,
        true,
    ) {
        Ok(sync_result) => {
            let mut result = prepared.result;
            result.changed_target_count = sync_result.changed_target_count;
            Ok(result)
        }
        Err(error) => {
            persist_catalog(db, &current.groups).map_err(|rollback_error| {
                AppError::Config(format!(
                    "{error}; failed to roll back VS Code Copilot catalog: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

pub fn sync(db: &Database) -> Result<CopilotByokSyncResult, AppError> {
    let _guard = operation_guard()?;
    let store = load_runtime_store(db)?;
    sync::sync_store(&store)
}

pub fn restore_backup(db: &Database, target_id: &str) -> Result<bool, AppError> {
    let _guard = operation_guard()?;
    let current = load_runtime_store(db)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot_byok::model::CopilotByokModel;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn group(id: &str, name: &str) -> CopilotByokGroup {
        CopilotByokGroup {
            id: id.to_string(),
            name: name.to_string(),
            url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
            api_type: "chat-completions".to_string(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            enabled: true,
            request_headers: BTreeMap::new(),
            models: vec![CopilotByokModel {
                id: format!("{id}:model"),
                model_id: "model-1".to_string(),
                name: "Model 1".to_string(),
                enabled: true,
                tool_calling: None,
                vision: None,
                thinking: None,
                streaming: None,
                context_window: None,
                max_input_tokens: None,
                max_output_tokens: None,
                edit_tools: Vec::new(),
                zero_data_retention_enabled: false,
                supports_reasoning_effort: Vec::new(),
                reasoning_effort_format: None,
                model_options: json!({}),
                extra: BTreeMap::new(),
            }],
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn portable_catalog_round_trips_through_provider_database() -> Result<(), AppError> {
        let db = Database::memory()?;
        let first = group("first", "First");
        let second = group("second", "Second");

        let usage_provider = Provider::with_id(
            "vscode-copilot".to_string(),
            "VSCode Copilot".to_string(),
            json!({ "source": "vscode_session" }),
            None,
        );
        db.save_provider(LEGACY_CATALOG_APP_TYPE, &usage_provider)?;

        persist_catalog(&db, &[first.clone(), second.clone()])?;
        assert_eq!(load_catalog(&db)?, vec![first, second.clone()]);
        assert!(db
            .get_provider_by_id("vscode-copilot", LEGACY_CATALOG_APP_TYPE)?
            .is_some());

        persist_catalog(&db, std::slice::from_ref(&second))?;
        assert_eq!(load_catalog(&db)?, vec![second]);
        assert!(db.get_provider_by_id("first", CATALOG_APP_TYPE)?.is_none());
        Ok(())
    }

    #[test]
    fn legacy_catalog_rows_move_without_consuming_usage_provider() -> Result<(), AppError> {
        let db = Database::memory()?;
        let legacy_group = group("legacy", "Legacy");
        db.save_provider(
            LEGACY_CATALOG_APP_TYPE,
            &group_to_provider(&legacy_group, 0)?,
        )?;
        db.save_provider(
            LEGACY_CATALOG_APP_TYPE,
            &Provider::with_id(
                "vscode-copilot".to_string(),
                "VSCode Copilot".to_string(),
                json!({ "source": "vscode_session" }),
                None,
            ),
        )?;

        assert_eq!(load_catalog(&db)?, vec![legacy_group]);
        assert!(db
            .get_provider_by_id("legacy", LEGACY_CATALOG_APP_TYPE)?
            .is_none());
        assert!(db
            .get_provider_by_id("vscode-copilot", LEGACY_CATALOG_APP_TYPE)?
            .is_some());
        Ok(())
    }
}
