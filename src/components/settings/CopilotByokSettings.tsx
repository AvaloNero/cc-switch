import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  Download,
  KeyRound,
  Loader2,
  Pencil,
  Plus,
  RefreshCw,
  RotateCcw,
  Server,
  ShieldAlert,
  Trash2,
  Unplug,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { copilotByokApi } from "@/lib/api";
import type {
  CopilotByokModel,
  CopilotByokState,
  CopilotByokTargetState,
} from "@/lib/api";
import copilotByokIcon from "@/assets/icons/vscode-copilot-byok.png";
import { CopilotByokModelDialog } from "./CopilotByokModelDialog";

const COPY = {
  zh: {
    title: "VS Code Copilot BYOK",
    description:
      "像 OpenCode 一样维护可选模型目录；最终使用哪个模型仍由 VS Code 的模型选择器决定。",
    refresh: "刷新",
    loading: "正在读取 VS Code 配置…",
    targets: "同步目标",
    targetsDescription:
      "选择要由 CC Switch 管理的 VS Code 版本和 Profile。模型增删、启停和目标变化会自动增量同步。",
    noTargets:
      "没有检测到 VS Code Stable 或 Insiders，可在下方添加 chatLanguageModels.json 的绝对路径。",
    defaultProfile: "默认 Profile",
    configExists: "已有配置",
    backupExists: "已有备份",
    managedGroups: "CC Switch 模型组",
    importExisting: "导入并接管",
    importSuccess: "已导入模型",
    importedGroups: "接管组",
    reusedModels: "复用模型",
    skippedGroups: "跳过组",
    importWarnings: "部分配置未导入",
    restore: "恢复备份",
    removeCustom: "移除目标",
    customTarget: "自定义目标",
    customName: "目标名称（可选）",
    customPath: "chatLanguageModels.json 的绝对路径",
    addTarget: "添加并选中",
    models: "模型目录",
    modelsDescription:
      "启用的模型会同时出现在所有已选 Profile 的 Copilot 模型选择器中。",
    addModel: "添加模型",
    noModels: "尚未配置 BYOK 模型。",
    enabled: "启用",
    disabled: "停用",
    edit: "编辑",
    delete: "删除",
    repairSync: "重新同步",
    stopManaging: "停止管理所选 Profile",
    selectedProfiles: "已选 Profile",
    enabledModels: "启用模型",
    changedFiles: "已更新文件",
    securityTitle: "API Key 存储提示",
    security:
      "VS Code Custom Endpoint 会把 API Key 保存在 chatLanguageModels.json。外部应用无法写入 VS Code SecretStorage，因此同步后的密钥会出现在对应 Profile 配置文件中。",
    readError: "配置读取失败",
    saveModelSuccess: "模型已保存并自动同步",
    deleteModelSuccess: "模型已删除并自动同步",
    targetUpdateSuccess: "同步目标已更新并自动同步",
    syncSuccess: "Copilot BYOK 已重新同步",
    removeSuccess: "已从所选 Profile 移除 CC Switch 模型组",
    restoreSuccess: "配置备份已恢复",
    restoreNoop: "没有可恢复的备份或受管模型",
    customAdded: "自定义目标已添加",
    customRemoved: "自定义目标已移除",
    confirmDelete:
      "确定删除这个 BYOK 模型吗？已同步到 VS Code 的副本会立即自动移除。",
    confirmStop:
      "确定从所选 Profile 中移除所有 CC Switch 模型组吗？其他 BYOK 配置不会受影响。",
  },
  en: {
    title: "VS Code Copilot BYOK",
    description:
      "Maintain an additive model catalog like OpenCode. VS Code still controls the selected model.",
    refresh: "Refresh",
    loading: "Reading VS Code configuration…",
    targets: "Sync targets",
    targetsDescription:
      "Choose the VS Code editions and profiles managed by CC Switch. Model edits, toggles, and target changes synchronize automatically.",
    noTargets:
      "No VS Code Stable or Insiders installation was detected. Add an absolute chatLanguageModels.json path below.",
    defaultProfile: "Default profile",
    configExists: "Config exists",
    backupExists: "Backup exists",
    managedGroups: "CC Switch groups",
    importExisting: "Import and manage",
    importSuccess: "Imported models",
    importedGroups: "Managed groups",
    reusedModels: "Reused models",
    skippedGroups: "Skipped groups",
    importWarnings: "Some configuration was not imported",
    restore: "Restore backup",
    removeCustom: "Remove target",
    customTarget: "Custom target",
    customName: "Target name (optional)",
    customPath: "Absolute path to chatLanguageModels.json",
    addTarget: "Add and select",
    models: "Model catalog",
    modelsDescription:
      "Enabled models appear in the Copilot model picker of every selected profile.",
    addModel: "Add model",
    noModels: "No BYOK models configured yet.",
    enabled: "Enabled",
    disabled: "Disabled",
    edit: "Edit",
    delete: "Delete",
    repairSync: "Re-sync",
    stopManaging: "Stop managing selected profiles",
    selectedProfiles: "Selected profiles",
    enabledModels: "Enabled models",
    changedFiles: "Changed files",
    securityTitle: "API key storage notice",
    security:
      "VS Code Custom Endpoint stores the API key in chatLanguageModels.json. External applications cannot populate VS Code SecretStorage, so synchronized credentials remain visible in the profile configuration file.",
    readError: "Configuration read failed",
    saveModelSuccess: "Model saved and synchronized",
    deleteModelSuccess: "Model deleted and synchronized",
    targetUpdateSuccess: "Sync targets updated and synchronized",
    syncSuccess: "Copilot BYOK synchronized",
    removeSuccess: "Removed CC Switch model groups from selected profiles",
    restoreSuccess: "Configuration backup restored",
    restoreNoop: "No backup or managed models were available to restore",
    customAdded: "Custom target added",
    customRemoved: "Custom target removed",
    confirmDelete:
      "Delete this BYOK model? Synchronized copies are removed automatically.",
    confirmStop:
      "Remove all CC Switch model groups from the selected profiles? Other BYOK configuration is preserved.",
  },
} as const;

