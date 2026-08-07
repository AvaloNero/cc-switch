use crate::copilot_byok::{
    self, CopilotByokGroup, CopilotByokImportResult, CopilotByokState, CopilotByokSyncResult,
};
use crate::services::stream_check::{StreamCheckResult, StreamCheckService};
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub fn copilot_byok_get_state() -> Result<CopilotByokState, String> {
    copilot_byok::get_state().map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_set_targets(target_ids: Vec<String>) -> Result<CopilotByokState, String> {
    copilot_byok::set_targets(target_ids).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_add_custom_target(
    path: String,
    name: Option<String>,
) -> Result<CopilotByokState, String> {
    copilot_byok::add_custom_target(path, name).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_remove_custom_target(target_id: String) -> Result<CopilotByokState, String> {
    copilot_byok::remove_custom_target(&target_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_upsert_group(group: CopilotByokGroup) -> Result<CopilotByokState, String> {
    copilot_byok::upsert_group(group).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_delete_group(group_id: String) -> Result<CopilotByokState, String> {
    copilot_byok::delete_group(&group_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_reorder_groups(group_ids: Vec<String>) -> Result<CopilotByokState, String> {
    copilot_byok::reorder_groups(group_ids).map_err(Into::into)
}

#[tauri::command]
pub fn copilot_byok_sync() -> Result<CopilotByokSyncResult, String> {
    copilot_byok::sync().map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_import_models(target_id: String) -> Result<CopilotByokImportResult, String> {
    copilot_byok::import_models(&target_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_restore_backup(target_id: String) -> Result<bool, String> {
    copilot_byok::restore_backup(&target_id).map_err(Into::into)
}

/// 只检查供应商 Base URL 是否可达，不调用模型列表或真实推理接口。
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_check_connection(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<StreamCheckResult, String> {
    let group = copilot_byok::usage_catalog()
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
