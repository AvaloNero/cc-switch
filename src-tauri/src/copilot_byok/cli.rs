use super::model::{CopilotByokGroup, CopilotByokModel};
#[cfg(windows)]
use super::store;
use super::store::CopilotByokStore;
#[cfg(any(windows, test))]
use super::store::{CopilotCliConfig, CopilotCliManagedEnvironment};
use crate::error::AppError;
use serde::Serialize;
#[cfg(any(windows, test))]
use std::collections::BTreeMap;

#[cfg(any(windows, test))]
const MANAGED_VARIABLES: &[&str] = &[
    "COPILOT_PROVIDER_BASE_URL",
    "COPILOT_PROVIDER_TYPE",
    "COPILOT_PROVIDER_API_KEY",
    "COPILOT_PROVIDER_BEARER_TOKEN",
    "COPILOT_PROVIDER_WIRE_API",
    "COPILOT_PROVIDER_TRANSPORT",
    "COPILOT_PROVIDER_AZURE_API_VERSION",
    "COPILOT_PROVIDER_HEADERS",
    "COPILOT_MODEL",
    "COPILOT_PROVIDER_MODEL_ID",
    "COPILOT_PROVIDER_WIRE_MODEL",
    "COPILOT_PROVIDER_MAX_PROMPT_TOKENS",
    "COPILOT_PROVIDER_MAX_OUTPUT_TOKENS",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopilotCliState {
    pub supported: bool,
    pub enabled: bool,
    pub selected_group_id: Option<String>,
    pub selected_model_id: Option<String>,
    pub selected_provider_name: Option<String>,
    pub selected_model_name: Option<String>,
    pub environment_matches: bool,
    /// Variable names only. Secret values are never returned to the frontend.
    pub environment_conflicts: Vec<String>,
}

#[cfg(any(windows, test))]
trait UserEnvironment {
    fn read(&self, name: &str) -> Result<Option<String>, AppError>;
    fn write(&self, name: &str, value: Option<&str>) -> Result<(), AppError>;
    fn broadcast_change(&self) -> Result<(), AppError>;
}

#[cfg(windows)]
struct WindowsUserEnvironment;

#[cfg(windows)]
impl WindowsUserEnvironment {
    fn environment_key(&self) -> Result<winreg::RegKey, AppError> {
        use winreg::enums::HKEY_CURRENT_USER;
        winreg::RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey("Environment")
            .map(|(key, _)| key)
            .map_err(|error| AppError::IoContext {
                context: "Failed to open HKEY_CURRENT_USER\\Environment".to_string(),
                source: error,
            })
    }
}

#[cfg(windows)]
impl UserEnvironment for WindowsUserEnvironment {
    fn read(&self, name: &str) -> Result<Option<String>, AppError> {
        let key = self.environment_key()?;
        match key.get_value::<String, _>(name) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(AppError::IoContext {
                context: format!("Failed to read user environment variable {name}"),
                source: error,
            }),
        }
    }

    fn write(&self, name: &str, value: Option<&str>) -> Result<(), AppError> {
        let key = self.environment_key()?;
        match value {
            Some(value) => key.set_value(name, &value),
            None => match key.delete_value(name) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
        }
        .map_err(|error| AppError::IoContext {
            context: format!("Failed to update user environment variable {name}"),
            source: error,
        })
    }

    fn broadcast_change(&self) -> Result<(), AppError> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        };

        let environment: Vec<u16> = std::ffi::OsStr::new("Environment")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut result = 0_usize;
        let delivered = unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                environment.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5_000,
                &mut result,
            )
        };
        if delivered == 0 {
            return Err(AppError::Config(
                "Updated the registry but failed to broadcast the environment change".to_string(),
            ));
        }
        Ok(())
    }
}

fn selected<'a>(
    groups: &'a [CopilotByokGroup],
    group_id: &str,
    model_id: &str,
) -> Result<(&'a CopilotByokGroup, &'a CopilotByokModel), AppError> {
    let group = groups
        .iter()
        .find(|group| group.id == group_id)
        .ok_or_else(|| {
            AppError::InvalidInput(format!("Unknown Copilot CLI provider: {group_id}"))
        })?;
    let model = group
        .models
        .iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "Unknown Copilot CLI model {model_id} in provider {group_id}"
            ))
        })?;
    Ok((group, model))
}

