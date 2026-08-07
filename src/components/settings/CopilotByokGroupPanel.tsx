import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type FormEvent,
} from "react";
import {
  ChevronRight,
  Download,
  Loader2,
  Plus,
  Save,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import ApiKeyInput from "@/components/providers/forms/ApiKeyInput";
import { ProviderPresetSelector } from "@/components/providers/forms/ProviderPresetSelector";
import {
  ModelDropdown,
  ProviderFormLayout,
  ProviderHeadersEditor,
  ProviderIdentityFields,
  PROVIDER_HEADER_DRAFT_PREFIX,
} from "@/components/providers/forms/shared";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  fetchModelsForConfig,
  showFetchModelsError,
  type FetchedModel,
} from "@/lib/api/model-fetch";
import type {
  CopilotByokApiType,
  CopilotByokEditTool,
  CopilotByokGroup,
  CopilotByokModel,
} from "@/lib/api";
import { cn } from "@/lib/utils";

const EDIT_TOOLS: CopilotByokEditTool[] = [
  "find-replace",
  "multi-find-replace",
  "apply-patch",
  "code-rewrite",
];

type DraftModel = CopilotByokModel & {
  reasoningEffortsText: string;
  modelOptionsText: string;
};

type DraftGroup = Omit<CopilotByokGroup, "models"> & {
  models: DraftModel[];
};

const emptyModel = (): DraftModel => ({
  id: crypto.randomUUID(),
  modelId: "",
  name: "",
  enabled: true,
  toolCalling: true,
  vision: false,
  thinking: true,
  streaming: true,
  contextWindow: 128000,
  maxInputTokens: null,
  maxOutputTokens: 8192,
  editTools: [],
  zeroDataRetentionEnabled: false,
  supportsReasoningEffort: [],
  reasoningEffortFormat: null,
  modelOptions: {},
  extra: {},
  reasoningEffortsText: "",
  modelOptionsText: "{}",
});

const emptyGroup = (): DraftGroup => ({
  id: crypto.randomUUID(),
  name: "",
  url: "",
  apiKey: "",
  apiType: "chat-completions",
  websiteUrl: null,
  notes: null,
  icon: null,
  iconColor: null,
  enabled: true,
  requestHeaders: {},
  models: [emptyModel()],
  extra: {},
});

function toDraft(group: CopilotByokGroup | null): DraftGroup {
  if (!group) return emptyGroup();
  return {
    ...structuredClone(group),
    requestHeaders: structuredClone(group.requestHeaders ?? {}),
    models: group.models.map((model) => ({
      ...structuredClone(model),
      reasoningEffortsText: model.supportsReasoningEffort.join(", "),
      modelOptionsText: JSON.stringify(model.modelOptions ?? {}, null, 2),
    })),
  };
}

interface CopilotByokGroupPanelProps {
  open: boolean;
  group: CopilotByokGroup | null;
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (group: CopilotByokGroup) => Promise<void>;
}

