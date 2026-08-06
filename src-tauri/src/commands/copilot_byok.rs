use crate::copilot_byok::{self, CopilotByokModel, CopilotByokState, CopilotByokSyncResult};

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
pub fn copilot_byok_upsert_model(model: CopilotByokModel) -> Result<CopilotByokState, String> {
    copilot_byok::upsert_model(model).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_delete_model(model_id: String) -> Result<CopilotByokState, String> {
    copilot_byok::delete_model(&model_id).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_sync(target_id: Option<String>) -> Result<serde_json::Value, String> {
    match target_id {
        Some(target_id) => {
            let result = copilot_byok::import_models(&target_id).map_err(String::from)?;
            serde_json::to_value(result).map_err(|error| error.to_string())
        }
        None => {
            let result: CopilotByokSyncResult = copilot_byok::sync().map_err(String::from)?;
            serde_json::to_value(result).map_err(|error| error.to_string())
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_remove_managed_models(
    target_ids: Option<Vec<String>>,
) -> Result<CopilotByokSyncResult, String> {
    copilot_byok::remove_managed_models(target_ids).map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub fn copilot_byok_restore_backup(target_id: String) -> Result<bool, String> {
    copilot_byok::restore_backup(&target_id).map_err(Into::into)
}
