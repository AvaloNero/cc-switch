use crate::copilot_byok::{self, CopilotByokModel, CopilotByokState, CopilotByokSyncResult};

#[tauri::command]
pub async fn copilot_byok_get_state() -> Result<CopilotByokState, String> {
    copilot_byok::get_state().await.map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_set_targets(
    target_ids: Vec<String>,
) -> Result<CopilotByokState, String> {
    copilot_byok::set_targets(target_ids)
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_add_custom_target(
    path: String,
    name: Option<String>,
) -> Result<CopilotByokState, String> {
    copilot_byok::add_custom_target(path, name)
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_remove_custom_target(
    target_id: String,
) -> Result<CopilotByokState, String> {
    copilot_byok::remove_custom_target(&target_id)
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_upsert_model(
    model: CopilotByokModel,
) -> Result<CopilotByokState, String> {
    copilot_byok::upsert_model(model).await.map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_delete_model(
    model_id: String,
) -> Result<CopilotByokState, String> {
    copilot_byok::delete_model(&model_id)
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_sync(
    target_id: Option<String>,
) -> Result<CopilotByokSyncResult, String> {
    copilot_byok::sync(target_id).await.map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_remove_managed_models(
    target_ids: Option<Vec<String>>,
) -> Result<CopilotByokSyncResult, String> {
    copilot_byok::remove_managed_models(target_ids)
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_byok_restore_backup(target_id: String) -> Result<bool, String> {
    copilot_byok::restore_backup(&target_id)
        .await
        .map_err(Into::into)
}
