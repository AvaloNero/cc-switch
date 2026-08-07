import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppVisibilitySettings } from "@/components/settings/AppVisibilitySettings";
import type { SettingsFormState } from "@/hooks/useSettings";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => (key === "apps.copilotByok" ? "VS Code Copilot" : key),
  }),
}));

const baseSettings = {
  language: "zh",
  showInTray: true,
  minimizeToTrayOnClose: true,
} as SettingsFormState;

describe("AppVisibilitySettings", () => {
  it("shows VS Code Copilot by default and lets the user hide it", () => {
    const onChange = vi.fn();
    render(
      <AppVisibilitySettings settings={baseSettings} onChange={onChange} />,
    );

    const copilot = screen.getByRole("button", {
      name: "VS Code Copilot",
    });
    expect(copilot).toBeEnabled();

    fireEvent.click(copilot);

    expect(onChange).toHaveBeenCalledWith({
      visibleApps: expect.objectContaining({ copilotByok: false }),
    });
  });

  it("lets the user show VS Code Copilot again", () => {
    const onChange = vi.fn();
    render(
      <AppVisibilitySettings
        settings={{
          ...baseSettings,
          visibleApps: {
            claude: true,
            "claude-desktop": true,
            codex: true,
            gemini: true,
            grokbuild: true,
            opencode: true,
            openclaw: true,
            hermes: true,
            copilotByok: false,
          },
        }}
        onChange={onChange}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "VS Code Copilot" }));

    expect(onChange).toHaveBeenCalledWith({
      visibleApps: expect.objectContaining({ copilotByok: true }),
    });
  });
});