#[cfg(any(windows, test))]
fn is_vscode_secret_reference(value: &str) -> bool {
    value
        .strip_prefix("${input:")
        .and_then(|value| value.strip_suffix('}'))
        .is_some_and(|key| !key.trim().is_empty())
}

#[cfg(any(windows, test))]
fn provider_base_url(raw: &str, api_type: &str) -> Result<String, AppError> {
    let mut parsed = url::Url::parse(raw).map_err(|error| {
        AppError::InvalidInput(format!("Invalid Copilot CLI provider URL: {error}"))
    })?;
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(AppError::InvalidInput(
            "Copilot CLI provider base URL must not contain a query or fragment".to_string(),
        ));
    }

    let path = parsed.path().trim_end_matches('/').to_string();
    let suffixes: &[&str] = match api_type {
        "chat-completions" => &["/chat/completions"],
        "responses" => &["/responses"],
        "messages" => &["/v1/messages", "/messages"],
        _ => &[],
    };
    if let Some(suffix) = suffixes.iter().find(|suffix| path.ends_with(**suffix)) {
        let base_path = &path[..path.len() - suffix.len()];
        parsed.set_path(if base_path.is_empty() { "/" } else { base_path });
    }

    let mut rendered = parsed.to_string();
    if parsed.path() == "/" {
        rendered = rendered.trim_end_matches('/').to_string();
    }
    Ok(rendered)
}

#[cfg(any(windows, test))]
fn format_headers(group: &CopilotByokGroup) -> Result<Option<String>, AppError> {
    if group.request_headers.is_empty() {
        return Ok(None);
    }
    let mut lines = Vec::with_capacity(group.request_headers.len());
    for (raw_name, raw_value) in &group.request_headers {
        let name = raw_name.trim();
        if name.is_empty() || name.contains([':', '\r', '\n']) || raw_value.contains(['\r', '\n']) {
            return Err(AppError::InvalidInput(format!(
                "Copilot CLI provider header is not representable: {raw_name}"
            )));
        }
        lines.push(format!(
            "{name}: {}",
            raw_value.replace("${apiKey}", &group.api_key)
        ));
    }
    Ok(Some(lines.join("\n")))
}

#[cfg(any(windows, test))]
fn desired_environment(
    group: &CopilotByokGroup,
    model: &CopilotByokModel,
) -> Result<BTreeMap<String, Option<String>>, AppError> {
    if is_vscode_secret_reference(&group.api_key) {
        return Err(AppError::InvalidInput(
            "Copilot CLI cannot resolve a VS Code SecretStorage ${input:...} reference; enter a literal API key or use an unauthenticated local provider"
                .to_string(),
        ));
    }
    let mut desired: BTreeMap<String, Option<String>> = MANAGED_VARIABLES
        .iter()
        .map(|name| ((*name).to_string(), None))
        .collect();
    desired.insert(
        "COPILOT_PROVIDER_BASE_URL".to_string(),
        Some(provider_base_url(&group.url, &group.api_type)?),
    );
    desired.insert(
        "COPILOT_PROVIDER_TYPE".to_string(),
        Some(if group.api_type == "messages" {
            "anthropic".to_string()
        } else {
            "openai".to_string()
        }),
    );
    if !group.api_key.is_empty() {
        desired.insert(
            "COPILOT_PROVIDER_API_KEY".to_string(),
            Some(group.api_key.clone()),
        );
    }
    desired.insert(
        "COPILOT_PROVIDER_WIRE_API".to_string(),
        match group.api_type.as_str() {
            "responses" => Some("responses".to_string()),
            "chat-completions" => Some("completions".to_string()),
            _ => None,
        },
    );
    desired.insert(
        "COPILOT_PROVIDER_HEADERS".to_string(),
        format_headers(group)?,
    );
    desired.insert("COPILOT_MODEL".to_string(), Some(model.model_id.clone()));
    desired.insert(
        "COPILOT_PROVIDER_MAX_PROMPT_TOKENS".to_string(),
        model.max_input_tokens.map(|value| value.to_string()),
    );
    desired.insert(
        "COPILOT_PROVIDER_MAX_OUTPUT_TOKENS".to_string(),
        model.max_output_tokens.map(|value| value.to_string()),
    );
    Ok(desired)
}

