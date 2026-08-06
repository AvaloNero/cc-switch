import { invoke } from "@tauri-apps/api/core";

export interface CopilotByokModel {
  id: string;
  name: string;
  endpoint: string;
  apiKey: string;
  modelId: string;
  enabled: boolean;
  requestHeaders: Record<string, string>;
  contextWindow: number;
  maxOutputTokens: number;
  toolCalling: boolean;
  vision: boolean;
  thinking: boolean;
  streaming: boolean;
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

export interface CopilotByokState {
  models: CopilotByokModel[];
  targets: CopilotByokTargetState[];
  selectedTargetIds: string[];
  currentProviderId: string | null;
  integrationEnabled: boolean;
  serverRunning: boolean;
  listenPort: number;
  proxyUrl: string;
  fixedModelId: string;
}

export interface CopilotByokSyncResult {
  targetIds: string[];
  managedModelCount: number;
  changedTargetCount: number;
  integrationEnabled: boolean;
  serverRunning: boolean;
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

  start(): Promise<CopilotByokSyncResult> {
    return invoke<CopilotByokSyncResult>("copilot_byok_sync", {
      targetId: null,
    });
  },

  selectProvider(providerId: string): Promise<CopilotByokSyncResult> {
    return invoke<CopilotByokSyncResult>("copilot_byok_sync", {
      targetId: providerId,
    });
  },

  stop(targetIds?: string[] | null): Promise<CopilotByokSyncResult> {
    return invoke<CopilotByokSyncResult>("copilot_byok_remove_managed_models", {
      targetIds: targetIds?.length ? targetIds : null,
    });
  },

  restoreBackup(targetId: string): Promise<boolean> {
    return invoke<boolean>("copilot_byok_restore_backup", { targetId });
  },
};
