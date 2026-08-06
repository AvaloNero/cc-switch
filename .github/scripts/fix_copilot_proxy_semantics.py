from pathlib import Path

path = Path("src-tauri/src/copilot_byok/proxy.rs")
text = path.read_text(encoding="utf-8")

count_expr = "managed_model_count: usize::from(store.current_provider_id.is_some()),"
if text.count(count_expr) != 2:
    raise SystemExit("managed model count anchors changed")
text = text.replace(
    count_expr,
    "managed_model_count: if store.current_provider_id.is_some() { 1 } else { 0 },",
)

old_sync = '''pub async fn sync(provider_id: Option<String>) -> Result<CopilotByokSyncResult, AppError> {
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        if let Some(provider_id) = provider_id {
            if !store
                .providers
                .iter()
                .any(|provider| provider.id == provider_id && provider.enabled)
            {
                return Err(AppError::InvalidInput(format!(
                    "Unknown or disabled Copilot proxy provider: {provider_id}"
                )));
            }
            store.current_provider_id = Some(provider_id);
        }
        if current_provider(&store).is_none() {
            return Err(AppError::InvalidInput(
                "Select an enabled Copilot proxy provider first".to_string(),
            ));
        }
        if selected_ids_for_store(&store)?.is_empty() {
            return Err(AppError::InvalidInput(
                "Select at least one VS Code profile first".to_string(),
            ));
        }
        store.integration_enabled = true;
        save_store(&store)?;
        store
    };
    activate_store(&store).await
}
'''
new_sync = '''pub async fn sync(provider_id: Option<String>) -> Result<CopilotByokSyncResult, AppError> {
    let selecting_provider = provider_id.is_some();
    let store = {
        let _guard = operation_guard()?;
        let mut store = load_store()?;
        if let Some(provider_id) = provider_id {
            if !store
                .providers
                .iter()
                .any(|provider| provider.id == provider_id && provider.enabled)
            {
                return Err(AppError::InvalidInput(format!(
                    "Unknown or disabled Copilot proxy provider: {provider_id}"
                )));
            }
            store.current_provider_id = Some(provider_id);
        }
        if current_provider(&store).is_none() {
            return Err(AppError::InvalidInput(
                "Select an enabled Copilot proxy provider first".to_string(),
            ));
        }
        if !selecting_provider {
            if selected_ids_for_store(&store)?.is_empty() {
                return Err(AppError::InvalidInput(
                    "Select at least one VS Code profile first".to_string(),
                ));
            }
            store.integration_enabled = true;
        }
        save_store(&store)?;
        store
    };

    if store.integration_enabled {
        activate_store(&store).await
    } else {
        Ok(CopilotByokSyncResult {
            target_ids: selected_ids_for_store(&store)?,
            managed_model_count: 1,
            changed_target_count: 0,
            integration_enabled: false,
            server_running: SERVER_RUNNING.load(Ordering::SeqCst),
        })
    }
}
'''
if text.count(old_sync) != 1:
    raise SystemExit("sync function anchor changed")
text = text.replace(old_sync, new_sync, 1)

path.write_text(text, encoding="utf-8")
