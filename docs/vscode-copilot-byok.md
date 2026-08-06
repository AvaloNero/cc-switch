# VS Code Copilot BYOK — Switch Proxy Variant

Branch: `feature/copilot-byok-switch-proxy`

## Behavior

This branch implements the current-provider/switching design:

1. CC Switch writes one fixed VS Code Custom Endpoint model named `CC Switch Current`.
2. The user selects that model once in the VS Code Copilot model picker.
3. CC Switch runs a loopback-only Chat Completions proxy.
4. Selecting a different provider in CC Switch changes the upstream endpoint and model behind the fixed VS Code model.
5. VS Code does not need to change its selected model after a provider switch.

The fixed model ID is:

```text
cc-switch-current
```

The default local endpoint is:

```text
http://127.0.0.1:15735/copilot/v1/chat/completions
```

## Provider contract

Each proxy provider contains:

- a full OpenAI-compatible Chat Completions endpoint,
- an upstream API key,
- an upstream model ID,
- optional request headers,
- context/output capability metadata,
- tool-calling, vision, thinking, and streaming capability flags.

The proxy rewrites the incoming `model` field to the current upstream model and streams the upstream response back to VS Code.

This variant intentionally does not attempt automatic Messages/Responses protocol conversion. The configured upstream must accept OpenAI Chat Completions requests.

## Credential model

VS Code receives only a random local gateway token in `chatLanguageModels.json`. The real upstream API key remains in the CC Switch application configuration directory in:

```text
copilot-byok-proxy.json
```

The store file is restricted to mode `0600` on Unix platforms. The local server only binds to `127.0.0.1`, and requests without the gateway Bearer token are rejected.

## VS Code targets and recovery

Stable/Insiders default profiles and named profiles are detected automatically. Custom absolute `chatLanguageModels.json` paths are supported.

CC Switch only replaces/removes the provider group named:

```text
CC Switch Proxy
```

Other BYOK groups are preserved. Before first modification of an existing file, CC Switch creates:

```text
chatLanguageModels.json.cc-switch.bak
```

Restoring a target removes it from active proxy management so that the restored file is not immediately overwritten again.

## Scope

This only affects the fixed Copilot Custom Endpoint chat model. It does not affect:

- inline completions,
- Next Edit Suggestions,
- embeddings,
- GitHub Copilot account authentication,
- existing CC Switch proxy takeover for Claude/Codex/Gemini.

The proxy is standalone and uses its own loopback port and lifecycle.

## Main review files

- `src-tauri/src/copilot_byok.rs`
- `src-tauri/src/copilot_byok/proxy.rs`
- `src-tauri/src/copilot_byok/vscode.rs`
- `src-tauri/src/commands/copilot_byok.rs`
- `src/lib/api/copilotByok.ts`
- `src/components/settings/CopilotByokSettings.tsx`
- `src/components/settings/CopilotByokModelDialog.tsx`
- `src/components/AppSwitcher.tsx`

## Suggested local review

```bash
pnpm install --frozen-lockfile
pnpm typecheck
pnpm format:check
pnpm test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml copilot_byok --lib
```

Manually verify gateway-token rejection, fixed-model installation, provider hot switching, streaming/tool calls, provider disable/delete behavior, occupied-port behavior, backup restoration, preservation of user-owned BYOK groups, and app restart with integration enabled.
