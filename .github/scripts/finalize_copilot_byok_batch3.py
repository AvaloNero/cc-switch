from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if new in text:
        return
    if text.count(old) != 1:
        raise SystemExit(f"anchor changed in {path}: {old[:80]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


# Register the import command with Tauri.
lib = Path("src-tauri/src/lib.rs")
replace_once(
    lib,
    "            commands::copilot_byok_delete_model,\n            commands::copilot_byok_sync,\n",
    "            commands::copilot_byok_delete_model,\n"
    "            commands::copilot_byok_import_models,\n"
    "            commands::copilot_byok_sync,\n",
)

# Ignore stale profile IDs when VS Code profiles are removed outside CC Switch.
root = Path("src-tauri/src/copilot_byok.rs")
replace_once(
    root,
    """    let detected = vscode::discover_vscode_targets()?;
    let selected_target_ids = sync::effective_selected_target_ids(&store, &detected);
    let selected_ids: HashSet<String> = selected_target_ids.iter().cloned().collect();
""",
    """    let detected = vscode::discover_vscode_targets()?;
    let available_ids: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(
            store
                .custom_targets
                .iter()
                .map(|target| target.id.clone()),
        )
        .collect();
    let mut selected_target_ids = sync::effective_selected_target_ids(&store, &detected);
    selected_target_ids.retain(|id| available_ids.contains(id));
    let selected_ids: HashSet<String> = selected_target_ids.iter().cloned().collect();
""",
)
replace_once(
    root,
    """fn effective_target_ids(store: &CopilotByokStore) -> Result<Vec<String>, AppError> {
    let detected = vscode::discover_vscode_targets()?;
    Ok(sync::effective_selected_target_ids(store, &detected))
}

fn sync_if_selected(store: &CopilotByokStore) -> Result<(), AppError> {
    if effective_target_ids(store)?.is_empty() {
        return Ok(());
    }
    sync::sync_store(store)?;
    Ok(())
}
""",
    """fn sync_if_selected(store: &CopilotByokStore) -> Result<(), AppError> {
    let detected = vscode::discover_vscode_targets()?;
    let available_ids: HashSet<String> = detected
        .iter()
        .map(|target| target.id.clone())
        .chain(
            store
                .custom_targets
                .iter()
                .map(|target| target.id.clone()),
        )
        .collect();
    let selected_target_ids: Vec<String> =
        sync::effective_selected_target_ids(store, &detected)
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
""",
)
replace_once(
    root,
    """    let previous_ids: HashSet<String> = sync::effective_selected_target_ids(&current, &detected)
        .into_iter()
        .collect();
""",
    """    let previous_ids: HashSet<String> = sync::effective_selected_target_ids(&current, &detected)
        .into_iter()
        .filter(|id| valid_ids.contains(id))
        .collect();
""",
)

# Improve import reporting and warn when a VS Code SecretStorage reference is reused.
import_rs = Path("src-tauri/src/copilot_byok/import.rs")
replace_once(
    import_rs,
    """        match parse_group(target_id, group)
            .and_then(|parsed| add_group_models(&mut store, parsed))
        {
            Ok((imported, reused)) => {
                accepted_indexes.insert(index);
                imported_group_count += 1;
                imported_model_count += imported;
                reused_model_count += reused;
            }
""",
    """        let parsed = parse_group(target_id, group);
        let secret_reference_group = parsed.as_ref().ok().and_then(|parsed| {
            parsed
                .models
                .iter()
                .any(|model| model.api_key.starts_with("${input:"))
                .then(|| parsed.name.clone())
        });
        match parsed.and_then(|parsed| add_group_models(&mut store, parsed)) {
            Ok((imported, reused)) => {
                if let Some(group_name) = secret_reference_group {
                    warnings.push(format!(
                        "{group_name} keeps a VS Code SecretStorage reference; other profiles may need the secret to be entered again"
                    ));
                }
                accepted_indexes.insert(index);
                imported_group_count += 1;
                imported_model_count += imported;
                reused_model_count += reused;
            }
""",
)
replace_once(
    import_rs,
    "        changed_target_count: sync_result.changed_target_count,\n",
    "        changed_target_count: sync_result.changed_target_count + 1,\n",
)

# Correct the literal placeholder shown to users.
dialog = Path("src/components/settings/CopilotByokModelDialog.tsx")
replace_once(
    dialog,
    '<code>${"${apiKey}"}</code>{" "}',
    '<code>{"${apiKey}"}</code>{" "}',
)

# Add import/takeover controls and describe automatic synchronization.
panel = Path("src/components/settings/CopilotByokSettings.tsx")
replace_once(panel, "  AlertCircle,\n  KeyRound,\n", "  AlertCircle,\n  Download,\n  KeyRound,\n")
replace_once(
    panel,
    '    managedGroups: "CC Switch 模型组",\n',
    '    managedGroups: "CC Switch 模型组",\n'
    '    importExisting: "导入并接管",\n'
    '    importSuccess: "已导入模型",\n'
    '    importedGroups: "接管组",\n'
    '    reusedModels: "复用模型",\n'
    '    skippedGroups: "跳过组",\n'
    '    importWarnings: "部分配置未导入",\n',
)
replace_once(
    panel,
    '    managedGroups: "CC Switch groups",\n',
    '    managedGroups: "CC Switch groups",\n'
    '    importExisting: "Import and manage",\n'
    '    importSuccess: "Imported models",\n'
    '    importedGroups: "Managed groups",\n'
    '    reusedModels: "Reused models",\n'
    '    skippedGroups: "Skipped groups",\n'
    '    importWarnings: "Some configuration was not imported",\n',
)
replace_once(
    panel,
    '      "这些模型会同时出现在所选 VS Code Profile 的 Copilot 模型选择器中。",\n',
    '      "这些模型会同时出现在所选 VS Code Profile 的 Copilot 模型选择器中；模型增删、启停和 Profile 选择会自动增量同步。",\n',
)
replace_once(
    panel,
    '      "Enabled models are added to the Copilot model picker of every selected VS Code profile.",\n',
    '      "Enabled models are added to every selected VS Code profile. Model edits, toggles, and profile selection are synchronized automatically.",\n',
)
replace_once(
    panel,
    '    saveModelSuccess: "模型已保存",\n'
    '    deleteModelSuccess: "模型已删除",\n'
    '    targetUpdateSuccess: "同步目标已更新",\n',
    '    saveModelSuccess: "模型已保存并自动同步",\n'
    '    deleteModelSuccess: "模型已删除并自动同步",\n'
    '    targetUpdateSuccess: "同步目标已更新并自动同步",\n',
)
replace_once(
    panel,
    '    saveModelSuccess: "Model saved",\n'
    '    deleteModelSuccess: "Model deleted",\n'
    '    targetUpdateSuccess: "Sync targets updated",\n',
    '    saveModelSuccess: "Model saved and synchronized",\n'
    '    deleteModelSuccess: "Model deleted and synchronized",\n'
    '    targetUpdateSuccess: "Sync targets updated and synchronized",\n',
)
replace_once(
    panel,
    '      "确定删除这个 BYOK 模型吗？已同步到 VS Code 的旧模型会在下次同步时移除。",\n',
    '      "确定删除这个 BYOK 模型吗？已同步到 VS Code 的旧模型会立即自动移除。",\n',
)
replace_once(
    panel,
    '      "Delete this BYOK model? A previously synchronized copy will be removed on the next sync.",\n',
    '      "Delete this BYOK model? Previously synchronized copies are removed automatically.",\n',
)
replace_once(
    panel,
    '  | "remove"\n  | `restore:${string}`\n',
    '  | "remove"\n  | `import:${string}`\n  | `restore:${string}`\n',
)
replace_once(panel, "if (showToast) toast.success(COPY.zh.refresh);", "if (showToast) toast.success(copy.refresh);")
replace_once(panel, "  }, []);\n\n  useEffect", "  }, [copy.refresh]);\n\n  useEffect")
replace_once(
    panel,
    """  const sync = async () => {
""",
    """  const importExistingModels = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`import:${target.id}`);
    try {
      const result = await copilotByokApi.importModels(target.id);
      toast.success(
        `${copy.importSuccess}: ${result.importedModelCount} · ${copy.importedGroups}: ${result.importedGroupCount} · ${copy.reusedModels}: ${result.reusedModelCount} · ${copy.skippedGroups}: ${result.skippedGroupCount}`,
      );
      if (result.warnings.length > 0) {
        toast.warning(copy.importWarnings, {
          description: result.warnings.join("\n"),
        });
      }
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const sync = async () => {
""",
)
replace_once(
    panel,
    """                    <div className="flex shrink-0 items-center gap-2">
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={Boolean(busy)}
                        onClick={() => void restoreTarget(target)}
""",
    """                    <div className="flex shrink-0 items-center gap-2">
                      {target.configExists && !target.readError ? (
                        <Button
                          size="sm"
                          variant="outline"
                          disabled={Boolean(busy)}
                          onClick={() => void importExistingModels(target)}
                        >
                          {busy === `import:${target.id}` ? (
                            <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                          ) : (
                            <Download className="mr-2 h-3.5 w-3.5" />
                          )}
                          {copy.importExisting}
                        </Button>
                      ) : null}
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={Boolean(busy)}
                        onClick={() => void restoreTarget(target)}
""",
)
