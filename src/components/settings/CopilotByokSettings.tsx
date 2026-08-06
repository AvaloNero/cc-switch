import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  Check,
  KeyRound,
  Loader2,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  RotateCcw,
  Server,
  ShieldCheck,
  Square,
  Trash2,
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
    title: "VS Code Copilot BYOK · 切换代理版",
    description:
      "VS Code 始终选择固定模型“CC Switch Current”；在这里切换当前供应商后，后续 Copilot Chat 请求会透明转发到新的上游。",
    limitationTitle: "作用范围",
    limitation:
      "该集成只接管 VS Code Copilot 的 Custom Endpoint Chat 模型，不接管行内代码补全、Next Edit Suggestions 或 Embeddings。上游必须兼容 OpenAI Chat Completions。",
    securityTitle: "密钥隔离",
    security:
      "VS Code 的 chatLanguageModels.json 只保存随机本地网关令牌；真实上游 API Key 保存在 CC Switch 配置中。",
    refresh: "刷新",
    loading: "正在读取 Copilot 代理配置…",
    integrationOn: "集成已启用",
    integrationOff: "集成未启用",
    serverRunning: "代理运行中",
    serverStopped: "代理未运行",
    fixedModel: "固定模型",
    localEndpoint: "本地端点",
    targets: "VS Code Profile",
    targetsDescription:
      "选择需要写入固定 CC Switch 模型的 Profile。集成启用后，目标变化会立即同步。",
    noTargets:
      "没有检测到 VS Code Stable 或 Insiders，可手动添加 chatLanguageModels.json 路径。",
    defaultProfile: "默认 Profile",
    configExists: "已有配置",
    backupExists: "已有备份",
    managedGroups: "代理模型组",
    restore: "恢复备份",
    removeCustom: "移除目标",
    customName: "目标名称（可选）",
    customPath: "chatLanguageModels.json 的绝对路径",
    addTarget: "添加并选中",
    providers: "代理供应商",
    providersDescription:
      "当前供应商决定固定模型实际请求的上游端点与模型 ID。切换无需修改 VS Code 选择。",
    addProvider: "添加供应商",
    noProviders: "尚未配置代理供应商。",
    current: "当前",
    enabled: "启用",
    disabled: "停用",
    select: "设为当前",
    edit: "编辑",
    delete: "删除",
    start: "启用并写入 VS Code",
    resync: "重新写入固定模型",
    stop: "停止接管并移除固定模型",
    providerSaved: "供应商已保存",
    providerDeleted: "供应商已删除",
    providerSelected: "当前供应商已切换",
    targetsUpdated: "Profile 选择已更新",
    started: "Copilot 切换代理已启用",
    stopped: "Copilot 切换代理已停止",
    restored: "Profile 备份已恢复",
    restoreNoop: "没有可恢复的备份或代理模型组",
    targetAdded: "自定义 Profile 已添加",
    targetRemoved: "自定义 Profile 已移除",
    changedFiles: "更新文件",
    confirmDelete: "确定删除这个代理供应商吗？",
    confirmStop:
      "确定从所选 Profile 移除固定 CC Switch 模型并停止本地代理吗？其他 BYOK 配置不会受影响。",
  },
  en: {
    title: "VS Code Copilot BYOK · Switch Proxy",
    description:
      "Select the fixed “CC Switch Current” model once in VS Code. Switching the current provider here transparently reroutes subsequent Copilot Chat requests.",
    limitationTitle: "Scope",
    limitation:
      "This integration only controls VS Code Copilot Custom Endpoint chat models. It does not affect inline completions, Next Edit Suggestions, or embeddings. Upstreams must support OpenAI Chat Completions.",
    securityTitle: "Credential isolation",
    security:
      "VS Code chatLanguageModels.json stores only a random local gateway token. Real upstream API keys remain in CC Switch configuration.",
    refresh: "Refresh",
    loading: "Reading Copilot proxy configuration…",
    integrationOn: "Integration enabled",
    integrationOff: "Integration disabled",
    serverRunning: "Proxy running",
    serverStopped: "Proxy stopped",
    fixedModel: "Fixed model",
    localEndpoint: "Local endpoint",
    targets: "VS Code profiles",
    targetsDescription:
      "Choose profiles that receive the fixed CC Switch model. Target changes synchronize immediately while integration is enabled.",
    noTargets:
      "No VS Code Stable or Insiders profile was detected. Add an absolute chatLanguageModels.json path manually.",
    defaultProfile: "Default profile",
    configExists: "Config exists",
    backupExists: "Backup exists",
    managedGroups: "Proxy groups",
    restore: "Restore backup",
    removeCustom: "Remove target",
    customName: "Target name (optional)",
    customPath: "Absolute path to chatLanguageModels.json",
    addTarget: "Add and select",
    providers: "Proxy providers",
    providersDescription:
      "The current provider controls the actual upstream endpoint and model ID behind the fixed VS Code model.",
    addProvider: "Add provider",
    noProviders: "No proxy providers configured yet.",
    current: "Current",
    enabled: "Enabled",
    disabled: "Disabled",
    select: "Make current",
    edit: "Edit",
    delete: "Delete",
    start: "Enable and write to VS Code",
    resync: "Rewrite fixed model",
    stop: "Stop takeover and remove fixed model",
    providerSaved: "Provider saved",
    providerDeleted: "Provider deleted",
    providerSelected: "Current provider switched",
    targetsUpdated: "Profile selection updated",
    started: "Copilot switch proxy enabled",
    stopped: "Copilot switch proxy stopped",
    restored: "Profile backup restored",
    restoreNoop: "No backup or proxy group was available to restore",
    targetAdded: "Custom profile added",
    targetRemoved: "Custom profile removed",
    changedFiles: "Changed files",
    confirmDelete: "Delete this proxy provider?",
    confirmStop:
      "Remove the fixed CC Switch model from selected profiles and stop the local proxy? Other BYOK configuration is preserved.",
  },
} as const;

