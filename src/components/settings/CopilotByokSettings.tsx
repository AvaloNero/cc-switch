import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  KeyRound,
  Loader2,
  Pencil,
  Plus,
  RefreshCw,
  RotateCcw,
  Save,
  Server,
  ShieldAlert,
  Trash2,
  Unplug,
} from "lucide-react";
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
import { useTranslation } from "react-i18next";

const COPY = {
  zh: {
    title: "VS Code Copilot BYOK",
    description:
      "像 OpenCode 一样维护可选模型目录；最终使用哪个模型仍由 VS Code 模型选择器决定。",
    refresh: "刷新",
    loading: "正在读取 VS Code 配置…",
    targets: "同步目标",
    targetsDescription:
      "选择需要写入 chatLanguageModels.json 的 VS Code 版本和 Profile。取消全部选择即可关闭同步。",
    noTargets:
      "没有检测到 VS Code Stable 或 Insiders。可在下方添加自定义 Profile 路径。",
    defaultProfile: "默认 Profile",
    configExists: "已有配置",
    backupExists: "已有备份",
    managedGroups: "CC Switch 模型组",
    restore: "恢复备份",
    removeCustom: "移除目标",
    customTarget: "自定义目标",
    customName: "目标名称（可选）",
    customPath: "chatLanguageModels.json 的绝对路径",
    addTarget: "添加并选中",
    models: "模型目录",
    modelsDescription:
      "这些模型会同时出现在所选 VS Code Profile 的 Copilot 模型选择器中。",
    addModel: "添加模型",
    noModels: "尚未配置 BYOK 模型。",
    enabled: "启用",
    disabled: "停用",
    edit: "编辑",
    delete: "删除",
    sync: "同步到 VS Code",
    stopManaging: "停止管理所选 Profile",
    selectedProfiles: "已选 Profile",
    enabledModels: "启用模型",
    changedFiles: "已更新文件",
    securityTitle: "API Key 存储提示",
    security:
      "VS Code Custom Endpoint 当前把 API Key 保存在 chatLanguageModels.json。外部应用无法写入 VS Code SecretStorage，因此同步后密钥会出现在对应 Profile 配置文件中。",
    readError: "配置读取失败",
    saveModelSuccess: "模型已保存",
    deleteModelSuccess: "模型已删除",
    targetUpdateSuccess: "同步目标已更新",
    syncSuccess: "Copilot BYOK 已同步",
    removeSuccess: "已从所选 Profile 移除 CC Switch 模型组",
    restoreSuccess: "配置备份已恢复",
    restoreNoop: "没有可恢复的备份或受管模型",
    customAdded: "自定义目标已添加",
    customRemoved: "自定义目标已移除",
    confirmDelete:
      "确定删除这个 BYOK 模型吗？已同步到 VS Code 的旧模型会在下次同步时移除。",
    confirmStop:
      "确定从所选 Profile 中移除所有 CC Switch 模型组吗？用户自己配置的其他 BYOK 模型不会受影响。",
  },
  en: {
    title: "VS Code Copilot BYOK",
    description:
      "Maintain an additive model catalog like OpenCode. VS Code still controls which model is selected.",
    refresh: "Refresh",
    loading: "Reading VS Code configuration…",
    targets: "Sync targets",
    targetsDescription:
      "Choose the VS Code editions and profiles whose chatLanguageModels.json files CC Switch should manage. Clear all selections to disable sync.",
    noTargets:
      "No VS Code Stable or Insiders installation was detected. Add a custom profile path below.",
    defaultProfile: "Default profile",
    configExists: "Config exists",
    backupExists: "Backup exists",
    managedGroups: "CC Switch groups",
    restore: "Restore backup",
    removeCustom: "Remove target",
    customTarget: "Custom target",
    customName: "Target name (optional)",
    customPath: "Absolute path to chatLanguageModels.json",
    addTarget: "Add and select",
    models: "Model catalog",
    modelsDescription:
      "Enabled models are added to the Copilot model picker of every selected VS Code profile.",
    addModel: "Add model",
    noModels: "No BYOK models configured yet.",
    enabled: "Enabled",
    disabled: "Disabled",
    edit: "Edit",
    delete: "Delete",
    sync: "Sync to VS Code",
    stopManaging: "Stop managing selected profiles",
    selectedProfiles: "Selected profiles",
    enabledModels: "Enabled models",
    changedFiles: "Changed files",
    securityTitle: "API key storage notice",
    security:
      "VS Code Custom Endpoint currently stores the API key in chatLanguageModels.json. External applications cannot populate VS Code SecretStorage, so synced credentials are visible in the profile configuration file.",
    readError: "Configuration read failed",
    saveModelSuccess: "Model saved",
    deleteModelSuccess: "Model deleted",
    targetUpdateSuccess: "Sync targets updated",
    syncSuccess: "Copilot BYOK synchronized",
    removeSuccess: "Removed CC Switch model groups from selected profiles",
    restoreSuccess: "Configuration backup restored",
    restoreNoop: "No backup or managed models were available to restore",
    customAdded: "Custom target added",
    customRemoved: "Custom target removed",
    confirmDelete:
      "Delete this BYOK model? A previously synchronized copy will be removed on the next sync.",
    confirmStop:
      "Remove every CC Switch model group from the selected profiles? Other user-managed BYOK models are preserved.",
  },
} as const;

