from pathlib import Path

path = Path("src/components/settings/SettingsPage.tsx")
text = path.read_text(encoding="utf-8")

import_anchor = (
    'import { CodexAuthSettings } from "@/components/settings/CodexAuthSettings";\n'
)
imports = (
    import_anchor
    + 'import { CopilotByokSettings } from "@/components/settings/CopilotByokSettings";\n'
    + 'import copilotByokIcon from "@/assets/icons/vscode-copilot-byok.png";\n'
)
if "CopilotByokSettings" not in text:
    if text.count(import_anchor) != 1:
        raise SystemExit("SettingsPage import anchor changed")
    text = text.replace(import_anchor, imports, 1)

accordion_anchor = """                    >
                      <AccordionItem
                        value="directory"
"""
copilot_item = """                    >
                      <AccordionItem
                        value="copilotByok"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <img
                              src={copilotByokIcon}
                              alt="VS Code Copilot"
                              className="h-10 w-10 rounded-lg border object-cover"
                            />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.copilotByok.title", {
                                  defaultValue: "VS Code Copilot BYOK",
                                })}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.copilotByok.description", {
                                  defaultValue:
                                    "Add compatible custom endpoint models to the VS Code Copilot model picker",
                                })}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <CopilotByokSettings />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="directory"
"""
if 'value="copilotByok"' not in text:
    if text.count(accordion_anchor) != 1:
        raise SystemExit("SettingsPage accordion anchor changed")
    text = text.replace(accordion_anchor, copilot_item, 1)

path.write_text(text, encoding="utf-8")
