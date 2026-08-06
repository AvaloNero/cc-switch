import { useEffect, useMemo, useState } from "react";
import { Eye, EyeOff, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import type {
  CopilotByokApiType,
  CopilotByokEditTool,
  CopilotByokModel,
} from "@/lib/api";

const EDIT_TOOLS: CopilotByokEditTool[] = [
  "find-replace",
  "multi-find-replace",
  "apply-patch",
  "code-rewrite",
];

const emptyModel = (): CopilotByokModel => ({
  id: crypto.randomUUID(),
  modelId: "",
  name: "",
  url: "",
  apiKey: "",
  apiType: "chat-completions",
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
  requestHeaders: {},
  modelOptions: {},
});

interface CopilotByokModelDialogProps {
  open: boolean;
  model: CopilotByokModel | null;
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (model: CopilotByokModel) => Promise<void>;
}

export function CopilotByokModelDialog({
  open,
  model,
  saving,
  onOpenChange,
  onSave,
}: CopilotByokModelDialogProps) {
  const [draft, setDraft] = useState<CopilotByokModel>(emptyModel);
  const [showApiKey, setShowApiKey] = useState(false);
  const [headersText, setHeadersText] = useState("{}");
  const [modelOptionsText, setModelOptionsText] = useState("{}");
  const [reasoningEffortsText, setReasoningEffortsText] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const next = model ? structuredClone(model) : emptyModel();
    setDraft(next);
    setHeadersText(JSON.stringify(next.requestHeaders ?? {}, null, 2));
    setModelOptionsText(JSON.stringify(next.modelOptions ?? {}, null, 2));
    setReasoningEffortsText(next.supportsReasoningEffort.join(", "));
    setError(null);
    setShowApiKey(false);
  }, [model, open]);

  const title = model ? "编辑 Copilot BYOK 模型" : "添加 Copilot BYOK 模型";

  const canSave = useMemo(
    () =>
      draft.name.trim().length > 0 &&
      draft.modelId.trim().length > 0 &&
      draft.url.trim().length > 0 &&
      draft.apiKey.trim().length > 0 &&
      !saving,
    [draft.apiKey, draft.modelId, draft.name, draft.url, saving],
  );

  const update = <K extends keyof CopilotByokModel>(
    key: K,
    value: CopilotByokModel[K],
  ) => setDraft((current) => ({ ...current, [key]: value }));

  const parsePositiveInteger = (
    value: string,
    fallback: number,
  ): number => {
    const parsed = Number.parseInt(value, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
  };

  const handleSave = async () => {
    setError(null);
    try {
      const requestHeaders = JSON.parse(headersText || "{}") as unknown;
      const modelOptions = JSON.parse(modelOptionsText || "{}") as unknown;
      if (
        !requestHeaders ||
        Array.isArray(requestHeaders) ||
        typeof requestHeaders !== "object"
      ) {
        throw new Error("请求头必须是 JSON 对象");
      }
      if (
        !modelOptions ||
        Array.isArray(modelOptions) ||
        typeof modelOptions !== "object"
      ) {
        throw new Error("Model Options 必须是 JSON 对象");
      }
      const headers = Object.fromEntries(
        Object.entries(requestHeaders as Record<string, unknown>).map(
          ([key, value]) => [key, String(value)],
        ),
      );
      const efforts = reasoningEffortsText
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);

      await onSave({
        ...draft,
        name: draft.name.trim(),
        modelId: draft.modelId.trim(),
        url: draft.url.trim(),
        apiKey: draft.apiKey.trim(),
        requestHeaders: headers,
        modelOptions,
        supportsReasoningEffort: [...new Set(efforts)],
      });
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : String(saveError));
    }
  };

  const toggleEditTool = (tool: CopilotByokEditTool, checked: boolean) => {
    update(
      "editTools",
      checked
        ? [...new Set([...draft.editTools, tool])]
        : draft.editTools.filter((item) => item !== tool),
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[88vh] max-w-3xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <div className="grid gap-5 px-1 py-2">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="copilot-byok-name">显示名称</Label>
              <Input
                id="copilot-byok-name"
                value={draft.name}
                onChange={(event) => update("name", event.target.value)}
                placeholder="Kimi K3"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-byok-model-id">Model ID</Label>
              <Input
                id="copilot-byok-model-id"
                value={draft.modelId}
                onChange={(event) => update("modelId", event.target.value)}
                placeholder="kimi-k3"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-byok-url">Endpoint URL</Label>
            <Input
              id="copilot-byok-url"
              value={draft.url}
              onChange={(event) => update("url", event.target.value)}
              placeholder="https://api.example.com/v1/chat/completions"
            />
            <p className="text-xs text-muted-foreground">
              可以填写完整 API 路径；VS Code 也会根据 API 类型补全标准路径。
            </p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="copilot-byok-api-type">API 类型</Label>
              <Select
                value={draft.apiType}
                onValueChange={(value) =>
                  update("apiType", value as CopilotByokApiType)
                }
              >
                <SelectTrigger id="copilot-byok-api-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="chat-completions">
                    Chat Completions
                  </SelectItem>
                  <SelectItem value="responses">Responses</SelectItem>
                  <SelectItem value="messages">Anthropic Messages</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-byok-api-key">API Key</Label>
              <div className="relative">
                <Input
                  id="copilot-byok-api-key"
                  type={showApiKey ? "text" : "password"}
                  value={draft.apiKey}
                  onChange={(event) => update("apiKey", event.target.value)}
                  className="pr-10"
                />
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="absolute right-0 top-0 h-full"
                  onClick={() => setShowApiKey((value) => !value)}
                >
                  {showApiKey ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-2">
              <Label htmlFor="copilot-context-window">Context Window</Label>
              <Input
                id="copilot-context-window"
                type="number"
                min={1}
                value={draft.contextWindow}
                onChange={(event) =>
                  update(
                    "contextWindow",
                    parsePositiveInteger(event.target.value, 128000),
                  )
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-max-input">Max Input Tokens</Label>
              <Input
                id="copilot-max-input"
                type="number"
                min={1}
                value={draft.maxInputTokens ?? ""}
                onChange={(event) =>
                  update(
                    "maxInputTokens",
                    event.target.value
                      ? parsePositiveInteger(event.target.value, 1)
                      : null,
                  )
                }
                placeholder="自动推导"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-max-output">Max Output Tokens</Label>
              <Input
                id="copilot-max-output"
                type="number"
                min={1}
                value={draft.maxOutputTokens}
                onChange={(event) =>
                  update(
                    "maxOutputTokens",
                    parsePositiveInteger(event.target.value, 8192),
                  )
                }
              />
            </div>
          </div>

          <div className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2 lg:grid-cols-3">
            {(
              [
                ["enabled", "启用"],
                ["toolCalling", "Tool Calling"],
                ["vision", "Vision"],
                ["thinking", "Thinking"],
                ["streaming", "Streaming"],
                ["zeroDataRetentionEnabled", "Zero Data Retention"],
              ] as const
            ).map(([key, label]) => (
              <div key={key} className="flex items-center justify-between gap-3">
                <Label htmlFor={`copilot-cap-${key}`}>{label}</Label>
                <Switch
                  id={`copilot-cap-${key}`}
                  checked={Boolean(draft[key])}
                  onCheckedChange={(checked) => update(key, checked)}
                />
              </div>
            ))}
          </div>

          <div className="space-y-3">
            <Label>Edit Tools</Label>
            <div className="grid gap-3 sm:grid-cols-2">
              {EDIT_TOOLS.map((tool) => (
                <label
                  key={tool}
                  className="flex items-center gap-2 rounded-md border p-3 text-sm"
                >
                  <Checkbox
                    checked={draft.editTools.includes(tool)}
                    onCheckedChange={(checked) =>
                      toggleEditTool(tool, checked === true)
                    }
                  />
                  <span>{tool}</span>
                </label>
              ))}
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="copilot-reasoning-efforts">
                Reasoning Efforts
              </Label>
              <Input
                id="copilot-reasoning-efforts"
                value={reasoningEffortsText}
                onChange={(event) => setReasoningEffortsText(event.target.value)}
                placeholder="low, medium, high"
              />
            </div>
            <div className="space-y-2">
              <Label>Reasoning Effort Format</Label>
              <Select
                value={draft.reasoningEffortFormat ?? "auto"}
                onValueChange={(value) =>
                  update(
                    "reasoningEffortFormat",
                    value === "auto"
                      ? null
                      : (value as CopilotByokApiType),
                  )
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">自动推导</SelectItem>
                  <SelectItem value="chat-completions">
                    Chat Completions
                  </SelectItem>
                  <SelectItem value="responses">Responses</SelectItem>
                  <SelectItem value="messages">Messages</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-request-headers">Request Headers (JSON)</Label>
            <Textarea
              id="copilot-request-headers"
              value={headersText}
              onChange={(event) => setHeadersText(event.target.value)}
              rows={5}
              className="font-mono text-xs"
              spellCheck={false}
            />
            <p className="text-xs text-muted-foreground">
              自定义认证头可使用 VS Code 支持的 <code>${"${apiKey}"}</code>{" "}
              占位符。
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-model-options">Model Options (JSON)</Label>
            <Textarea
              id="copilot-model-options"
              value={modelOptionsText}
              onChange={(event) => setModelOptionsText(event.target.value)}
              rows={5}
              className="font-mono text-xs"
              spellCheck={false}
            />
          </div>

          {error ? (
            <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </p>
          ) : null}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={saving}
          >
            取消
          </Button>
          <Button type="button" onClick={handleSave} disabled={!canSave}>
            {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
            保存模型
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
