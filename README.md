k
# QAI — QA Automation AI Agent

A Rust CLI + TUI tool that connects to LLMs (Ollama, OpenAI, Anthropic, xAI, Zen, or any custom endpoint) and runs an autonomous QA agent with a ReAct reasoning loop. Chat with models, use Agent Mode to let the AI read/write files and run commands, and manage the QA-Bot system prompt — all from a beautiful terminal interface.

---

## Installation

```bash
git clone https://github.com/your-org/QAI.git
cd QAI
cargo build --release
sudo cp ./target/release/qai-cli /usr/local/bin/qai-cli
```

Then launch the TUI:

```bash
qai-cli
```

---

## TUI Overview

The TUI has five screens, navigated from the main menu:

| Screen | Description |
|--------|-------------|
| **Info** | System info and current configuration |
| **Show** | Display the full QA-Bot system prompt |
| **Validate** | Validate the system prompt file |
| **Tools** | Browse available LLM providers with details |
| **Chat** | Interactive chat with any LLM provider |

### General Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` or `j` / `k` | Navigate menus |
| `Enter` | Select / confirm |
| `q` | Go back / quit |
| `Esc` | Go back to menu |

---

## Chat Screen

### Connecting to a Provider

1. Open the TUI and select **Chat**
2. Choose a provider from the list (`↑`/`↓`, then `Enter`)
3. For **Ollama**: models are fetched automatically from your local instance
4. For cloud providers: enter your API token (saved automatically to `~/.config/qai/config.toml`)
5. Select a model and start chatting

### Supported Providers

| Provider | Default Model | API Endpoint |
|----------|--------------|--------------|
| OpenAI | `gpt-4o` | `https://api.openai.com/v1/chat/completions` |
| Anthropic | `claude-opus-4-5` | `https://api.anthropic.com/v1/messages` |
| xAI | `grok-4` | `https://api.x.ai/v1/chat/completions` |
| Ollama | `llama3.2` | `http://localhost:11434/api/chat` |
| Zen | `anthropic/claude-sonnet-4-5` | `https://api.opencode.ai/v1/chat/completions` |
| Custom | *(user-defined)* | *(user-defined)* |

### Chat Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus: Provider → Model → Token → Message → Conversation |
| `Enter` | Send message |
| `Shift+Enter` or `Ctrl+J` | Insert newline in message box |
| `↑` / `↓` | Navigate lists or scroll conversation (when focused) |
| `PageUp` / `PageDown` | Scroll conversation 5 lines |
| `End` | Jump to bottom and resume auto-scroll |
| `Esc` (×1) | Show stop hint |
| `Esc` (×2, within 1s) | Cancel active inference / stop streaming |
| `Ctrl+C` (Linux/Win) / `Cmd+C` (macOS) | Copy selected conversation text |
| `F2` | Toggle Agent Mode on/off |

### Conversation Features

- **Auto-scroll**: conversation follows new tokens in real time as the model streams
- **Manual scroll**: `Tab` to focus the Conversation panel, then `↑`/`↓` or `PageUp`/`PageDown`; scrollbar visible on the right edge
- **Mouse scroll**: trackpad/mouse wheel scrolls the conversation
- **Click scrollbar**: click or drag the scrollbar to jump to any position
- **Text selection**: click and drag to select text; copy with `Ctrl+C` / `Cmd+C`
- **Thinking indicator**: a blinking `⏳ Thinking...` appears while the model is generating

### Message Box Features

- **Multi-line input**: `Shift+Enter` or `Ctrl+J` to add new lines
- **Cursor navigation**: `←`/`→`/`Home`/`End` move the cursor; `↑`/`↓` move between wrapped lines
- **Auto-scroll**: the message box scrolls to keep the cursor visible for long prompts
- **Scrollbar**: visible on the right edge when content overflows; click/drag to scroll

### API Token Persistence

API tokens are saved automatically to `~/.config/qai/config.toml` the moment you type them. A `✓ API token saved` confirmation appears in the status bar. On next launch, the token is loaded automatically — no need to re-enter it.

---

## Agent Mode

Press **F2** in the Chat screen to toggle Agent Mode. When enabled, your messages are routed through a **ReAct (Reason → Act → Observe)** loop instead of a plain chat completion.

### How It Works