type BusyAction =
  | "load"
  | "targets"
  | "custom"
  | "model"
  | "sync"
  | "remove"
  | `restore:${string}`
  | `custom-remove:${string}`
  | null;

function targetTitle(target: CopilotByokTargetState, defaultLabel: string) {
  const edition = target.editionName ?? target.profileName;
  const profile = target.isDefault ? defaultLabel : target.profileName;
  return target.source === "custom"
    ? target.profileName
    : `${edition} · ${profile}`;
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

  const load = useCallback(async (showToast = false) => {
    setBusy("load");
    try {
      const next = await copilotByokApi.getState();
      setState(next);
      if (showToast) toast.success(COPY.zh.refresh);
    } catch (error) {
      console.error("[CopilotByokSettings] Failed to load", error);
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  }, []);

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

  const openNewModel = () => {
    setEditingModel(null);
    setEditorOpen(true);
  };

  const openEditModel = (model: CopilotByokModel) => {
    setEditingModel(model);
    setEditorOpen(true);
  };

  const saveModel = async (model: CopilotByokModel) => {
    setBusy("model");
    try {
      const next = await copilotByokApi.upsertModel(model);
      setState(next);
      setEditorOpen(false);
      toast.success(copy.saveModelSuccess);
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

  const sync = async () => {
    if (busy) return;
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
    if (!window.confirm(copy.confirmStop) || busy || !state) return;
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
                  className="rounded-lg border bg-card/50 p-4"
                >
                  <div className="flex items-start justify-between gap-4">
                    <label className="flex min-w-0 flex-1 items-start gap-3">
                      <Checkbox
                        className="mt-1"
                        checked={target.selected}
                        disabled={busy === "targets"}
                        onCheckedChange={(checked) =>
                          void updateTargets(target.id, checked === true)
                        }
                      />
                      <span className="min-w-0">
                        <span className="block font-medium">
                          {targetTitle(target, copy.defaultProfile)}
                        </span>
                        <span className="mt-1 block break-all font-mono text-xs text-muted-foreground">
                          {target.languageModelsPath}
                        </span>
                      </span>
                    </label>
                    <div className="flex shrink-0 items-center gap-2">
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
                          size="icon"
                          variant="ghost"
                          disabled={Boolean(busy)}
                          onClick={() => void removeCustomTarget(target)}
                          aria-label={copy.removeCustom}
                        >
                          {busy === `custom-remove:${target.id}` ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <Trash2 className="h-4 w-4" />
                          )}
                        </Button>
                      ) : null}
                    </div>
                  </div>
                  <div className="mt-3 flex flex-wrap gap-2">
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
                    {target.readError ? (
                      <Badge variant="destructive">{copy.readError}</Badge>
                    ) : null}
                  </div>
                  {target.readError ? (
                    <p className="mt-2 break-all text-xs text-destructive">
                      {target.readError}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          )}

          <div className="grid gap-3 rounded-lg border border-dashed p-4 md:grid-cols-[minmax(0,0.7fr)_minmax(0,1.3fr)_auto]">
            <div className="space-y-2">
              <Label htmlFor="copilot-custom-name">{copy.customName}</Label>
              <Input
                id="copilot-custom-name"
                value={customName}
                onChange={(event) => setCustomName(event.target.value)}
                placeholder={copy.customTarget}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-custom-path">{copy.customPath}</Label>
              <Input
                id="copilot-custom-path"
                value={customPath}
                onChange={(event) => setCustomPath(event.target.value)}
                placeholder="C:\\Users\\name\\AppData\\Roaming\\Code\\User\\chatLanguageModels.json"
              />
            </div>
            <Button
              className="self-end"
              variant="outline"
              disabled={!customPath.trim() || Boolean(busy)}
              onClick={() => void addCustomTarget()}
            >
              {busy === "custom" ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Plus className="mr-2 h-4 w-4" />
              )}
              {copy.addTarget}
            </Button>
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
            <Button size="sm" onClick={openNewModel} disabled={Boolean(busy)}>
              <Plus className="mr-2 h-4 w-4" />
              {copy.addModel}
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {state.models.length === 0 ? (
            <p className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
              {copy.noModels}
            </p>
          ) : (
            <div className="space-y-3">
              {state.models.map((model) => (
                <div
                  key={model.id}
                  className="flex items-start justify-between gap-4 rounded-lg border p-4"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{model.name}</span>
                      <Badge variant={model.enabled ? "secondary" : "outline"}>
                        {model.enabled ? copy.enabled : copy.disabled}
                      </Badge>
                      <Badge variant="outline">{model.apiType}</Badge>
                    </div>
                    <p className="mt-1 font-mono text-xs text-muted-foreground">
                      {model.modelId}
                    </p>
                    <p className="mt-1 break-all text-xs text-muted-foreground">
                      {model.url}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-2 text-xs text-muted-foreground">
                      <span>
                        Context: {model.contextWindow.toLocaleString()}
                      </span>
                      <span>
                        Output: {model.maxOutputTokens.toLocaleString()}
                      </span>
                      {model.toolCalling ? <span>Tools</span> : null}
                      {model.vision ? <span>Vision</span> : null}
                      {model.thinking ? <span>Thinking</span> : null}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Switch
                      checked={model.enabled}
                      disabled={Boolean(busy)}
                      onCheckedChange={(checked) =>
                        void toggleModel(model, checked)
                      }
                      aria-label={model.enabled ? copy.enabled : copy.disabled}
                    />
                    <Button
                      size="icon"
                      variant="ghost"
                      disabled={Boolean(busy)}
                      onClick={() => openEditModel(model)}
                      aria-label={copy.edit}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      disabled={Boolean(busy)}
                      onClick={() => void deleteModel(model)}
                      aria-label={copy.delete}
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-4">
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
            disabled={Boolean(busy) || selectedTargets.length === 0}
            onClick={() => void stopManaging()}
          >
            {busy === "remove" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Unplug className="mr-2 h-4 w-4" />
            )}
            {copy.stopManaging}
          </Button>
          <Button
            disabled={
              Boolean(busy) ||
              selectedTargets.length === 0 ||
              state.managedModelCount === 0
            }
            onClick={() => void sync()}
          >
            {busy === "sync" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Save className="mr-2 h-4 w-4" />
            )}
            {copy.sync}
          </Button>
        </div>
      </div>

      <CopilotByokModelDialog
        open={editorOpen}
        model={editingModel}
        saving={busy === "model"}
        onOpenChange={setEditorOpen}
        onSave={saveModel}
      />
    </div>
  );
}