type BusyAction =
  | "load"
  | "targets"
  | "custom"
  | "provider"
  | "start"
  | "stop"
  | `select:${string}`
  | `restore:${string}`
  | `custom-remove:${string}`
  | null;

function targetTitle(target: CopilotByokTargetState, defaultLabel: string) {
  if (target.source === "custom") return target.profileName;
  const edition = target.editionName ?? "VS Code";
  const profile = target.isDefault ? defaultLabel : target.profileName;
  return `${edition} · ${profile}`;
}

function capabilityLabels(provider: CopilotByokModel) {
  const labels: string[] = [];
  if (provider.toolCalling) labels.push("Tools");
  if (provider.vision) labels.push("Vision");
  if (provider.thinking) labels.push("Thinking");
  if (provider.streaming) labels.push("Streaming");
  return labels;
}

export function CopilotByokSettings() {
  const { i18n } = useTranslation();
  const copy = i18n.resolvedLanguage?.startsWith("zh") ? COPY.zh : COPY.en;
  const [state, setState] = useState<CopilotByokState | null>(null);
  const [busy, setBusy] = useState<BusyAction>("load");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingProvider, setEditingProvider] =
    useState<CopilotByokModel | null>(null);
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
        console.error("[CopilotByokSettings] Failed to load proxy state", error);
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
  const currentProvider = useMemo(
    () =>
      state?.models.find(
        (provider) => provider.id === state.currentProviderId,
      ) ?? null,
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
      toast.success(copy.targetsUpdated);
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
      toast.success(copy.targetAdded);
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
      toast.success(copy.targetRemoved);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const saveProvider = async (provider: CopilotByokModel) => {
    setBusy("provider");
    try {
      const next = await copilotByokApi.upsertModel(provider);
      setState(next);
      setEditorOpen(false);
      setEditingProvider(null);
      toast.success(copy.providerSaved);
    } catch (error) {
      toast.error(String(error));
      throw error;
    } finally {
      setBusy(null);
    }
  };

  const toggleProvider = async (
    provider: CopilotByokModel,
    enabled: boolean,
  ) => {
    if (busy) return;
    setBusy("provider");
    try {
      const next = await copilotByokApi.upsertModel({ ...provider, enabled });
      setState(next);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const deleteProvider = async (provider: CopilotByokModel) => {
    if (busy || !window.confirm(copy.confirmDelete)) return;
    setBusy("provider");
    try {
      const next = await copilotByokApi.deleteModel(provider.id);
      setState(next);
      toast.success(copy.providerDeleted);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(null);
    }
  };

  const selectProvider = async (provider: CopilotByokModel) => {
    if (busy || !provider.enabled) return;
    setBusy(`select:${provider.id}`);
    try {
      await copilotByokApi.selectProvider(provider.id);
      toast.success(copy.providerSelected);
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const startIntegration = async () => {
    if (busy) return;
    setBusy("start");
    try {
      const result = await copilotByokApi.start();
      toast.success(
        `${copy.started} · ${copy.changedFiles}: ${result.changedTargetCount}`,
      );
      await load();
    } catch (error) {
      toast.error(String(error));
      setBusy(null);
    }
  };

  const stopIntegration = async () => {
    if (!state || busy || !window.confirm(copy.confirmStop)) return;
    setBusy("stop");
    try {
      const result = await copilotByokApi.stop(state.selectedTargetIds);
      toast.success(
        `${copy.stopped} · ${copy.changedFiles}: ${result.changedTargetCount}`,
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
      toast.success(restored ? copy.restored : copy.restoreNoop);
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
        <AlertTitle>{copy.loading}</AlertTitle>
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
            <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
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

      <div className="grid gap-3 md:grid-cols-2">
        <Alert>
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>{copy.limitationTitle}</AlertTitle>
          <AlertDescription>{copy.limitation}</AlertDescription>
        </Alert>
        <Alert>
          <ShieldCheck className="h-4 w-4" />
          <AlertTitle>{copy.securityTitle}</AlertTitle>
          <AlertDescription>{copy.security}</AlertDescription>
        </Alert>
      </div>

      <Card>
        <CardContent className="grid gap-4 pt-6 md:grid-cols-2 xl:grid-cols-4">
          <div>
            <p className="text-xs text-muted-foreground">Status</p>
            <div className="mt-2 flex flex-wrap gap-2">
              <Badge variant={state.integrationEnabled ? "secondary" : "outline"}>
                {state.integrationEnabled
                  ? copy.integrationOn
                  : copy.integrationOff}
              </Badge>
              <Badge variant={state.serverRunning ? "secondary" : "outline"}>
                {state.serverRunning
                  ? copy.serverRunning
                  : copy.serverStopped}
              </Badge>
            </div>
          </div>
          <div>
            <p className="text-xs text-muted-foreground">{copy.fixedModel}</p>
            <p className="mt-2 font-mono text-sm">{state.fixedModelId}</p>
          </div>
          <div className="md:col-span-2">
            <p className="text-xs text-muted-foreground">{copy.localEndpoint}</p>
            <p className="mt-2 break-all font-mono text-sm">
              {state.proxyUrl}
            </p>
          </div>
        </CardContent>
      </Card>

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
                          {target.readError}
                        </p>
                      ) : null}
                    </div>
                  </div>
                  <div className="flex shrink-0 flex-wrap items-center gap-2">
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
              <Label htmlFor="copilot-proxy-target-name">
                {copy.customName}
              </Label>
              <Input
                id="copilot-proxy-target-name"
                value={customName}
                onChange={(event) => setCustomName(event.target.value)}
                disabled={Boolean(busy)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-proxy-target-path">
                {copy.customPath}
              </Label>
              <Input
                id="copilot-proxy-target-path"
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
                {copy.providers}
              </CardTitle>
              <p className="mt-1 text-sm text-muted-foreground">
                {copy.providersDescription}
              </p>
            </div>
            <Button
              size="sm"
              onClick={() => {
                setEditingProvider(null);
                setEditorOpen(true);
              }}
              disabled={Boolean(busy)}
            >
              <Plus className="mr-2 h-4 w-4" />
              {copy.addProvider}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {state.models.length === 0 ? (
            <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              {copy.noProviders}
            </p>
          ) : (
            state.models.map((provider) => {
              const isCurrent = provider.id === state.currentProviderId;
              const capabilities = capabilityLabels(provider);
              return (
                <div
                  key={provider.id}
                  className={`flex flex-col gap-3 rounded-lg border p-4 lg:flex-row lg:items-center lg:justify-between ${
                    isCurrent ? "border-primary/50 bg-primary/5" : ""
                  }`}
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{provider.name}</span>
                      {isCurrent ? (
                        <Badge variant="secondary">
                          <Check className="mr-1 h-3 w-3" />
                          {copy.current}
                        </Badge>
                      ) : null}
                      <Badge variant={provider.enabled ? "outline" : "secondary"}>
                        {provider.enabled ? copy.enabled : copy.disabled}
                      </Badge>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {provider.modelId}
                    </p>
                    <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                      {provider.endpoint}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                      <span>
                        Context: {provider.contextWindow.toLocaleString()}
                      </span>
                      <span>
                        Output: {provider.maxOutputTokens.toLocaleString()}
                      </span>
                      {capabilities.map((label) => (
                        <span key={label}>{label}</span>
                      ))}
                    </div>
                  </div>
                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    <Switch
                      checked={provider.enabled}
                      disabled={Boolean(busy)}
                      onCheckedChange={(checked) =>
                        void toggleProvider(provider, checked)
                      }
                      aria-label={`${provider.name} ${copy.enabled}`}
                    />
                    {!isCurrent ? (
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={Boolean(busy) || !provider.enabled}
                        onClick={() => void selectProvider(provider)}
                      >
                        {busy === `select:${provider.id}` ? (
                          <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Check className="mr-2 h-3.5 w-3.5" />
                        )}
                        {copy.select}
                      </Button>
                    ) : null}
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={Boolean(busy)}
                      onClick={() => {
                        setEditingProvider(provider);
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
                      onClick={() => void deleteProvider(provider)}
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
        <div className="text-sm text-muted-foreground">
          {currentProvider ? (
            <span>
              {copy.current}: {currentProvider.name} / {currentProvider.modelId}
            </span>
          ) : (
            <span>{copy.noProviders}</span>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() => void startIntegration()}
            disabled={
              Boolean(busy) ||
              selectedTargets.length === 0 ||
              !currentProvider ||
              !currentProvider.enabled
            }
          >
            {busy === "start" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : state.integrationEnabled ? (
              <RefreshCw className="mr-2 h-4 w-4" />
            ) : (
              <Play className="mr-2 h-4 w-4" />
            )}
            {state.integrationEnabled ? copy.resync : copy.start}
          </Button>
          <Button
            variant="destructive"
            onClick={() => void stopIntegration()}
            disabled={Boolean(busy) || !state.integrationEnabled}
          >
            {busy === "stop" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Square className="mr-2 h-4 w-4" />
            )}
            {copy.stop}
          </Button>
        </div>
      </div>

      <CopilotByokModelDialog
        open={editorOpen}
        model={editingProvider}
        saving={busy === "provider"}
        onOpenChange={(open) => {
          setEditorOpen(open);
          if (!open) setEditingProvider(null);
        }}
        onSave={saveProvider}
      />
    </div>
  );
}
