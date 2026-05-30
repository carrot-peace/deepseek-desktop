# DeepSeek Desktop

macOS-first DeepSeek desktop chat client built with Tauri 2, React, TypeScript, Tailwind CSS, Zustand, SQLite, and Keychain-backed secrets.

## Current MVP

- Local conversations and messages in SQLite.
- DeepSeek API Key and Tavily API Key saved through the system credential store.
- Streaming chat events from Rust to React.
- Model switch: `deepseek-v4-pro` / `deepseek-v4-flash`.
- Thinking mode switch: off / high / max.
- Optional Tavily-backed `web_search` tool call flow.
- Markdown rendering, code highlighting, copy buttons, and stop generation.

## Run

```bash
npm install --cache .npm-cache
npm run tauri dev
```

If Cargo cannot write to the default user cache in this environment, run Tauri with a project-local cache:

```bash
CARGO_HOME="$PWD/.cargo-home" npm run tauri dev
```

## Notes

- API keys are never written to SQLite or localStorage.
- The first search implementation only allows the `web_search` tool.
- Request context is limited to the latest 20 conversation messages.

## Roadmap

- Deep Research mode for longer, source-backed research workflows.
