import { invoke } from "@tauri-apps/api/core";

export type CopilotByokApiType = "chat-completions" | "responses" | "messages";

export type CopilotByokReasoningEffortFormat = CopilotByokApiType;

export type CopilotByokEditTool =
  | "find-replace"
  | "multi-find-replace"
  | "apply-patch"
  | "code-rewrite";

export interface CopilotByokModel {
  id: string;
  modelId: string;
  name: string;
  url: string;
  apiKey: string;
  apiType: CopilotByokApiType;
  enabled: boolean;
  toolCalling: boolean;
  vision: boolean;
  thinking: boolean;
  streaming: boolean;
  contextWindow: number;
  maxInputTokens: number | null;
  maxOutputTokens: number;
  editTools: CopilotByokEditTool[];
  zeroDataRetentionEnabled: boolean;
  supportsReasoningEffort: string[];
  reasoningEffortFormat: CopilotByokReasoningEffortFormat | null;
  requestHeaders: Record<string, string>;
  modelOptions: unknown;
}

export type VsCodeEdition = "stable" | "insiders";

export interface CopilotByokTargetState {
  id: string;
  source: "detected" | "custom";
  edition: VsCodeEdition | null;
  editionName: string | null;
  profileId: string | null;
  profileName: string;
  isDefault: boolean;
  languageModelsPath: string;
  configExists: boolean;
  backupExists: boolean;
  selected: boolean;
  managedGroupCount: number;
  readError: string | null;
}

export interface CopilotByokSecurityNotice {
  apiKeysAreWrittenToVscodeConfig: boolean;
  message: string;
}

export interface CopilotByokState {
  models: CopilotByokModel[];
  targets: CopilotByokTargetState[];
  selectedTargetIds: string[];
  managedModelCount: number;
  securityNotice: CopilotByokSecurityNotice;
}

export interface CopilotByokSyncResult {
  targetIds: string[];
  managedModelCount: number;
  changedTargetCount: number;
}

export interface CopilotByokImportResult {
  targetId: string;
  importedGroupCount: number;
  importedModelCount: number;
  reusedModelCount: number;
  skippedGroupCount: number;
  changedTargetCount: number;
  warnings: string[];
}

export const copilotByokApi = {
  getState(): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_get_state");
  },

  setTargets(targetIds: string[]): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_set_targets", {
      targetIds,
    });
  },

  addCustomTarget(
    path: string,
    name?: string | null,
  ): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_add_custom_target", {
      path,
      name: name || null,
    });
  },

  removeCustomTarget(targetId: string): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_remove_custom_target", {
      targetId,
    });
  },

  upsertModel(model: CopilotByokModel): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_upsert_model", {
      model,
    });
  },

  deleteModel(modelId: string): Promise<CopilotByokState> {
    return invoke<CopilotByokState>("copilot_byok_delete_model", {
      modelId,
    });
  },

  importModels(targetId: string): Promise<CopilotByokImportResult> {
    return invoke<CopilotByokImportResult>("copilot_byok_sync", {
      targetId,
    });
  },

  sync(): Promise<CopilotByokSyncResult> {
    return invoke<CopilotByokSyncResult>("copilot_byok_sync", {
      targetId: null,
    });
  },

  removeManagedModels(
    targetIds?: string[] | null,
  ): Promise<CopilotByokSyncResult> {
    return invoke<CopilotByokSyncResult>("copilot_byok_remove_managed_models", {
      targetIds: targetIds?.length ? targetIds : null,
    });
  },

  restoreBackup(targetId: string): Promise<boolean> {
    return invoke<boolean>("copilot_byok_restore_backup", { targetId });
  },
};