export function CopilotByokGroupPanel({
  open,
  group,
  saving,
  onOpenChange,
  onSave,
}: CopilotByokGroupPanelProps) {
  const { t } = useTranslation();
  const copy = {
    addTitle: t("provider.addNewProvider"),
    editTitle: t("provider.editProvider"),
    hint: t("provider.addFooterHint"),
    url: t("opencode.baseUrl"),
    urlPlaceholder: t("copilotByok.form.urlPlaceholder"),
    apiType: t("copilotByok.form.apiType"),
    apiKey: t("copilotByok.form.apiKey"),
    sharedConnection: t("opencode.baseUrlHint"),
    headers: t("opencode.headers"),
    headersDescription: t("opencode.headersHint"),
    noHeaders: t("opencode.noHeaders"),
    addHeader: t("opencode.addHeader"),
    headerName: t("opencode.headerName"),
    headerValue: t("opencode.headerValue"),
    removeHeader: t("opencode.removeHeader"),
    models: t("opencode.models"),
    modelDescription: t("opencode.modelsHint"),
    addModel: t("opencode.addModel"),
    modelId: t("opencode.modelId"),
    modelName: t("opencode.modelName"),
    modelDetails: t("opencode.toggleModelDetails"),
    enabled: t("common.enabled"),
    tokenLimits: t("opencode.modelLimits"),
    context: t("opencode.limitContext"),
    maxInput: t("copilotByok.form.maxInput"),
    maxOutput: t("opencode.limitOutput"),
    automatic: t("common.auto"),
    capabilities: t("opencode.modelExtraFields"),
    toolCalling: t("copilotByok.form.toolCalling"),
    vision: t("copilotByok.form.vision"),
    thinking: t("copilotByok.form.thinking"),
    streaming: t("copilotByok.form.streaming"),
    zeroData: t("copilotByok.form.zeroDataRetention"),
    editTools: t("copilotByok.form.editTools"),
    reasoning: t("copilotByok.form.reasoningEfforts"),
    reasoningPlaceholder: t("copilotByok.form.reasoningEffortsPlaceholder"),
    modelOptions: t("opencode.extraOptions"),
    chatCompletions: t("copilotByok.form.chatCompletions"),
    responses: t("copilotByok.form.responses"),
    messages: t("copilotByok.form.messages"),
    modelIdPlaceholder: t("copilotByok.form.modelIdPlaceholder"),
    modelNamePlaceholder: t("copilotByok.form.modelNamePlaceholder"),
    cancel: t("common.cancel"),
    add: t("provider.addToConfig"),
    save: t("common.save"),
    duplicateModelId: t("opencode.providerKeyDuplicate"),
    invalidOptions: t("jsonEditor.mustBeObject"),
  };
  const [draft, setDraft] = useState<DraftGroup>(emptyGroup);
  const [expandedModels, setExpandedModels] = useState<Set<string>>(new Set());
  const [fetchedModels, setFetchedModels] = useState<FetchedModel[]>([]);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const next = toDraft(group);
    setDraft(next);
    setExpandedModels(new Set());
    setFetchedModels([]);
    setIsFetchingModels(false);
    setError(null);
  }, [group, open]);

  const handleFetchModels = useCallback(() => {
    const baseUrl = draft.url.trim();
    const apiKey = draft.apiKey.trim();
    if (!baseUrl) {
      showFetchModelsError(null, t, {
        hasApiKey: true,
        hasBaseUrl: Boolean(baseUrl),
      });
      return;
    }

    setIsFetchingModels(true);
    const requestHeaders = Object.fromEntries(
      Object.entries(draft.requestHeaders).filter(
        ([key]) => !key.startsWith(PROVIDER_HEADER_DRAFT_PREFIX),
      ),
    );
    fetchModelsForConfig(
      baseUrl,
      apiKey,
      false,
      undefined,
      undefined,
      draft.apiType,
      requestHeaders,
    )
      .then((models) => {
        setFetchedModels(models);
        if (models.length === 0) {
          toast.info(t("providerForm.fetchModelsEmpty"));
        } else {
          toast.success(
            t("providerForm.fetchModelsSuccess", { count: models.length }),
          );
        }
      })
      .catch((fetchError) => {
        console.warn("[ModelFetch] Failed:", fetchError);
        showFetchModelsError(fetchError, t);
      })
      .finally(() => setIsFetchingModels(false));
  }, [draft.apiKey, draft.apiType, draft.requestHeaders, draft.url, t]);

  const canSave = useMemo(
    () =>
      draft.name.trim().length > 0 &&
      draft.url.trim().length > 0 &&
      draft.models.length > 0 &&
      draft.models.every(
        (model) => model.modelId.trim() && model.name.trim(),
      ) &&
      !saving,
    [draft, saving],
  );

  const updateGroup = <K extends keyof DraftGroup>(
    key: K,
    value: DraftGroup[K],
  ) => setDraft((current) => ({ ...current, [key]: value }));

  const updateModel = <K extends keyof DraftModel>(
    id: string,
    key: K,
    value: DraftModel[K],
  ) =>
    setDraft((current) => ({
      ...current,
      models: current.models.map((model) =>
        model.id === id ? { ...model, [key]: value } : model,
      ),
    }));

  const updateModelId = (
    id: string,
    modelId: string,
    fetchedModel?: FetchedModel,
  ) =>
    setDraft((current) => ({
      ...current,
      models: current.models.map((model) => {
        if (model.id !== id) return model;
        const shouldSyncName =
          Boolean(fetchedModel) ||
          !model.name.trim() ||
          model.name === model.modelId;
        return {
          ...model,
          modelId,
          name: shouldSyncName
            ? fetchedModel?.name?.trim() || modelId
            : model.name,
        };
      }),
    }));

  const removeModel = (id: string) => {
    setDraft((current) => ({
      ...current,
      models: current.models.filter((model) => model.id !== id),
    }));
    setExpandedModels((current) => {
      const next = new Set(current);
      next.delete(id);
      return next;
    });
  };

  const addModel = () => {
    const model = emptyModel();
    setDraft((current) => ({
      ...current,
      models: [...current.models, model],
    }));
  };

  const toggleExpanded = (id: string) =>
    setExpandedModels((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const toggleEditTool = (
    model: DraftModel,
    tool: CopilotByokEditTool,
    checked: boolean,
  ) =>
    updateModel(
      model.id,
      "editTools",
      checked
        ? [...new Set([...model.editTools, tool])]
        : model.editTools.filter((item) => item !== tool),
    );

  const parsePositiveInteger = (value: string, fallback: number): number => {
    const parsed = Number(value);
    return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
  };

  const parseObject = (text: string): Record<string, unknown> => {
    const parsed = JSON.parse(text || "{}") as unknown;
    if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
      throw new Error(copy.invalidOptions);
    }
    return parsed as Record<string, unknown>;
  };

  const handleSave = async () => {
    setError(null);
    try {
      const normalizedIds = draft.models.map((model) =>
        model.modelId.trim().toLowerCase(),
      );
      if (new Set(normalizedIds).size !== normalizedIds.length) {
        throw new Error(copy.duplicateModelId);
      }
      const requestHeaders = Object.fromEntries(
        Object.entries(draft.requestHeaders).filter(
          ([key]) => !key.startsWith(PROVIDER_HEADER_DRAFT_PREFIX),
        ),
      );
      const models: CopilotByokModel[] = draft.models.map((model) => {
        const supportsReasoningEffort = [
          ...new Set(
            model.reasoningEffortsText
              .split(",")
              .map((value) => value.trim())
              .filter(Boolean),
          ),
        ];
        const {
          reasoningEffortsText: _reasoningEffortsText,
          modelOptionsText: _modelOptionsText,
          ...savedModel
        } = model;
        return {
          ...savedModel,
          modelId: model.modelId.trim(),
          name: model.name.trim(),
          supportsReasoningEffort,
          reasoningEffortFormat:
            supportsReasoningEffort.length > 0
              ? model.reasoningEffortFormat || draft.apiType
              : null,
          modelOptions: parseObject(model.modelOptionsText),
        };
      });
      await onSave({
        ...draft,
        id: draft.id,
        name: draft.name.trim(),
        url: draft.url.trim(),
        apiKey: draft.apiKey.trim(),
        apiType: draft.apiType,
        websiteUrl: draft.websiteUrl?.trim() || null,
        notes: draft.notes?.trim() || null,
        icon: draft.icon?.trim() || null,
        iconColor: draft.iconColor?.trim() || null,
        enabled: draft.enabled,
        requestHeaders,
        models,
      });
    } catch (saveError) {
      setError(
        saveError instanceof Error ? saveError.message : String(saveError),
      );
    }
  };

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (canSave) void handleSave();
  };

  const footer = (
    <>
      <span className="mr-auto min-w-0 truncate text-xs text-muted-foreground">
        {copy.hint}
      </span>
      <Button
        type="button"
        variant="outline"
        onClick={() => onOpenChange(false)}
        disabled={saving}
        className="border-border/20 hover:bg-accent hover:text-accent-foreground"
      >
        {copy.cancel}
      </Button>
      <Button
        type="submit"
        form="copilot-byok-provider-form"
        disabled={!canSave}
        className="bg-primary text-primary-foreground hover:bg-primary/90"
      >
        {saving ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : group ? (
          <Save className="mr-2 h-4 w-4" />
        ) : (
          <Plus className="mr-2 h-4 w-4" />
        )}
        {group ? copy.save : copy.add}
      </Button>
    </>
  );

  return (
    <FullScreenPanel
      isOpen={open}
      title={group ? copy.editTitle : copy.addTitle}
      onClose={() => onOpenChange(false)}
      footer={footer}
      contentClassName="pt-3"
    >
      <ProviderFormLayout
        id="copilot-byok-provider-form"
        onSubmit={handleSubmit}
      >
        {!group && (
          <ProviderPresetSelector
            selectedPresetId="custom"
            presetEntries={[]}
            presetCategoryLabels={{}}
            onPresetChange={() => undefined}
            category="custom"
          />
        )}

        <ProviderIdentityFields
          name={draft.name}
          notes={draft.notes}
          websiteUrl={draft.websiteUrl}
          icon={draft.icon}
          iconColor={draft.iconColor}
          onNameChange={(value) => updateGroup("name", value)}
          onNotesChange={(value) => updateGroup("notes", value)}
          onWebsiteUrlChange={(value) => updateGroup("websiteUrl", value)}
          onIconChange={(icon, color) => {
            updateGroup("icon", icon);
            updateGroup("iconColor", color);
          }}
        />

        <div className="space-y-2">
          <Label htmlFor="copilot-byok-api-type">{copy.apiType}</Label>
          <Select
            value={draft.apiType}
            onValueChange={(value) =>
              updateGroup("apiType", value as CopilotByokApiType)
            }
          >
            <SelectTrigger id="copilot-byok-api-type">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="chat-completions">
                {copy.chatCompletions}
              </SelectItem>
              <SelectItem value="responses">{copy.responses}</SelectItem>
              <SelectItem value="messages">{copy.messages}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <ApiKeyInput
          id="copilot-byok-api-key"
          label={copy.apiKey}
          value={draft.apiKey}
          onChange={(value) => updateGroup("apiKey", value)}
        />

        <div className="space-y-2">
          <Label htmlFor="copilot-byok-url">{copy.url}</Label>
          <Input
            id="copilot-byok-url"
            value={draft.url}
            onChange={(event) => updateGroup("url", event.target.value)}
            placeholder={copy.urlPlaceholder}
          />
          <p className="text-xs text-muted-foreground">
            {copy.sharedConnection}
          </p>
        </div>

        <ProviderHeadersEditor
          headers={draft.requestHeaders}
          onHeadersChange={(headers) => updateGroup("requestHeaders", headers)}
          label={copy.headers}
          hint={copy.headersDescription}
          emptyText={copy.noHeaders}
          addLabel={copy.addHeader}
          addAriaLabel={copy.addHeader}
          nameLabel={copy.headerName}
          valueLabel={copy.headerValue}
          namePlaceholder={t("opencode.headerNamePlaceholder")}
          valuePlaceholder={t("opencode.headerValuePlaceholder")}
          removeAriaLabel={copy.removeHeader}
        />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <Label>{copy.models}</Label>
            <div className="flex gap-1">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleFetchModels}
                disabled={isFetchingModels}
                className="h-7 gap-1"
              >
                {isFetchingModels ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Download className="h-3.5 w-3.5" />
                )}
                {t("providerForm.fetchModels")}
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={addModel}
                className="h-7 gap-1"
              >
                <Plus className="h-3.5 w-3.5" />
                {copy.addModel}
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <div className="mb-1 flex items-center gap-2 px-1 text-xs text-muted-foreground">
              <span className="w-9" />
              <span className="flex-1">{copy.modelId}</span>
              <span className="flex-1">{copy.modelName}</span>
              <span className="w-[4.75rem] text-center">{copy.enabled}</span>
              <span className="w-9" />
            </div>
            {draft.models.map((model) => (
              <div key={model.id} className="space-y-2">
                <div className="flex items-center gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => toggleExpanded(model.id)}
                    aria-label={copy.modelDetails}
                    className="h-9 w-9 shrink-0"
                  >
                    <ChevronRight
                      className={cn(
                        "h-4 w-4 transition-transform",
                        expandedModels.has(model.id) && "rotate-90",
                      )}
                    />
                  </Button>
                  <div className="flex min-w-0 flex-1 gap-1">
                    <Input
                      value={model.modelId}
                      onChange={(event) =>
                        updateModelId(model.id, event.target.value)
                      }
                      placeholder={copy.modelIdPlaceholder}
                      className="min-w-0 flex-1"
                    />
                    {fetchedModels.length > 0 && (
                      <ModelDropdown
                        models={fetchedModels}
                        onSelect={(id) =>
                          updateModelId(
                            model.id,
                            id,
                            fetchedModels.find((item) => item.id === id),
                          )
                        }
                      />
                    )}
                  </div>
                  <Input
                    value={model.name}
                    onChange={(event) =>
                      updateModel(model.id, "name", event.target.value)
                    }
                    placeholder={copy.modelNamePlaceholder}
                    className="min-w-0 flex-1"
                  />
                  <div className="flex w-[4.75rem] justify-center">
                    <Switch
                      checked={model.enabled}
                      onCheckedChange={(checked) =>
                        updateModel(model.id, "enabled", checked)
                      }
                      aria-label={`${model.name || model.modelId} ${copy.enabled}`}
                    />
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => removeModel(model.id)}
                    disabled={draft.models.length === 1}
                    className="h-9 w-9 shrink-0 text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>

                {expandedModels.has(model.id) && (
                  <div className="ml-9 space-y-4 border-l-2 border-muted pl-4">
                    <div className="space-y-2">
                      <span className="text-xs font-medium text-muted-foreground">
                        {copy.tokenLimits}
                      </span>
                      <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
                        <div className="space-y-1">
                          <Label className="text-xs text-muted-foreground">
                            {copy.context}
                          </Label>
                          <Input
                            type="number"
                            min={1}
                            value={model.contextWindow}
                            onChange={(event) =>
                              updateModel(
                                model.id,
                                "contextWindow",
                                parsePositiveInteger(
                                  event.target.value,
                                  128000,
                                ),
                              )
                            }
                          />
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs text-muted-foreground">
                            {copy.maxInput}
                          </Label>
                          <Input
                            type="number"
                            min={1}
                            value={model.maxInputTokens ?? ""}
                            placeholder={copy.automatic}
                            onChange={(event) =>
                              updateModel(
                                model.id,
                                "maxInputTokens",
                                event.target.value
                                  ? parsePositiveInteger(event.target.value, 1)
                                  : null,
                              )
                            }
                          />
                        </div>
                        <div className="space-y-1">
                          <Label className="text-xs text-muted-foreground">
                            {copy.maxOutput}
                          </Label>
                          <Input
                            type="number"
                            min={1}
                            value={model.maxOutputTokens}
                            onChange={(event) =>
                              updateModel(
                                model.id,
                                "maxOutputTokens",
                                parsePositiveInteger(event.target.value, 8192),
                              )
                            }
                          />
                        </div>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <span className="text-xs font-medium text-muted-foreground">
                        {copy.capabilities}
                      </span>
                      <div className="grid grid-cols-1 gap-x-6 gap-y-2 sm:grid-cols-2">
                        {(
                          [
                            ["toolCalling", copy.toolCalling],
                            ["vision", copy.vision],
                            ["thinking", copy.thinking],
                            ["streaming", copy.streaming],
                            ["zeroDataRetentionEnabled", copy.zeroData],
                          ] as const
                        ).map(([key, label]) => (
                          <label
                            key={key}
                            className="flex items-center justify-between gap-3 text-sm"
                          >
                            <span>{label}</span>
                            <Switch
                              checked={model[key]}
                              onCheckedChange={(checked) =>
                                updateModel(model.id, key, checked)
                              }
                            />
                          </label>
                        ))}
                      </div>
                    </div>

                    <div className="space-y-2">
                      <span className="text-xs font-medium text-muted-foreground">
                        {copy.editTools}
                      </span>
                      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                        {EDIT_TOOLS.map((tool) => (
                          <label
                            key={tool}
                            className="flex items-center gap-2 text-sm"
                          >
                            <Checkbox
                              checked={model.editTools.includes(tool)}
                              onCheckedChange={(checked) =>
                                toggleEditTool(model, tool, checked === true)
                              }
                            />
                            <span>{tool}</span>
                          </label>
                        ))}
                      </div>
                    </div>

                    <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">
                          {copy.reasoning}
                        </Label>
                        <Input
                          value={model.reasoningEffortsText}
                          onChange={(event) =>
                            updateModel(
                              model.id,
                              "reasoningEffortsText",
                              event.target.value,
                            )
                          }
                          placeholder={copy.reasoningPlaceholder}
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">
                          {copy.modelOptions}
                        </Label>
                        <Textarea
                          value={model.modelOptionsText}
                          onChange={(event) =>
                            updateModel(
                              model.id,
                              "modelOptionsText",
                              event.target.value,
                            )
                          }
                          rows={3}
                          className="font-mono text-xs"
                          spellCheck={false}
                        />
                      </div>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>

          <p className="text-xs text-muted-foreground">
            {copy.modelDescription}
          </p>
        </div>

        {error ? (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
            {error}
          </p>
        ) : null}
      </ProviderFormLayout>
    </FullScreenPanel>
  );
}
