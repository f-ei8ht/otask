# AI Coding Agent — Ratatui TUI

A beautiful terminal UI for an AI coding agent built with Rust + Ratatui + Crossterm.

## Features

- **Two modes**: Plan mode and Edit mode (switch with `p` / `b`)
- **Slash commands**: `/connect <provider> <api-key>` to link AI providers
- **Multi-provider support**: Cerebras, Anthropic, Anthropic, Codex/OpenAI
- **Dark theme**: Custom dark color palette using Ratatui RGB colors
- **Async API calls**: Non-blocking UI while waiting for AI responses

## Running

```bash
cargo run
```

## Keybindings

| Key     | Action                  |
|---------|-------------------------|
| `p`     | Switch to Plan mode     |
| `b`     | Switch to Edit mode     |
| `i`     | Start typing a message  |
| `/`     | Start a slash command   |
| `↑ ↓`  | Scroll message history  |
| `Esc`   | Stop typing             |
| `q`     | Quit                    |
| `Ctrl+C`| Force quit              |

## Slash Commands

```
/connect cerebras  <api-key>   Connect Cerebras (llama-3.3-70b)
/connect anthropic <api-key>   Connect Anthropic (claude-opus-4-5)
/connect codex     <api-key>   Connect Codex/OpenAI (gpt-4o)
/help                          Show all commands
```

## Structure

```
src/
  main.rs              Entry point, terminal setup
  app.rs               App state, event loop, mode switching
  ui.rs                Ratatui rendering (header, messages, input, statusbar)
  providers/
    mod.rs             Provider trait + shared types
    cerebras.rs        Cerebras API (OpenAI-compatible)
    anthropic.rs       Anthropic Messages API
    codex.rs           OpenAI Chat Completions API
```

## Dependencies

- `ratatui` 0.30 — TUI framework
- `crossterm` 0.28 — Cross-platform terminal backend
- `tokio` — Async runtime
- `reqwest` (rustls-tls) — HTTP client for API calls
- `serde` / `serde_json` — JSON serialization
- `anyhow` — Error handling