#[cfg(any(windows, test))]
fn snapshot(
    environment: &dyn UserEnvironment,
) -> Result<BTreeMap<String, Option<String>>, AppError> {
    MANAGED_VARIABLES
        .iter()
        .map(|name| Ok(((*name).to_string(), environment.read(name)?)))
        .collect()
}

#[cfg(any(windows, test))]
fn conflicts(
    current: &BTreeMap<String, Option<String>>,
    expected: &BTreeMap<String, Option<String>>,
) -> Vec<String> {
    MANAGED_VARIABLES
        .iter()
        .filter(|name| current.get(**name) != expected.get(**name))
        .map(|name| (*name).to_string())
        .collect()
}

#[cfg(any(windows, test))]
fn write_atomic(
    environment: &dyn UserEnvironment,
    values: &BTreeMap<String, Option<String>>,
) -> Result<BTreeMap<String, Option<String>>, AppError> {
    let before = snapshot(environment)?;
    for name in MANAGED_VARIABLES {
        let value = values.get(*name).and_then(Option::as_deref);
        if let Err(error) = environment.write(name, value) {
            let rollback_failures: Vec<String> = MANAGED_VARIABLES
                .iter()
                .filter_map(|rollback_name| {
                    environment
                        .write(
                            rollback_name,
                            before.get(*rollback_name).and_then(Option::as_deref),
                        )
                        .err()
                        .map(|_| (*rollback_name).to_string())
                })
                .collect();
            if rollback_failures.is_empty() {
                return Err(error);
            }
            return Err(AppError::Config(format!(
                "{error}; failed to roll back environment variables: {}",
                rollback_failures.join(", ")
            )));
        }
    }
    Ok(before)
}

