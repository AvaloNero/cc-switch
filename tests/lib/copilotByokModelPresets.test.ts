import { describe, expect, it } from "vitest";
import {
  getCopilotByokModelPreset,
  mergeCopilotByokModelOptions,
} from "@/lib/copilotByokModelPresets";

describe("Copilot BYOK model presets", () => {
  it("fills K3's documented capabilities and fixed request options", () => {
    const preset = getCopilotByokModelPreset(
      {
        providerName: "Kimi",
        url: "https://api.kimi.com/coding/v1",
        apiType: "chat-completions",
      },
      "k3",
    );

    expect(preset).toEqual(
      expect.objectContaining({
        toolCalling: true,
        vision: true,
        thinking: true,
        streaming: true,
        contextWindow: 1_000_000,
        supportsReasoningEffort: ["low", "high", "max"],
        reasoningEffortFormat: "chat-completions",
        modelOptions: { temperature: 1, top_p: 0.95 },
      }),
    );
  });

  it("does not invent metadata for an unknown model", () => {
    expect(
      getCopilotByokModelPreset(
        {
          providerName: "Custom",
          url: "https://api.example.com/v1",
          apiType: "chat-completions",
        },
        "my-model",
      ),
    ).toBeNull();
  });

  it("fills MiniMax M-series capabilities from the official model contract", () => {
    const context = {
      providerName: "MiniMax",
      url: "https://api.minimax.io/v1",
      apiType: "chat-completions" as const,
    };

    expect(getCopilotByokModelPreset(context, "MiniMax-M3")).toEqual(
      expect.objectContaining({
        toolCalling: true,
        vision: true,
        thinking: true,
        streaming: true,
        contextWindow: 1_000_000,
        maxOutputTokens: 524_288,
        modelOptions: { temperature: 1, top_p: 0.95 },
      }),
    );
    expect(
      getCopilotByokModelPreset(context, "MiniMax-M2.7-highspeed"),
    ).toEqual(
      expect.objectContaining({
        toolCalling: true,
        vision: false,
        thinking: true,
        streaming: true,
        contextWindow: 204_800,
        maxOutputTokens: 204_800,
        modelOptions: { temperature: 1, top_p: 0.9 },
      }),
    );
    expect(
      mergeCopilotByokModelOptions(
        context,
        "MiniMax-M3",
        { top_p: 0.7 },
        { temperature: 1, top_p: 0.95 },
      ),
    ).toEqual({ temperature: 1, top_p: 0.7 });
  });

  it("canonicalizes K3's fixed top_p constraint", () => {
    expect(
      mergeCopilotByokModelOptions(
        {
          providerName: "Kimi",
          url: "https://api.kimi.com/coding/v1",
          apiType: "chat-completions",
        },
        "k3",
        { top_p: 1, custom: true },
        { temperature: 1, top_p: 0.95 },
      ),
    ).toEqual({ temperature: 1, top_p: 0.95, custom: true });
  });
});
