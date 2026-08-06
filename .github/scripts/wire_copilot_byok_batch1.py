from pathlib import Path


def replace_once(text: str, anchor: str, replacement: str, label: str) -> str:
    count = text.count(anchor)
    if count != 1:
        raise SystemExit(f"{label} anchor count is {count}, expected 1")
    return text.replace(anchor, replacement, 1)


path = Path("src-tauri/src/lib.rs")
text = path.read_text(encoding="utf-8")

if "mod copilot_byok;" not in text:
    text = replace_once(
        text,
        "mod commands;\nmod config;\n",
        "mod commands;\nmod config;\nmod copilot_byok;\n",
        "lib.rs module",
    )

if "commands::copilot_byok_get_state," not in text:
    command_anchor = (
        "            commands::copilot_get_usage_for_account,\n"
        "            // OMO commands\n"
    )
    command_replacement = "\n".join(
        [
            "            commands::copilot_get_usage_for_account,",
            "            // VS Code Copilot BYOK model catalog",
            "            commands::copilot_byok_get_state,",
            "            commands::copilot_byok_set_targets,",
            "            commands::copilot_byok_add_custom_target,",
            "            commands::copilot_byok_remove_custom_target,",
            "            commands::copilot_byok_upsert_model,",
            "            commands::copilot_byok_delete_model,",
            "            commands::copilot_byok_sync,",
            "            commands::copilot_byok_remove_managed_models,",
            "            commands::copilot_byok_restore_backup,",
            "            // OMO commands",
            "",
        ]
    )
    text = replace_once(
        text,
        command_anchor,
        command_replacement,
        "lib.rs command",
    )

path.write_text(text, encoding="utf-8")
