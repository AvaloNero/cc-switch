use crate::app_config::AppType;
use crate::copilot_byok::{
    self, CopilotByokGroup, CopilotByokImportResult, CopilotByokState, CopilotByokSyncResult,
};
use crate::services::stream_check::{StreamCheckResult, StreamCheckService};
use crate::services::{McpService, PromptService};
use crate::store::AppState;
use tauri::State;

fn reproject_shared_resources(state: &AppState) -> Result<(), String> {
    McpService::sync_enabled_for_app(state, &AppType::CopilotByok).map_err(String::from)?;

    let prompts =
        PromptService::get_prompts(state, AppType::CopilotByok).map_err(String::from)?;
    if let Some(prompt) = prompts.values().find(|prompt| prompt.enabled) {
        PromptService::upsert_prompt(
            state,
            AppType::CopilotByok,
            &prompt.id,
            prompt.clone(),
        )
        .map_err(String::from)?;
    }

    Ok(())
}

#[tauri::command]
pub fn copilot_byok_get_state(state: State<'_, AppState>) -> Result<CopilotByokState, String> {
    copilot_byok::get_state(state.db.as_ref()).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_set_targets(
    state: State<'_, AppState>,
    target_ids: Vec<String>,
) -> Result<CopilotByokState, String> {
    let updated = copilot_byok::set_targets(state.db.as_ref(), target_ids).map_err(String::from)?;
    reproject_shared_resources(state.inner())?;
    Ok(updated)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_add_custom_target(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> Result<CopilotByokState, String> {
    let updated =
        copilot_byok::add_custom_target(state.db.as_ref(), path, name).map_err(String::from)?;
    reproject_shared_resources(state.inner())?;
    Ok(updated)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_remove_custom_target(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<CopilotByokState, String> {
    let updated = copilot_byok::remove_custom_target(state.db.as_ref(), &target_id)
        .map_err(String::from)?;
    reproject_shared_resources(state.inner())?;
    Ok(updated)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_upsert_group(
    state: State<'_, AppState>,
    group: CopilotByokGroup,
) -> Result<CopilotByokState, String> {
    copilot_byok::upsert_group(state.db.as_ref(), group).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_delete_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<CopilotByokState, String> {
    copilot_byok::delete_group(state.db.as_ref(), &group_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_reorder_groups(
    state: State<'_, AppState>,
    group_ids: Vec<String>,
) -> Result<CopilotByokState, String> {
    copilot_byok::reorder_groups(state.db.as_ref(), group_ids).map_err(Into::into)
}

#[tauri::command]
pub fn copilot_byok_sync(state: State<'_, AppState>) -> Result<CopilotByokSyncResult, String> {
    copilot_byok::sync(state.db.as_ref()).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_import_models(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<CopilotByokImportResult, String> {
    copilot_byok::import_models(state.db.as_ref(), &target_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_restore_backup(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<bool, String> {
    copilot_byok::restore_backup(state.db.as_ref(), &target_id).map_err(Into::into)
}

/// 只检查供应商 Base URL 是否可达，不调用模型列表或真实推理接口。
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_check_connection(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<StreamCheckResult, String> {
    let group = copilot_byok::usage_catalog(state.db.as_ref())
        .map_err(String::from)?
        .into_iter()
        .find(|group| group.id == group_id)
        .ok_or_else(|| format!("VS Code Copilot provider {group_id} does not exist"))?;
    let config = state.db.get_stream_check_config().map_err(String::from)?;
    let result = StreamCheckService::check_url_with_retry(&group.url, &config)
        .await
        .map_err(String::from)?;
    let _ = state
        .db
        .save_stream_check_log(&group.id, &group.name, "copilot-byok", &result);
    Ok(result)
}
