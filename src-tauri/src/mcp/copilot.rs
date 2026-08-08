use crate::app_config::{McpApps, McpServer};
use crate::config::write_json_file;
use crate::error::AppError;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;

fn target_paths() -> Result<Vec<PathBuf>, AppError> {
    crate::copilot_byok::selected_language_model_paths().map(|paths| {
        paths
            .into_iter()
            .filter_map(|path| path.parent().map(Path::to_path_buf))
            .map(|dir| dir.join("mcp.json"))
            .collect()
    })
}

fn read_root(path: &Path) -> Result<Map<String, Value>, AppError> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::InvalidInput(format!(
            "VS Code MCP target is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(AppError::InvalidInput(format!(
            "VS Code MCP config exceeds {} MiB: {}",
            MAX_CONFIG_BYTES / 1024 / 1024,
            path.display()
        )));
    }

    let text = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    json5::from_str::<Value>(&text)
        .map_err(|error| {
            AppError::Config(format!(
                "Failed to parse VS Code MCP config {}: {error}",
                path.display()
            ))
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            AppError::Config(format!(
                "VS Code MCP config must be a JSON object: {}",
                path.display()
            ))
        })
}

fn update_server(path: &Path, id: &str, server: Option<&Value>) -> Result<(), AppError> {
    let mut root = read_root(path)?;
    let servers = root
        .entry("servers".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            AppError::Config(format!(
                "VS Code MCP 'servers' field must be an object: {}",
                path.display()
            ))
        })?;

    match server {
        Some(server) => {
            if !server.is_object() {
                return Err(AppError::InvalidInput(
                    "VS Code MCP server configuration must be a JSON object".to_string(),
                ));
            }
            servers.insert(id.to_string(), server.clone());
        }
        None => {
            servers.remove(id);
        }
    }
    write_json_file(path, &Value::Object(root))
}

pub fn sync_single_server_to_copilot(id: &str, server: &Value) -> Result<(), AppError> {
    for path in target_paths()? {
        update_server(&path, id, Some(server))?;
    }
    Ok(())
}

pub fn remove_server_from_copilot(id: &str) -> Result<(), AppError> {
    for path in target_paths()? {
        if path.exists() {
            update_server(&path, id, None)?;
        }
    }
    Ok(())
}

pub fn import_from_copilot() -> Result<Vec<McpServer>, AppError> {
    let mut imported = BTreeMap::<String, Value>::new();
    for path in target_paths()? {
        let root = read_root(&path)?;
        if let Some(servers) = root.get("servers").and_then(Value::as_object) {
            for (id, server) in servers {
                imported.entry(id.clone()).or_insert_with(|| server.clone());
            }
        }
    }

    Ok(imported
        .into_iter()
        .map(|(id, server)| McpServer {
            name: id.clone(),
            id,
            server,
            apps: McpApps {
                copilot_byok: true,
                ..McpApps::default()
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        })
        .collect())
}
