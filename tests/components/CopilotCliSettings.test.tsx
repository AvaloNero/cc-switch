import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CopilotCliSettings } from "@/components/settings/CopilotCliSettings";
import type { CopilotByokGroup, CopilotByokState } from "@/lib/api";

const mocks = vi.hoisted(() => ({
  cliGetState: vi.fn(),
  setSelection: vi.fn(),
  disable: vi.fn(),
  upsertGroup: vi.fn(),
  deleteGroup: vi.fn(),
  reorderGroups: vi.fn(),
  checkConnection: vi.fn(),
  vscodeGetState: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  copilotCliApi: {
    getState: mocks.cliGetState,
    setSelection: mocks.setSelection,
    disable: mocks.disable,
    upsertGroup: mocks.upsertGroup,
    deleteGroup: mocks.deleteGroup,
    reorderGroups: mocks.reorderGroups,
    checkConnection: mocks.checkConnection,
  },
  copilotByokApi: {
    getState: mocks.vscodeGetState,
  },
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "apps.copilotCli": "Copilot CLI",
        "copilotByok.cli.provider": "供应商",
        "copilotByok.cli.model": "模型",
        "copilotByok.cli.defaultModel": "默认模型",
        "copilotByok.cli.apply": "应用到 Copilot CLI",
        "copilotByok.cli.disable": "恢复原环境",
        "copilotByok.cli.active": "已生效",
        "copilotByok.cli.inactive": "未配置",
        "copilotByok.cli.needsApply": "需要重新应用",
        "copilotByok.cli.catalogDescription": "CLI 独立供应商目录",
        "provider.tabProvider": "供应商",
        "provider.inUse": "使用中",
        "provider.enable": "启用",
        "provider.removeFromConfig": "移除",
        "provider.addToConfig": "添加",
        "provider.dragHandle": "拖拽排序",
        "provider.duplicate": "复制",
        "provider.connectivityCheck": "检测连通",
        "provider.configureUsage": "配置用量查询",
        "common.refresh": "刷新",
        "common.edit": "编辑",
        "common.delete": "删除",
        "common.copy": "复制",
      })[key] ?? key,
    i18n: { resolvedLanguage: "zh" },
  }),
}));

vi.mock("@/components/settings/CopilotByokGroupPanel", () => ({
  CopilotByokGroupPanel: () => null,
}));

const group: CopilotByokGroup = {
  id: "cli-provider",
  name: "CLI Provider",
  url: "https://api.example.com/v1/responses",
  apiKey: "secret",
  apiType: "responses",
  enabled: true,
  requestHeaders: {},
  extra: {},
  models: [
    {
      id: "cli-model",
      modelId: "gpt-custom",
      name: "GPT Custom",
      enabled: true,
      toolCalling: true,
      vision: false,
      thinking: true,
      streaming: true,
      contextWindow: 128_000,
      maxInputTokens: null,
      maxOutputTokens: 8_192,
      editTools: [],
      zeroDataRetentionEnabled: false,
      supportsReasoningEffort: [],
      reasoningEffortFormat: null,
      modelOptions: {},
      extra: {},
    },
  ],
};

function cliState(active: boolean): CopilotByokState {
  return {
    groups: [group],
    targets: [],
    selectedTargetIds: [],
    managedModelCount: 1,
    cli: {
      supported: true,
      enabled: active,
      selectedGroupId: active ? group.id : null,
      selectedModelId: active ? group.models[0].id : null,
      selectedProviderName: active ? group.name : null,
      selectedModelName: active ? group.models[0].name : null,
      environmentMatches: active,
      environmentConflicts: [],
    },
  };
}

describe("CopilotCliSettings", () => {
  beforeEach(() => {
    mocks.cliGetState.mockReset();
    mocks.setSelection.mockReset();
    mocks.disable.mockReset();
    mocks.upsertGroup.mockReset();
    mocks.deleteGroup.mockReset();
    mocks.reorderGroups.mockReset();
    mocks.checkConnection.mockReset();
    mocks.vscodeGetState.mockReset();

    mocks.cliGetState.mockResolvedValue(cliState(false));
    mocks.setSelection.mockResolvedValue(cliState(true));
    mocks.disable.mockResolvedValue(cliState(false));
    mocks.upsertGroup.mockResolvedValue(cliState(false));
    mocks.deleteGroup.mockResolvedValue(cliState(false));
    mocks.reorderGroups.mockResolvedValue(cliState(false));
  });

  it("switches a provider directly to its single default model", async () => {
    render(<CopilotCliSettings />);

    expect(await screen.findByText("CLI Provider")).toBeInTheDocument();
    expect(screen.getByText(/默认模型:/)).toHaveTextContent(
      "GPT Custom · gpt-custom",
    );
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "启用" }));

    await waitFor(() =>
      expect(mocks.setSelection).toHaveBeenCalledWith("cli-provider"),
    );
    expect(mocks.cliGetState).toHaveBeenCalled();
    expect(mocks.vscodeGetState).not.toHaveBeenCalled();
  });

  it("marks the active provider and protects it until the environment is restored", async () => {
    mocks.cliGetState.mockResolvedValue(cliState(true));
    render(<CopilotCliSettings />);

    expect(
      await screen.findByRole("button", { name: "使用中" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "删除" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "恢复原环境" }));
    await waitFor(() => expect(mocks.disable).toHaveBeenCalledTimes(1));
  });
});
