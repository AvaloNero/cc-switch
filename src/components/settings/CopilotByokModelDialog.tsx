import { useEffect, useMemo, useState } from "react";
import { Eye, EyeOff, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type { CopilotByokModel } from "@/lib/api";

const emptyModel = (): CopilotByokModel => ({
  id: crypto.randomUUID(),
  name: "",
  endpoint: "",
  apiKey: "",
  modelId: "",
  enabled: true,
  requestHeaders: {},
  contextWindow: 262144,
  maxOutputTokens: 32768,
  toolCalling: true,
  vision: false,
  thinking: true,
  streaming: true,
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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const next = model ? structuredClone(model) : emptyModel();
    setDraft(next);
    setHeadersText(JSON.stringify(next.requestHeaders ?? {}, null, 2));
    setShowApiKey(false);
    setError(null);
  }, [model, open]);

  const canSave = useMemo(
    () =>
      draft.name.trim().length > 0 &&
      draft.endpoint.trim().length > 0 &&
      draft.apiKey.trim().length > 0 &&
      draft.modelId.trim().length > 0 &&
      !saving,
    [
      draft.apiKey,
      draft.endpoint,
      draft.modelId,
      draft.name,
      saving,
    ],
  );

  const update = <K extends keyof CopilotByokModel>(
    key: K,
    value: CopilotByokModel[K],
  ) => setDraft((current) => ({ ...current, [key]: value }));

  const parsePositiveInteger = (value: string, fallback: number) => {
    const parsed = Number.parseInt(value, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
  };

  const handleSave = async () => {
    setError(null);
    try {
      const parsedHeaders = JSON.parse(headersText || "{}") as unknown;
      if (
        !parsedHeaders ||
        Array.isArray(parsedHeaders) ||
        typeof parsedHeaders !== "object"
      ) {
        throw new Error("Request Headers 必须是 JSON 对象");
      }
      const requestHeaders = Object.fromEntries(
        Object.entries(parsedHeaders as Record<string, unknown>).map(
          ([name, value]) => [name, String(value)],
        ),
      );

      await onSave({
        ...draft,
        name: draft.name.trim(),
        endpoint: draft.endpoint.trim(),
        apiKey: draft.apiKey.trim(),
        modelId: draft.modelId.trim(),
        requestHeaders,
      });
    } catch (saveError) {
      setError(
        saveError instanceof Error ? saveError.message : String(saveError),
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent zIndex="alert" className="max-h-[90vh] max-w-3xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {model ? "编辑 Copilot 代理供应商" : "添加 Copilot 代理供应商"}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-5 px-6 py-2">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="copilot-proxy-name">显示名称</Label>
              <Input
                id="copilot-proxy-name"
                value={draft.name}
                onChange={(event) => update("name", event.target.value)}
                placeholder="Kimi Code Plan"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-proxy-model">上游模型 ID</Label>
              <Input
                id="copilot-proxy-model"
                value={draft.modelId}
                onChange={(event) => update("modelId", event.target.value)}
                placeholder="kimi-k2.5"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-proxy-endpoint">
              Chat Completions 完整端点
            </Label>
            <Input
              id="copilot-proxy-endpoint"
              value={draft.endpoint}
              onChange={(event) => update("endpoint", event.target.value)}
              placeholder="https://api.example.com/v1/chat/completions"
              className="font-mono text-xs"
            />
            <p className="text-xs text-muted-foreground">
              该版本直接透传 OpenAI Chat Completions 请求；这里填写完整请求 URL，而不是仅填写 Base URL。
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-proxy-key">上游 API Key</Label>
            <div className="flex gap-2">
              <Input
                id="copilot-proxy-key"
                type={showApiKey ? "text" : "password"}
                value={draft.apiKey}
                onChange={(event) => update("apiKey", event.target.value)}
                className="font-mono text-xs"
              />
              <Button
                type="button"
                size="icon"
                variant="outline"
                onClick={() => setShowApiKey((current) => !current)}
                aria-label={showApiKey ? "隐藏 API Key" : "显示 API Key"}
              >
                {showApiKey ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              真实密钥只保存在 CC Switch 配置中；VS Code 配置里只写入随机本地网关令牌。
            </p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="copilot-proxy-context">Context Window</Label>
              <Input
                id="copilot-proxy-context"
                type="number"
                min={1}
                value={draft.contextWindow}
                onChange={(event) =>
                  update(
                    "contextWindow",
                    parsePositiveInteger(event.target.value, 262144),
                  )
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="copilot-proxy-output">Max Output Tokens</Label>
              <Input
                id="copilot-proxy-output"
                type="number"
                min={1}
                value={draft.maxOutputTokens}
                onChange={(event) =>
                  update(
                    "maxOutputTokens",
                    parsePositiveInteger(event.target.value, 32768),
                  )
                }
              />
            </div>
          </div>

          <div className="grid gap-3 rounded-lg border p-4 md:grid-cols-2">
            {(
              [
                ["enabled", "启用供应商"],
                ["toolCalling", "Tool Calling"],
                ["vision", "Vision"],
                ["thinking", "Thinking"],
                ["streaming", "Streaming"],
              ] as const
            ).map(([key, label]) => (
              <div key={key} className="flex items-center justify-between gap-3">
                <Label htmlFor={`copilot-proxy-${key}`}>{label}</Label>
                <Switch
                  id={`copilot-proxy-${key}`}
                  checked={draft[key]}
                  onCheckedChange={(checked) => update(key, checked)}
                />
              </div>
            ))}
          </div>

          <div className="space-y-2">
            <Label htmlFor="copilot-proxy-headers">
              额外 Request Headers（JSON）
            </Label>
            <Textarea
              id="copilot-proxy-headers"
              value={headersText}
              onChange={(event) => setHeadersText(event.target.value)}
              rows={6}
              className="font-mono text-xs"
              spellCheck={false}
            />
            <p className="text-xs text-muted-foreground">
              Authorization、Host、Content-Length 和传输相关请求头由代理管理，不能覆盖。
            </p>
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
          <Button type="button" onClick={() => void handleSave()} disabled={!canSave}>
            {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
            保存供应商
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