```
User gives a task
Loop (up to 15 steps):
  a. Agent THINKS about what to do        → <think>...</think>
  b. Agent calls a TOOL                   → <tool name="...">input</tool>
  c. Tool executes and returns a RESULT   → 👁 Observation
  d. Agent observes and decides next step
Agent provides final answer               → <answer>...</answer>
```

Each step is streamed into the conversation panel so you can follow the agent's reasoning in real time.

### Built-in Tools

| Tool | Description | Input Format |
|------|-------------|--------------|
| `read_file` | Read a local file | File path |
| `write_file` | Create or overwrite a file | `path\ncontent` |
| `edit_file` | Search-and-replace in a file | `path\n<<<\nsearch\n===\nreplacement\n>>>` |
| `shell` | Run any shell command | Shell command string |
| `web_search` | Query DuckDuckGo instant answers | Search query |
| `git_status` | Show working tree status | *(empty)* |
| `git_diff` | Show unstaged changes | *(empty)* |
| `git_add` | Stage files | File path(s) |
| `git_commit` | Commit staged changes | Commit message |
| `git_log` | Show recent commits | Optional count (default: 10) |

### Conversation Memory

The agent retains the full conversation history across all turns in a session, giving the LLM context from previous exchanges when reasoning about new tasks.

### Example

```
You: Refactor README.md to improve clarity

QA-Bot:
  💭 I'll read the current README first.
  🔧 read_file → README.md
  👁 [file contents]
  💭 I'll rewrite the introduction section.
  🔧 write_file → README.md
  👁 File written successfully.
  ✅ Done. README.md has been updated.
```

---

## CLI Mode (Non-TUI)

Pass a subcommand to skip the TUI entirely — useful for scripting:

```bash
qai-cli info                                      # Show system info
qai-cli show                                      # Print the system prompt
qai-cli copy ./qa-agent-system-prompt.md          # Copy prompt to a file
qai-cli copy ./qa-agent-system-prompt.md --force  # Overwrite if exists
qai-cli validate                                  # Validate the prompt file
qai-cli tools                                     # List available tools
```

Use `--no-tui` to suppress the TUI when no subcommand is given:

```bash
qai-cli --no-tui
```

---

## QA-Bot System Prompt

`qa-agent-system-prompt.md` is the authoritative system prompt that defines QA-Bot's behavior, modes, and policies. Load it into any LLM that supports system prompts to use QA-Bot outside the TUI.

### QA-Bot Modes (defined in `qa-agent-system-prompt.md`)

| Mode | Purpose |
|------|---------|
| `[TEST_CODE]` | Multi-step test writing, BDD, refactoring |
| `[FAST_TEST]` | Quick single-file edits (1–3 steps) |
| `[RUN_VERIFY]` | Run tests, collect evidence |
| `[SETUP]` | Install/configure test frameworks |
| `[CHAT]` | Quick Q&A about testing |
| `[ADVANCED_CHAT]` | In-depth test project analysis |
| `[NICHE]` | Trace analysis, locator forensics |

---

## Project Structure

| Path | Description |
|------|-------------|
| `qa-agent-system-prompt.md` | QA-Bot system prompt (authoritative) |
| `src/main.rs` | CLI entry point |
| `src/lib.rs` | Public library API |
| `src/agent/` | ReAct agent loop and tool dispatcher |
| `src/tui/` | TUI screens, state, drawing, event handling |
| `tests/` | Integration and unit tests |

---

## Requirements

- **Rust** 1.75+ (for building)
- **LLM provider**: Ollama (local), OpenAI, Anthropic, xAI, Zen, or any OpenAI-compatible endpoint
- **Ollama** (optional): install from [ollama.com](https://ollama.com) for local model support

---

## FAQ

**Which model works best?**
For Agent Mode, use a capable model: `gpt-4o`, `claude-opus-4-5`, `grok-4`, or a large local model via Ollama (e.g. `llama3.1:70b`). Smaller models may not follow the `<tool>` tag format reliably.

**Does Agent Mode work with Ollama?**
Yes. Select Ollama as the provider, pick a model, enable Agent Mode with `F2`, and type your task.

**Where is my API token stored?**
In `~/.config/qai/config.toml`. It is never sent anywhere except the provider's API endpoint.

**Can I use QA-Bot without the TUI?**
Yes — use the CLI subcommands, or load `qa-agent-system-prompt.md` directly into any LLM chat interface.

---

## License

MIT