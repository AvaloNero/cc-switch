# GitHub Copilot CLI

## First-class application

CC Switch exposes GitHub Copilot CLI as an application independent from VS Code Copilot. It has its own app-switcher entry, visibility setting, official GitHub Copilot glyph (`githubcopilot`), provider catalog, active provider, sessions, and usage filter. Editing or deleting a provider in one Copilot application does not modify the other application's catalog.

The portable provider catalog uses the `copilot-cli-catalog` provider namespace and participates in the normal database export, WebDAV, and S3 synchronization flows. Each provider defines exactly one default model. This constraint applies only to Copilot CLI; VS Code Copilot continues to use the separate `copilot-byok-catalog` namespace and supports multiple models under one provider. The active provider/default model and the original environment snapshot are device state in `copilot-byok.json`, because environment restoration is specific to the machine.

## Provider switching

CC Switch manages the provider variables supported by Copilot CLI:

- `COPILOT_PROVIDER_TYPE` (`openai`, `azure`, or `anthropic`)
- `COPILOT_PROVIDER_BASE_URL`
- `COPILOT_PROVIDER_API_KEY` or `COPILOT_PROVIDER_BEARER_TOKEN`
- `COPILOT_PROVIDER_WIRE_API` (`completions` or `responses`)
- `COPILOT_PROVIDER_TRANSPORT` (`http` or `websockets`)
- `COPILOT_PROVIDER_AZURE_API_VERSION`
- `COPILOT_PROVIDER_HEADERS`
- `COPILOT_MODEL`, `COPILOT_PROVIDER_MODEL_ID`, and `COPILOT_PROVIDER_WIRE_MODEL`
- `COPILOT_PROVIDER_MAX_PROMPT_TOKENS` and `COPILOT_PROVIDER_MAX_OUTPUT_TOKENS`

The provider editor follows the provider-switching model used by the CLI applications in CC Switch: a Copilot CLI provider has one default model, and activating the provider applies that model directly. There is no independent model selector and no VS Code-style multi-model list. The backend rejects CLI provider records containing zero or multiple models. Existing multi-model CLI records are migrated once by retaining the active model for the active provider and the first enabled model for every other provider.

The editor also validates protocol combinations before saving. Anthropic uses the Messages API. WebSocket transport is only accepted with the Responses API. Azure endpoints are reduced to their resource host, while terminal paths such as `/responses`, `/chat/completions`, and `/v1/messages` are removed for other provider types.

Activating a provider writes its connection and default model to the user environment, so typing `copilot` in a newly opened terminal uses it directly; no wrapper executable or alias is required. Existing processes keep their inherited environment and must be restarted.

### Windows

Values are written to `HKCU\Environment`, followed by the Windows environment-change broadcast. A newly opened CMD or PowerShell session inherits them; Windows Terminal may need to be fully closed if it reuses a server process.

### macOS and Linux

Values are stored in private files:

```text
~/.cc-switch/copilot-cli-env.sh
~/.cc-switch/copilot-cli-env.fish
```

CC Switch adds a bounded, reversible source block to `.profile`, `.bashrc`, and `.zshrc`, as well as existing `.bash_profile`, `.bash_login`, or `.zprofile` files. Fish uses `~/.config/fish/conf.d/cc-switch-copilot.fish`. The managed environment files are written with user-only permissions on Unix.

On every platform, the first apply snapshots the original managed variables. **Restore Original Environment** restores that snapshot. Before switching or restoring, CC Switch compares live values and managed shell artifacts with its last successful write; external edits cause a conflict instead of being overwritten. Environment, state, and shell-hook updates are rollback-protected.

VS Code SecretStorage references such as `${input:provider-key}` cannot be resolved by Copilot CLI and are rejected. Provider credentials are stored in the CC Switch database and may be included in configured backups or cloud sync. Applied credentials are also present in the user environment or private shell files and are inherited by newly started processes.

## Native resources

The first-class toolbar manages Copilot CLI's official user resources, honoring `COPILOT_HOME` when it is set and otherwise using `~/.copilot`:

- Custom instructions: `copilot-instructions.md`
- Skills: `skills/`
- MCP: `mcp-config.json`, under the top-level `mcpServers` object
- Sessions: `session-state/<session-id>/events.jsonl`

GitHub defines the same user-level `~/.copilot/skills` directory for VS Code Copilot and Copilot CLI when `COPILOT_HOME` is not customized. CC Switch keeps the two application enablement flags independent, but materializes a Skill in that shared directory while either flag is enabled, so synchronizing or disabling one application cannot delete a Skill still selected for the other. GitHub's shared user directory cannot provide per-client physical isolation; set a distinct `COPILOT_HOME` if the CLI installation itself is intentionally separated.

MCP updates preserve unrelated top-level fields and use atomic private writes because server definitions can contain credentials. Session cards load messages from the official JSONL stream, delete only validated session directories, and resume with `copilot --resume=<session-id>`.

Copilot CLI does not support VS Code-style Prompt Files. The shared CC Switch editor is therefore labeled **Custom Instructions** for this application and composes the selected content into `copilot-instructions.md`.

## Usage statistics

Usage is reconstructed from the latest cumulative `session.shutdown.modelMetrics` snapshot in each session. Input, output, cache-read, and cache-write tokens are upserted under stable session/model request IDs, so resuming or rescanning a session replaces the previous totals instead of double-counting them. Active sessions without a shutdown snapshot are skipped until a complete cumulative snapshot exists.

Copilot CLI's event identifies the model but does not record whether that model ran through the GitHub subscription or a BYOK provider. Imported CLI session rows are therefore deliberately unpriced; assigning catalog pricing by model name alone would fabricate historical costs.

## Upstream references

- [Using BYOK models in GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/use-byok-models)
- [Adding MCP servers](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers)
- [Adding custom instructions](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)
- [Adding skills](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)

## Main review files

- `src-tauri/src/copilot_byok.rs`
- `src-tauri/src/copilot_byok/cli.rs`
- `src-tauri/src/session_manager/providers/copilot_cli.rs`
- `src-tauri/src/services/session_usage_copilot_cli.rs`
- `src-tauri/src/mcp/copilot_cli.rs`
- `src/components/settings/CopilotCliSettings.tsx`
- `src/components/settings/CopilotByokGroupPanel.tsx`
- `src/components/AppSwitcher.tsx`

## Suggested local review

```bash
pnpm typecheck
pnpm test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml copilot_cli --lib
```

Also verify provider switching in a newly opened CMD/PowerShell, Bash/Zsh, and Fish process; original-environment restoration; external-edit conflict handling; MCP field preservation; custom-instructions and Skills paths; session resume/delete; and idempotent usage re-import.