type BusyAction =
  | "load"
  | "targets"
  | "custom"
  | "model"
  | "sync"
  | "remove"
  | `import:${string}`
  | `restore:${string}`
  | `custom-remove:${string}`
  | null;

function targetTitle(target: CopilotByokTargetState, defaultLabel: string) {
  if (target.source === "custom") return target.profileName;
  const edition = target.editionName ?? "VS Code";
  const profile = target.isDefault ? defaultLabel : target.profileName;
  return `${edition} · ${profile}`;
}

function capabilityLabels(model: CopilotByokModel) {
  const labels: string[] = [];
  if (model.toolCalling) labels.push("Tools");
  if (model.vision) labels.push("Vision");
  if (model.thinking) labels.push("Thinking");
  if (model.streaming) labels.push("Streaming");
  return labels;
}

export function CopilotByokSettings() {
  const { i18n } = useTranslation();
  const copy = i18n.resolvedLanguage?.startsWith("zh") ? COPY.zh : COPY.en;
  const [state, setState] = useState<CopilotByokState | null>(null);
  const [busy, setBusy] = useState<BusyAction>("load");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<CopilotByokModel | null>(
    null,
  );
  const [customName, setCustomName] = useState("");
  const [customPath, setCustomPath] = useState("");

  const load = useCallback(
    async (showToast = false) => {
      setBusy("load");
      try {
        const next = await copilotByokApi.getState();
        setState(next);
        if (showToast) toast.success(copy.refresh);
      } catch (error) {
        console.error("[CopilotByokSettings] Failed to load", error);
        toast.error(String(error));
      } finally {
        setBusy(null);
      }
    },
    [copy.refresh],
  );

  useEffect(() => {
    void load();
  }, [load]);

  const selectedTargets = useMemo(
    () => state?.targets.filter((target) => target.selected) ?? [],
    [state],
  );

  const updateTargets = async (targetId: string, checked: boolean) => {
    if (!state || busy) return;
    setBusy("targets");
    try {
      const nextIds = checked
        ? [...new Set([...state.selectedTargetIds, targetId])]
        : state.selectedTargetIds.filter((id) => id !== targetId);
      const next = await copilotByokApi.setTargets(nextIds);
      setState(next);
      toast.success(copy.targetUpdateSuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const addCustomTarget = async () => {
    if (!customPath.trim() || busy) return;
    setBusy("custom");
    try {
      const next = await copilotByokApi.addCustomTarget(
        customPath.trim(),
        customName.trim() || null,
      );
      setState(next);
      setCustomName("");
      setCustomPath("");
      toast.success(copy.customAdded);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const removeCustomTarget = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`custom-remove:${target.id}`);
    try {
      const next = await copilotByokApi.removeCustomTarget(target.id);
      setState(next);
      toast.success(copy.customRemoved);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const saveModel = async (model: CopilotByokModel) => {
    setBusy("model");
    try {
      const next = await copilotByokApi.upsertModel(model);
      setState(next);
      setEditorOpen(false);
      toast.success(copy.saveModelSuccess);
    } catch (error) {
      toast.error(String(error));
      throw error;
    } finally {
      setBusy(null);
    }
  };

  const toggleModel = async (model: CopilotByokModel, enabled: boolean) => {
    if (busy) return;
    setBusy("model");
    try {
      const next = await copilotByokApi.upsertModel({ ...model, enabled });
      setState(next);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const deleteModel = async (model: CopilotByokModel) => {
    if (!window.confirm(copy.confirmDelete) || busy) return;
    setBusy("model");
    try {
      const next = await copilotByokApi.deleteModel(model.id);
      setState(next);
      toast.success(copy.deleteModelSuccess);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const importExistingModels = async (target: CopilotByokTargetState) => {
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
    if (busy || selectedTargets.length === 0) return;
    setBusy("sync");
    try {
      const result = await copilotByokApi.sync();
      toast.success(
        `${copy.syncSuccess} · ${copy.changedFiles}: ${result.changedTargetCount}`,
      );
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const stopManaging = async () => {
    if (!state || busy || selectedTargets.length === 0) return;
    if (!window.confirm(copy.confirmStop)) return;
    setBusy("remove");
    try {
      const result = await copilotByokApi.removeManagedModels(
        state.selectedTargetIds,
      );
      toast.success(
        `${copy.removeSuccess} · ${copy.changedFiles}: ${result.changedTargetCount}`,
      );
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const restoreTarget = async (target: CopilotByokTargetState) => {
    if (busy) return;
    setBusy(`restore:${target.id}`);
    try {
      const restored = await copilotByokApi.restoreBackup(target.id);
      toast.success(restored ? copy.restoreSuccess : copy.restoreNoop);
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  if (!state && busy === "load") {
    return (
      <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        {copy.loading}
      </div>
    );
  }

  if (!state) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>{copy.readError}</AlertTitle>
        <AlertDescription>
          <Button className="mt-3" variant="outline" onClick={() => load()}>
            {copy.refresh}
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-start gap-4">
          <img
            src={copilotByokIcon}
            alt="VS Code Copilot"
            className="h-14 w-14 rounded-xl border object-cover shadow-sm"
          />
          <div>
            <h3 className="text-lg font-semibold">{copy.title}</h3>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              {copy.description}
            </p>
          </div>
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() => load(true)}
          disabled={Boolean(busy)}
        >
          <RefreshCw
            className={`mr-2 h-4 w-4 ${busy === "load" ? "animate-spin" : ""}`}
          />
          {copy.refresh}
        </Button>
      </div>

      <Alert variant="destructive">
        <ShieldAlert className="h-4 w-4" />
        <AlertTitle>{copy.securityTitle}</AlertTitle>
        <AlertDescription>{copy.security}</AlertDescription>
      </Alert>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Server className="h-4 w-4" />
            {copy.targets}
          </CardTitle>
          <p className="text-sm text-muted-foreground">
            {copy.targetsDescription}
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          {state.targets.length === 0 ? (
            <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              {copy.noTargets}
            </p>
          ) : (
            <div className="space-y-3">
              {state.targets.map((target) => (
                <div
                  key={target.id}
                  className="flex flex-col gap-3 rounded-lg border bg-card/50 p-4 xl:flex-row xl:items-center xl:justify-between"
                >
                  <div className="flex min-w-0 items-start gap-3">
                    <Checkbox
                      checked={target.selected}
                      disabled={Boolean(busy)}
                      onCheckedChange={(checked) =>
                        void updateTargets(target.id, checked === true)
                      }
                      aria-label={targetTitle(target, copy.defaultProfile)}
                    />
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-medium">
                          {targetTitle(target, copy.defaultProfile)}
                        </span>
                        {target.configExists ? (
                          <Badge variant="secondary">{copy.configExists}</Badge>
                        ) : null}
                        {target.backupExists ? (
                          <Badge variant="outline">{copy.backupExists}</Badge>
                        ) : null}
                        {target.managedGroupCount > 0 ? (
                          <Badge variant="outline">
                            {copy.managedGroups}: {target.managedGroupCount}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                        {target.languageModelsPath}
                      </p>
                      {target.readError ? (
                        <p className="mt-2 text-xs text-destructive">
                          {copy.readError}: {target.readError}
                        </p>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-2">
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
                    >
                      {busy === `restore:${target.id}` ? (
                        <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <RotateCcw className="mr-2 h-3.5 w-3.5" />
                      )}
                      {copy.restore}
                    </Button>
                    {target.source === "custom" ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={Boolean(busy)}
                        onClick={() => void removeCustomTarget(target)}
                      >
                        {busy === `custom-remove:${target.id}` ? (
                          <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Trash2 className="mr-2 h-3.5 w-3.5" />
                        )}
                        {copy.removeCustom}
                      </Button>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          )}

          <div className="grid gap-3 rounded-lg border border-dashed p-4 md:grid-cols-[minmax(0,0.8fr)_minmax(0,1.5fr)_auto]">
            <div className="space-y-2">
              <Label htmlFor="copilot-custom-name">{copy.customName}</Label>
              <Input
                id="copilot-custom-name"
                value={customName}
                onChange={(event) => setCustomName(event.target.value)}
                disabled={Boolean(busy)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-custom-path">{copy.customPath}</Label>
              <Input
                id="copilot-custom-path"
                value={customPath}
                onChange={(event) => setCustomPath(event.target.value)}
                disabled={Boolean(busy)}
                className="font-mono text-xs"
              />
            </div>
            <div className="flex items-end">
              <Button
                onClick={() => void addCustomTarget()}
                disabled={Boolean(busy) || customPath.trim().length === 0}
              >
                {busy === "custom" ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Plus className="mr-2 h-4 w-4" />
                )}
                {copy.addTarget}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between gap-4">
            <div>
              <CardTitle className="flex items-center gap-2 text-base">
                <KeyRound className="h-4 w-4" />
                {copy.models}
              </CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">
                {copy.modelsDescription}
              </p>
            </div>
            <Button
              size="sm"
              onClick={() => {
                setEditingModel(null);
                setEditorOpen(true);
              }}
              disabled={Boolean(busy)}
            >
              <Plus className="mr-2 h-4 w-4" />
              {copy.addModel}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {state.models.length === 0 ? (
            <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              {copy.noModels}
            </p>
          ) : (
            state.models.map((model) => {
              const capabilities = capabilityLabels(model);
              return (
                <div
                  key={model.id}
                  className="flex flex-col gap-3 rounded-lg border p-4 lg:flex-row lg:items-center lg:justify-between"
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{model.name}</span>
                      <Badge variant={model.enabled ? "secondary" : "outline"}>
                        {model.enabled ? copy.enabled : copy.disabled}
                      </Badge>
                      <Badge variant="outline">{model.apiType}</Badge>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {model.modelId}
                    </p>
                    <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                      {model.url}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                      <span>Context: {model.contextWindow.toLocaleString()}</span>
                      <span>Output: {model.maxOutputTokens.toLocaleString()}</span>
                      {capabilities.map((label) => (
                        <span key={label}>{label}</span>
                      ))}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Switch
                      checked={model.enabled}
                      disabled={Boolean(busy)}
                      onCheckedChange={(checked) =>
                        void toggleModel(model, checked)
                      }
                      aria-label={`${model.name} ${copy.enabled}`}
                    />
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={Boolean(busy)}
                      onClick={() => {
                        setEditingModel(model);
                        setEditorOpen(true);
                      }}
                    >
                      <Pencil className="mr-2 h-3.5 w-3.5" />
                      {copy.edit}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={Boolean(busy)}
                      onClick={() => void deleteModel(model)}
                    >
                      <Trash2 className="mr-2 h-3.5 w-3.5" />
                      {copy.delete}
                    </Button>
                  </div>
                </div>
              );
            })
          )}
        </CardContent>
      </Card>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-muted/20 p-4">
        <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
          <span>
            {copy.selectedProfiles}: {selectedTargets.length}
          </span>
          <span>
            {copy.enabledModels}: {state.managedModelCount}
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            onClick={() => void sync()}
            disabled={Boolean(busy) || selectedTargets.length === 0}
          >
            {busy === "sync" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {copy.repairSync}
          </Button>
          <Button
            variant="destructive"
            onClick={() => void stopManaging()}
            disabled={Boolean(busy) || selectedTargets.length === 0}
          >
            {busy === "remove" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Unplug className="mr-2 h-4 w-4" />
            )}
            {copy.stopManaging}
          </Button>
        </div>
      </div>

      <CopilotByokModelDialog
        open={editorOpen}
        model={editingModel}
        saving={busy === "model"}
        onOpenChange={(open) => {
          setEditorOpen(open);
          if (!open) setEditingModel(null);
        }}
        onSave={saveModel}
      />
    </div>
  );
}
