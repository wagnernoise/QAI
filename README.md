# QAI - QA Automation AI Agents

A collection of AI agents specialized in QA test automation. Built to assist QA automation engineers working with modern
testing frameworks.

## What Is This?

QAI is a **prompt-based AI agent** — it is not a standalone application or CLI tool. The agent lives entirely in the
file `qa-agent-system-prompt.md`, which you load as a system prompt into any LLM that supports tool-calling (
function-calling). The LLM then behaves as **QA-Bot**, an autonomous QA automation assistant.

No server, no build step, no runtime dependencies — just a system prompt and an LLM.

## QA-Bot

Autonomous AI agent that helps QA automation engineers with day-to-day test automation tasks.

### Features

- **Test creation** — e2e, component, and API tests from scratch or from requirements
- **Flaky test debugging** — root cause analysis, retry strategies, wait improvements
- **Test refactoring** — page objects, fixtures, data-driven patterns
- **Best practices** — locators (role/text/testid), assertions, auto-wait
- **Multi-language** — JS/TS, Python, Java, C#
- **CI/CD** — parallel execution, reporter configuration
- **BDD/Gherkin** — auto-detects BDD projects and prioritizes Gherkin standards with Cucumber, Playwright-BDD, Serenity,
  pytest-bdd

## Installation

### Prerequisites

- Access to an LLM with **tool-calling / function-calling** support. Compatible platforms include:
    - [OpenAI API](https://platform.openai.com/) (GPT-4o, GPT-4-turbo, etc.)
    - [Anthropic API](https://www.anthropic.com/) (Claude 3.5 Sonnet, Claude 4, etc.)
    - [xAI API](https://x.ai/) (Grok)
    - [Google Gemini API](https://ai.google.dev/)
    - Any local model with tool-calling support (e.g.,
      via [Ollama](https://ollama.com/), [LM Studio](https://lmstudio.ai/))
    - IDE-integrated AI assistants that accept custom system prompts (e.g., JetBrains AI, Cursor, Windsurf, Continue)

- A test project (or a new directory where you want to create one)

### Steps

1. **Clone this repository:**
   ```bash
   git clone https://github.com/your-username/QAI.git
   cd QAI
   ```

2. **Copy the system prompt** into your LLM platform of choice:
    - Open `qa-agent-system-prompt.md`
    - Copy its entire contents
    - Paste it as the **system prompt** (also called "system message" or "instructions") in your LLM client

   > **Tip:** Some platforms let you upload a file directly as the system prompt — use that if available.

3. **Configure tools** — QA-Bot expects the LLM to have access to these tool categories:
    - **File operations** — open, create, edit, search files in your project
    - **Terminal / bash** — run shell commands (`npm test`, `npx playwright test`, etc.)
    - **Web search** — look up documentation or current framework info

   For the exact tool definitions and policies, see `qa-agent-system-prompt.md`.

   How you provide these depends on your platform:
    - **API-based setups**: Define tools/functions in your API request (see your provider's function-calling docs)
    - **IDE assistants** (Cursor, JetBrains AI, Continue): Tools are usually built-in — just set the system prompt
    - **Chat UIs** (ChatGPT, Claude.ai): Limited tool access; the agent will still give guidance but can't execute
      commands directly

4. **Done.** Start chatting with the agent about your test automation tasks.

## Usage

### Basic workflow

1. Open a conversation with the LLM (with the system prompt loaded)
2. Describe your task — the agent will automatically pick the right mode and start working

**Example — create a test:**

```
Write a Playwright test that verifies the login page rejects invalid credentials
and shows an appropriate error message.
```

**Example — fix a flaky test:**

```
The test in tests/checkout.spec.ts fails intermittently with a timeout on the
payment confirmation step. Help me fix it.
```

**Example — BDD (auto-detected):**

```gherkin
Feature: Login validation
  Scenario: Invalid credentials show error
    Given I am on the login page
    When I enter invalid credentials
    And I submit the login form
    Then an error message is displayed
```

### Using the `<test_issue_description>` tag

For structured requests, wrap your task in a tag — this helps the agent parse complex multi-part requests:

```xml

<test_issue_description>
    Refactor the checkout tests to use page object model.
    Extract locators into a CheckoutPage class.
</test_issue_description>
```

### Setting up a new test project with the agent

You can ask QA-Bot to set up a project from scratch:

```
Set up a new Playwright project with TypeScript in this directory.
Include a basic config with Chrome and Firefox, and a sample test.
```

The agent will enter `[SETUP]` mode and run the necessary commands (`npm init`, `npx playwright install`, etc.).

### Supported Modes

The agent automatically selects the appropriate mode based on your request:

> **Note:** The authoritative mode list is defined in `qa-agent-system-prompt.md` and applies when that prompt is loaded.

| Mode              | Purpose                                                    |
|-------------------|------------------------------------------------------------|
| `[TEST_CODE]`     | Multi-step test writing, BDD scenarios, refactoring        |
| `[FAST_TEST]`     | Quick single-file test edits (1–3 steps)                   |
| `[RUN_VERIFY]`    | Run tests and collect evidence                             |
| `[SETUP]`         | Install/configure Playwright, Selenium, BDD frameworks     |
| `[CHAT]`          | Quick Q&A about testing                                    |
| `[ADVANCED_CHAT]` | In-depth test project analysis (read-only)                 |
| `[NICHE]`         | Trace analysis, locator forensics, flakiness investigation |

## Project Files

| File                        | Description                                                                        |
|-----------------------------|------------------------------------------------------------------------------------|
| `qa-agent-system-prompt.md` | The complete agent — system prompt with all modes, BDD detection, tool definitions |
| `README.md`                 | This file                                                                          |

## FAQ

**Q: Do I need to install anything to use QA-Bot?**
A: No. QA-Bot is a system prompt, not software. You only need access to an LLM. Your test project's own dependencies (
Playwright, Selenium, etc.) are separate — the agent can help you install those.

**Q: Which LLM works best?**
A: Any model with strong tool-calling support. GPT-4o, Claude 3.5 Sonnet / Claude 4, and Grok 4 all work well. Smaller
or local models may struggle with complex multi-step test tasks.

**Q: Can I use this without tool-calling (e.g., plain ChatGPT)?**
A: Partially. The agent will still provide test code, advice, and debugging help, but it won't be able to directly run
tests, create files, or interact with your project. You'll need to copy-paste commands and code manually.

**Q: Does it work with my existing test project?**
A: Yes. Point the agent at your project directory and describe what you need. It will read your existing tests, configs,
and page objects to understand the context.

**Q: How does BDD detection work?**
A: The agent scans your request and project files for BDD indicators (`.feature` files, Gherkin keywords like
`Given/When/Then`, framework names like Cucumber or playwright-bdd). If detected, it automatically prioritizes
Gherkin-style test generation.

## License

MIT
