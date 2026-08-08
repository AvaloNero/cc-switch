import type {
  CopilotByokApiType,
  CopilotByokModel,
} from "@/lib/api/copilotByok";

export interface CopilotByokModelPresetContext {
  providerName: string;
  url: string;
  apiType: CopilotByokApiType;
}

export type CopilotByokModelPreset = Partial<
  Pick<
    CopilotByokModel,
    | "toolCalling"
    | "vision"
    | "thinking"
    | "streaming"
    | "contextWindow"
    | "maxInputTokens"
    | "maxOutputTokens"
    | "editTools"
    | "supportsReasoningEffort"
    | "reasoningEffortFormat"
    | "modelOptions"
  >
>;

function objectValue(value: unknown): Record<string, unknown> {
  return value && !Array.isArray(value) && typeof value === "object"
    ? (value as Record<string, unknown>)
    : {};
}

function endpointHostname(value: string): string {
  try {
    return new URL(value).hostname.toLowerCase();
  } catch {
    return "";
  }
}

function isKimiProvider(context: CopilotByokModelPresetContext): boolean {
  const providerName = context.providerName.trim().toLowerCase();
  const hostname = endpointHostname(context.url);
  return (
    providerName.includes("kimi") ||
    providerName.includes("moonshot") ||
    hostname === "api.kimi.com" ||
    hostname.endsWith(".api.kimi.com") ||
    hostname === "api.moonshot.cn" ||
    hostname.endsWith(".api.moonshot.cn") ||
    hostname === "api.moonshot.ai" ||
    hostname.endsWith(".api.moonshot.ai")
  );
}

function isKimiK3(modelId: string): boolean {
  const normalized = modelId.trim().toLowerCase();
  return (
    normalized === "k3" ||
    normalized === "kimi-k3" ||
    normalized.endsWith("/k3") ||
    normalized.endsWith(":k3")
  );
}

function isMiniMaxProvider(context: CopilotByokModelPresetContext): boolean {
  const providerName = context.providerName.trim().toLowerCase();
  const hostname = endpointHostname(context.url);
  return (
    providerName.includes("minimax") ||
    hostname === "api.minimax.io" ||
    hostname.endsWith(".api.minimax.io") ||
    hostname === "api.minimaxi.com" ||
    hostname.endsWith(".api.minimaxi.com")
  );
}

function miniMaxModelFamily(modelId: string): "m3" | "m2" | null {
  const normalized =
    modelId.trim().toLowerCase().split(/[/:]/).filter(Boolean).at(-1) ?? "";
  if (normalized === "minimax-m3") return "m3";
  if (
    [
      "minimax-m2",
      "minimax-m2.1",
      "minimax-m2.1-highspeed",
      "minimax-m2.5",
      "minimax-m2.5-highspeed",
      "minimax-m2.7",
      "minimax-m2.7-highspeed",
    ].includes(normalized)
  ) {
    return "m2";
  }
  return null;
}

/**
 * Return only documented, model-specific defaults. Unknown models deliberately
 * remain in VS Code's automatic/unspecified state instead of receiving guessed
 * capabilities that could make requests fail.
 */
export function getCopilotByokModelPreset(
  context: CopilotByokModelPresetContext,
  modelId: string,
): CopilotByokModelPreset | null {
  if (isKimiProvider(context) && isKimiK3(modelId)) {
    return {
      toolCalling: true,
      vision: true,
      thinking: true,
      streaming: true,
      contextWindow: 1_000_000,
      supportsReasoningEffort: ["low", "high", "max"],
      reasoningEffortFormat: context.apiType,
      // VS Code Custom Endpoint currently applies only temperature and top_p
      // from modelOptions. K3 fixes these values at 1 and 0.95 respectively.
      modelOptions: { temperature: 1, top_p: 0.95 },
    };
  }

  const miniMaxFamily = isMiniMaxProvider(context)
    ? miniMaxModelFamily(modelId)
    : null;
  if (miniMaxFamily === "m3") {
    return {
      toolCalling: true,
      vision: true,
      thinking: true,
      streaming: true,
      contextWindow: 1_000_000,
      maxOutputTokens: 524_288,
      modelOptions: { temperature: 1, top_p: 0.95 },
    };
  }
  if (miniMaxFamily === "m2") {
    return {
      toolCalling: true,
      vision: false,
      thinking: true,
      streaming: true,
      contextWindow: 204_800,
      maxOutputTokens: 204_800,
      modelOptions: { temperature: 1, top_p: 0.9 },
    };
  }

  return null;
}

export function mergeCopilotByokModelOptions(
  context: CopilotByokModelPresetContext,
  modelId: string,
  current: unknown,
  preset: unknown,
): Record<string, unknown> {
  const currentOptions = objectValue(current);
  const presetOptions = objectValue(preset);
  // K3's sampling values are fixed server constraints rather than defaults.
  // Other known providers keep any explicit user override.
  return isKimiProvider(context) && isKimiK3(modelId)
    ? { ...currentOptions, ...presetOptions }
    : { ...presetOptions, ...currentOptions };
}