#[cfg(any(windows, test))]
fn restore_snapshot(
    environment: &dyn UserEnvironment,
    values: &BTreeMap<String, Option<String>>,
) -> Result<(), AppError> {
    let failures: Vec<String> = MANAGED_VARIABLES
        .iter()
        .filter_map(|name| {
            environment
                .write(name, values.get(*name).and_then(Option::as_deref))
                .err()
                .map(|_| (*name).to_string())
        })
        .collect();
    if !failures.is_empty() {
        return Err(AppError::Config(format!(
            "Failed to restore environment variables: {}",
            failures.join(", ")
        )));
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn state_with_backend(
    store: &CopilotByokStore,
    groups: &[CopilotByokGroup],
    environment: &dyn UserEnvironment,
) -> Result<CopilotCliState, AppError> {
    let selected = match (
        store.cli.selected_group_id.as_deref(),
        store.cli.selected_model_id.as_deref(),
    ) {
        (Some(group_id), Some(model_id)) => selected(groups, group_id, model_id).ok(),
        _ => None,
    };
    let current = snapshot(environment)?;
    let environment_conflicts = if store.cli.enabled {
        conflicts(&current, &store.cli.managed_environment.last_written)
    } else {
        Vec::new()
    };
    let desired_matches_last_written = selected
        .and_then(|(group, model)| desired_environment(group, model).ok())
        .is_some_and(|desired| desired == store.cli.managed_environment.last_written);
    Ok(CopilotCliState {
        supported: true,
        enabled: store.cli.enabled,
        selected_group_id: store.cli.selected_group_id.clone(),
        selected_model_id: store.cli.selected_model_id.clone(),
        selected_provider_name: selected.map(|(group, _)| group.name.clone()),
        selected_model_name: selected.map(|(_, model)| model.name.clone()),
        environment_matches: store.cli.enabled
            && selected.is_some()
            && desired_matches_last_written
            && environment_conflicts.is_empty(),
        environment_conflicts,
    })
}

#[cfg(not(windows))]
fn unsupported_state(store: &CopilotByokStore, groups: &[CopilotByokGroup]) -> CopilotCliState {
    let selected = match (
        store.cli.selected_group_id.as_deref(),
        store.cli.selected_model_id.as_deref(),
    ) {
        (Some(group_id), Some(model_id)) => selected(groups, group_id, model_id).ok(),
        _ => None,
    };
    CopilotCliState {
        supported: false,
        enabled: false,
        selected_group_id: store.cli.selected_group_id.clone(),
        selected_model_id: store.cli.selected_model_id.clone(),
        selected_provider_name: selected.map(|(group, _)| group.name.clone()),
        selected_model_name: selected.map(|(_, model)| model.name.clone()),
        environment_matches: false,
        environment_conflicts: Vec::new(),
    }
}

#[cfg(any(windows, test))]
fn ensure_no_external_edits(
    store: &CopilotByokStore,
    current: &BTreeMap<String, Option<String>>,
) -> Result<(), AppError> {
    if !store.cli.enabled {
        return Ok(());
    }
    let changed = conflicts(current, &store.cli.managed_environment.last_written);
    if changed.is_empty() {
        return Ok(());
    }
    Err(AppError::Conflict(format!(
        "Copilot CLI environment was changed outside CC Switch: {}",
        changed.join(", ")
    )))
}

#[cfg(any(windows, test))]
fn apply_with_backend<F>(
    store: &mut CopilotByokStore,
    groups: &[CopilotByokGroup],
    group_id: &str,
    model_id: &str,
    environment: &dyn UserEnvironment,
    persist: F,
) -> Result<CopilotCliState, AppError>
where
    F: Fn(&CopilotByokStore) -> Result<(), AppError>,
{
    let (group, model) = selected(groups, group_id, model_id)?;
    let desired = desired_environment(group, model)?;
    let current = snapshot(environment)?;
    ensure_no_external_edits(store, &current)?;
    let original = if store.cli.enabled {
        store.cli.managed_environment.original.clone()
    } else {
        current.clone()
    };

    let before = write_atomic(environment, &desired)?;
    let previous = store.cli.clone();
    store.cli = CopilotCliConfig {
        enabled: true,
        selected_group_id: Some(group_id.to_string()),
        selected_model_id: Some(model_id.to_string()),
        managed_environment: CopilotCliManagedEnvironment {
            original,
            last_written: desired,
        },
    };
    if let Err(error) = persist(store) {
        store.cli = previous;
        restore_snapshot(environment, &before).map_err(|rollback_error| {
            AppError::Config(format!(
                "{error}; failed to roll back Copilot CLI environment: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    if let Err(error) = environment.broadcast_change() {
        log::warn!("Copilot CLI environment updated, but change broadcast failed: {error}");
    }
    state_with_backend(store, groups, environment)
}

#[cfg(any(windows, test))]
fn disable_with_backend<F>(
    store: &mut CopilotByokStore,
    groups: &[CopilotByokGroup],
    environment: &dyn UserEnvironment,
    persist: F,
) -> Result<CopilotCliState, AppError>
where
    F: Fn(&CopilotByokStore) -> Result<(), AppError>,
{
    if !store.cli.enabled {
        return state_with_backend(store, groups, environment);
    }
    let current = snapshot(environment)?;
    ensure_no_external_edits(store, &current)?;
    let original = store.cli.managed_environment.original.clone();
    let before = write_atomic(environment, &original)?;
    let previous = store.cli.clone();
    store.cli = CopilotCliConfig::default();
    if let Err(error) = persist(store) {
        store.cli = previous;
        restore_snapshot(environment, &before).map_err(|rollback_error| {
            AppError::Config(format!(
                "{error}; failed to reapply Copilot CLI environment: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    if let Err(error) = environment.broadcast_change() {
        log::warn!("Copilot CLI environment restored, but change broadcast failed: {error}");
    }
    state_with_backend(store, groups, environment)
}

pub(super) fn get_state(
    store: &CopilotByokStore,
    groups: &[CopilotByokGroup],
) -> Result<CopilotCliState, AppError> {
    #[cfg(windows)]
    {
        state_with_backend(store, groups, &WindowsUserEnvironment)
    }
    #[cfg(not(windows))]
    {
        Ok(unsupported_state(store, groups))
    }
}

pub(super) fn apply(
    store: &mut CopilotByokStore,
    groups: &[CopilotByokGroup],
    group_id: &str,
    model_id: &str,
) -> Result<CopilotCliState, AppError> {
    #[cfg(windows)]
    {
        apply_with_backend(
            store,
            groups,
            group_id,
            model_id,
            &WindowsUserEnvironment,
            store::save_device_store,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (store, groups, group_id, model_id);
        Err(AppError::InvalidInput(
            "Copilot CLI user-environment switching is currently supported on Windows only"
                .to_string(),
        ))
    }
}

pub(super) fn disable(
    store: &mut CopilotByokStore,
    groups: &[CopilotByokGroup],
) -> Result<CopilotCliState, AppError> {
    #[cfg(windows)]
    {
        disable_with_backend(
            store,
            groups,
            &WindowsUserEnvironment,
            store::save_device_store,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (store, groups);
        Err(AppError::InvalidInput(
            "Copilot CLI user-environment switching is currently supported on Windows only"
                .to_string(),
        ))
    }
}

pub(super) fn validate_selection(store: &CopilotByokStore) -> Result<(), AppError> {
    if !store.cli.enabled {
        return Ok(());
    }
    let group_id = store.cli.selected_group_id.as_deref().ok_or_else(|| {
        AppError::Config("Copilot CLI is enabled without a selected provider".to_string())
    })?;
    let model_id = store.cli.selected_model_id.as_deref().ok_or_else(|| {
        AppError::Config("Copilot CLI is enabled without a selected model".to_string())
    })?;
    selected(&store.groups, group_id, model_id).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct MemoryEnvironment {
        values: RefCell<BTreeMap<String, String>>,
        broadcasts: Cell<usize>,
    }

    impl UserEnvironment for MemoryEnvironment {
        fn read(&self, name: &str) -> Result<Option<String>, AppError> {
            Ok(self.values.borrow().get(name).cloned())
        }

        fn write(&self, name: &str, value: Option<&str>) -> Result<(), AppError> {
            match value {
                Some(value) => {
                    self.values
                        .borrow_mut()
                        .insert(name.to_string(), value.to_string());
                }
                None => {
                    self.values.borrow_mut().remove(name);
                }
            }
            Ok(())
        }

        fn broadcast_change(&self) -> Result<(), AppError> {
            self.broadcasts.set(self.broadcasts.get() + 1);
            Ok(())
        }
    }

    fn group() -> CopilotByokGroup {
        CopilotByokGroup {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            url: "https://api.example.com/v1/responses".to_string(),
            api_key: "secret".to_string(),
            api_type: "responses".to_string(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            enabled: true,
            request_headers: BTreeMap::from([(
                "X-Token".to_string(),
                "Token ${apiKey}".to_string(),
            )]),
            models: vec![CopilotByokModel {
                id: "model-record".to_string(),
                model_id: "wire-model".to_string(),
                name: "Model".to_string(),
                enabled: true,
                tool_calling: Some(true),
                vision: None,
                thinking: None,
                streaming: Some(true),
                context_window: Some(128_000),
                max_input_tokens: Some(120_000),
                max_output_tokens: Some(8_000),
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
    fn maps_responses_provider_to_cli_environment() {
        let desired = desired_environment(&group(), &group().models[0]).expect("environment");
        assert_eq!(
            desired["COPILOT_PROVIDER_BASE_URL"].as_deref(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            desired["COPILOT_PROVIDER_WIRE_API"].as_deref(),
            Some("responses")
        );
        assert_eq!(desired["COPILOT_MODEL"].as_deref(), Some("wire-model"));
        assert_eq!(
            desired["COPILOT_PROVIDER_HEADERS"].as_deref(),
            Some("X-Token: Token secret")
        );
    }

    #[test]
    fn apply_then_disable_restores_original_values() {
        let environment = MemoryEnvironment::default();
        environment
            .values
            .borrow_mut()
            .insert("COPILOT_MODEL".to_string(), "original-model".to_string());
        let groups = vec![group()];
        let mut store = CopilotByokStore {
            groups: groups.clone(),
            ..CopilotByokStore::default()
        };

        let applied = apply_with_backend(
            &mut store,
            &groups,
            "provider",
            "model-record",
            &environment,
            |_| Ok(()),
        )
        .expect("apply CLI environment");
        assert!(applied.enabled);
        assert!(applied.environment_matches);
        assert_eq!(
            environment.values.borrow().get("COPILOT_MODEL"),
            Some(&"wire-model".to_string())
        );

        let disabled = disable_with_backend(&mut store, &groups, &environment, |_| Ok(()))
            .expect("restore CLI environment");
        assert!(!disabled.enabled);
        assert_eq!(
            environment.values.borrow().get("COPILOT_MODEL"),
            Some(&"original-model".to_string())
        );
        assert!(!environment
            .values
            .borrow()
            .contains_key("COPILOT_PROVIDER_BASE_URL"));
        assert_eq!(environment.broadcasts.get(), 2);
    }

    #[test]
    fn external_edit_blocks_restore_without_overwriting_it() {
        let environment = MemoryEnvironment::default();
        let groups = vec![group()];
        let mut store = CopilotByokStore {
            groups: groups.clone(),
            ..CopilotByokStore::default()
        };
        apply_with_backend(
            &mut store,
            &groups,
            "provider",
            "model-record",
            &environment,
            |_| Ok(()),
        )
        .expect("apply CLI environment");
        environment.values.borrow_mut().insert(
            "COPILOT_PROVIDER_API_KEY".to_string(),
            "external-secret".to_string(),
        );

        let error = disable_with_backend(&mut store, &groups, &environment, |_| Ok(()))
            .expect_err("external edit must block restore");
        assert!(matches!(error, AppError::Conflict(_)));
        assert_eq!(
            environment
                .values
                .borrow()
                .get("COPILOT_PROVIDER_API_KEY")
                .map(String::as_str),
            Some("external-secret")
        );
        assert!(store.cli.enabled);
    }

    #[test]
    fn provider_edit_requires_reapply_even_when_environment_is_untouched() {
        let environment = MemoryEnvironment::default();
        let groups = vec![group()];
        let mut store = CopilotByokStore {
            groups: groups.clone(),
            ..CopilotByokStore::default()
        };
        apply_with_backend(
            &mut store,
            &groups,
            "provider",
            "model-record",
            &environment,
            |_| Ok(()),
        )
        .expect("apply CLI environment");

        let mut edited_groups = groups;
        edited_groups[0].url = "https://new.example.com/v1/responses".to_string();
        let state = state_with_backend(&store, &edited_groups, &environment)
            .expect("read CLI environment state");

        assert!(!state.environment_matches);
        assert!(state.environment_conflicts.is_empty());
    }

    #[test]
    fn persistence_failure_rolls_back_environment_and_selection() {
        let environment = MemoryEnvironment::default();
        environment
            .values
            .borrow_mut()
            .insert("COPILOT_MODEL".to_string(), "original-model".to_string());
        let groups = vec![group()];
        let mut store = CopilotByokStore {
            groups: groups.clone(),
            ..CopilotByokStore::default()
        };

        let error = apply_with_backend(
            &mut store,
            &groups,
            "provider",
            "model-record",
            &environment,
            |_| {
                Err(AppError::Config(
                    "simulated persistence failure".to_string(),
                ))
            },
        )
        .expect_err("persistence failure must abort the switch");

        assert!(matches!(error, AppError::Config(_)));
        assert!(!store.cli.enabled);
        assert_eq!(
            environment.values.borrow().get("COPILOT_MODEL"),
            Some(&"original-model".to_string())
        );
        assert!(!environment
            .values
            .borrow()
            .contains_key("COPILOT_PROVIDER_BASE_URL"));
        assert_eq!(environment.broadcasts.get(), 0);
    }

    #[test]
    fn vscode_secret_reference_is_rejected_for_cli() {
        let mut provider = group();
        provider.api_key = "${input:provider-key}".to_string();

        let error = desired_environment(&provider, &provider.models[0])
            .expect_err("VS Code SecretStorage references are not available to the CLI");

        assert!(matches!(error, AppError::InvalidInput(_)));
    }
}
