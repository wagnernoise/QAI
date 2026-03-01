pub mod tools;
pub mod pr_review;

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

use crate::tui::providers::Provider;

// ── Constants ─────────────────────────────────────────────────────────────────


// ── ReAct step types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    Thought,
    ToolCall { name: String, input: String },
    Observation(String),
    Answer(String),
}

// ── Tag parsing ───────────────────────────────────────────────────────────────

/// Parse all recognised tags from an LLM response, in order of appearance.
/// Returns a Vec of StepKind found in the text.
pub fn parse_steps(text: &str) -> Vec<StepKind> {
    let mut steps = Vec::new();
    let mut remaining = text;

    loop {
        // Find the earliest tag among think/tool/answer
        let think_pos = find_tag_start(remaining, "think");
        let tool_pos = find_tag_start(remaining, "tool");
        let answer_pos = find_tag_start(remaining, "answer");

        // Pick the earliest one
        let earliest = [
            think_pos.map(|p| (p, "think")),
            tool_pos.map(|p| (p, "tool")),
            answer_pos.map(|p| (p, "answer")),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(pos, _)| *pos);

        match earliest {
            None => break,
            Some((_, "think")) => {
                if let Some(inner) = extract_tag(remaining, "think") {
                    steps.push(StepKind::Thought);
                    // advance past this tag
                    let close = "</think>";
                    if let Some(end) = remaining.find(close) {
                        remaining = &remaining[end + close.len()..];
                    } else {
                        break;
                    }
                    // Also emit the thought content as a sub-step so callers can display it
                    // We store it in a special way: re-use Thought but carry text via a separate emit
                    let _ = inner; // content extracted separately in run()
                } else {
                    break;
                }
            }
            Some((_, "tool")) => {
                if let Some(inner) = extract_tag(remaining, "tool") {
                    let name_attr = extract_attr_in(remaining, "tool", "name");
                    let step = if let Some(name) = name_attr {
                        StepKind::ToolCall { name, input: inner.trim().to_string() }
                    } else {
                        let mut lines = inner.trim().splitn(2, '\n');
                        let name = lines.next().unwrap_or("").trim().to_string();
                        let input = lines.next().unwrap_or("").trim().to_string();
                        if name.is_empty() {
                            break;
                        }
                        StepKind::ToolCall { name, input }
                    };
                    steps.push(step);
                    let close = "</tool>";
                    if let Some(end) = remaining.find(close) {
                        remaining = &remaining[end + close.len()..];
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            Some((_, "answer")) => {
                if let Some(inner) = extract_tag(remaining, "answer") {
                    steps.push(StepKind::Answer(inner.trim().to_string()));
                }
                break; // answer always terminates
            }
            _ => break,
        }
    }

    steps
}

/// Backwards-compatible single-step parser (used by tests).
pub fn parse_step(text: &str) -> Option<StepKind> {
    parse_steps(text).into_iter().next()
}

/// Attempt to recover a tool call from a plain-text LLM response that ignored the XML format.
/// Handles patterns like:
///   read_file\npath
///   shell\nls -la
///   tool_name\nparam=value\n...
///   [TOOL_CALL]{"tool": "name", "args": "..."}[/TOOL_CALL]
///   [TOOL_CALL]{"name": "name", "input": "..."}[/TOOL_CALL]
///   [TOOL_CALL]{"tool": "name", "parameters": {...}}[/TOOL_CALL]
pub fn try_recover_plain_tool(text: &str) -> Option<StepKind> {
    const KNOWN_TOOLS: &[&str] = &[
        "read_file", "write_file", "edit_file", "shell", "grep_search", "web_search",
        "git_status", "git_diff", "git_add", "git_commit", "git_log", "answer",
    ];

    let trimmed = text.trim();

    // Pattern 0: [TOOL_CALL]{...}[/TOOL_CALL] or [TOOL_CALL]{...} (unclosed)
    // Also handles variations: [[TOOL_CALL]], <TOOL_CALL>, etc.
    if let Some(step) = try_recover_bracketed_tool_call(trimmed) {
        return Some(step);
    }

    // Pattern 1: first non-empty line is a known tool name
    let mut lines = trimmed.lines();
    let first_line = lines.next()?.trim();
    // Strip markdown backticks if present
    let tool_name = first_line.trim_matches('`').trim();
    if KNOWN_TOOLS.contains(&tool_name) {
        let input = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        return Some(StepKind::ToolCall {
            name: tool_name.to_string(),
            input,
        });
    }

    // Pattern 2: line contains "tool_name:" or "tool_name :" prefix
    for line in trimmed.lines() {
        let l = line.trim();
        for &tool in KNOWN_TOOLS {
            let prefix_colon = format!("{tool}:");
            let prefix_space = format!("{tool} :");
            if l.starts_with(&prefix_colon) || l.starts_with(&prefix_space) {
                let input = l
                    .trim_start_matches(tool)
                    .trim_start_matches(':')
                    .trim()
                    .to_string();
                return Some(StepKind::ToolCall {
                    name: tool.to_string(),
                    input,
                });
            }
        }
    }

    None
}

/// Extract a tool call from bracket-style formats emitted by some models:
///   [TOOL_CALL]{...}[/TOOL_CALL]
///   [[TOOL_CALL]]{...}[[/TOOL_CALL]]
///   <TOOL_CALL>{...}</TOOL_CALL>
///   [TOOL_CALL]{...}   (unclosed)
///
/// The JSON object may use any of these field names:
///   tool name  : "tool", "name", "function", "tool_name", "function_name"
///   tool input : "args", "input", "arguments", "parameters", "content"
fn try_recover_bracketed_tool_call(text: &str) -> Option<StepKind> {
    // Locate the JSON object — find the first '{' after an opening marker
    // We support several opening markers (case-insensitive search via to_uppercase)
    let upper = text.to_uppercase();
    let markers = ["[TOOL_CALL]", "[[TOOL_CALL]]", "<TOOL_CALL>", "[TOOL_USE]", "[[TOOL_USE]]"];
    let json_start = markers.iter()
        .filter_map(|m| upper.find(m).map(|pos| pos + m.len()))
        .min()?;

    // Find the first '{' at or after json_start
    let brace_start = text[json_start..].find('{').map(|p| json_start + p)?;

    // Find the matching closing '}' (handle nesting)
    let json_str = extract_balanced_braces(&text[brace_start..])?;

    // Parse the JSON
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;

    // Extract tool name from common field names
    let name = ["tool", "name", "function", "tool_name", "function_name"]
        .iter()
        .find_map(|&k| v.get(k).and_then(|n| n.as_str()).map(|s| s.to_string()))?;

    // Extract tool input from common field names
    // "args" / "arguments" / "parameters" may be a string or an object
    let input = ["args", "input", "arguments", "parameters", "content"]
        .iter()
        .find_map(|&k| {
            v.get(k).map(|val| match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
        })
        .unwrap_or_default();

    Some(StepKind::ToolCall { name, input })
}

fn looks_like_tool_call_scaffold(text: &str) -> bool {
    let upper = text.to_uppercase();
    upper.contains("[TOOL_CALL")
        || upper.contains("[[TOOL_CALL")
        || upper.contains("<TOOL_CALL")
        || upper.contains("[TOOL_USE")
        || upper.contains("[[TOOL_USE")
}

/// Walk `text` starting at the first `{` and return the slice that forms a
/// balanced JSON object (including the surrounding braces).  Returns `None`
/// if the braces are never balanced.
fn extract_balanced_braces(text: &str) -> Option<&str> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    let mut end = 0usize;

    for (i, ch) in text.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if in_string {
            match ch {
                '\\' => escape_next = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > 0 {
        Some(&text[..=end])
    } else {
        None
    }
}

fn find_tag_start(text: &str, tag: &str) -> Option<usize> {
    text.find(&format!("<{tag}"))
}

pub fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = text.find(&open)?;
    let tag_end = text[start..].find('>')? + start + 1;
    let end = text.find(&close)?;
    if end < tag_end {
        return None;
    }
    Some(text[tag_end..end].to_string())
}

fn extract_attr(text: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = text.find(&needle)? + needle.len();
    let end = text[start..].find('"')? + start;
    Some(text[start..end].to_string())
}

/// Extract an attribute from the opening tag of a specific element.
fn extract_attr_in(text: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{tag}");
    let tag_start = text.find(&open)?;
    let tag_end = text[tag_start..].find('>')? + tag_start;
    let tag_text = &text[tag_start..tag_end];
    extract_attr(tag_text, attr)
}

// ── ReActAgent ────────────────────────────────────────────────────────────────

pub struct ReActAgent {
    pub provider: Provider,
    pub api_token: String,
    pub custom_url: String,
    pub model: String,
    pub system_prompt: String,
}

impl ReActAgent {
    pub fn new(
        provider: Provider,
        api_token: String,
        custom_url: String,
        model: String,
        system_prompt: String,
    ) -> Self {
        Self {
            provider,
            api_token,
            custom_url,
            model,
            system_prompt,
        }
    }

    /// Run the ReAct loop for the given task.
    /// `prior_history` contains all previous conversation turns (role, content)
    /// so the agent has memory of the full session.
    /// Each step (Thought / ToolCall / Observation / Answer) is sent as a
    /// `Some(String)` token through `tx`; `None` signals completion.
    pub async fn run(
        &self,
        task: String,
        prior_history: Vec<(String, String)>,
        tx: mpsc::UnboundedSender<Option<String>>,
    ) -> Result<()> {
        let react_system = format!(
            "{}\n\n\
            You are operating in ReAct mode. You MUST use XML tags for EVERY response. No plain text outside tags.\n\n\
            RESPONSE FORMAT — you must use EXACTLY these XML tags:\n\
              <think>your reasoning here</think>\n\
              <tool name=\"TOOL_NAME\">tool input here</tool>\n\
              <answer>final answer to the user</answer>\n\n\
            Available tools:\n\
              read_file    — read a file. Input: file path\n\
              write_file   — create/overwrite a file. Input: path on first line, then full content\n\
              edit_file    — search-and-replace in a file. Input: path\\n<<<\\nsearch\\n===\\nreplacement\\n>>>\n\
              shell        — run a shell command. Input: the command string\n\
              grep_search  — search file contents by regex. Input: pattern on first line, path on second (optional, default .), file glob on third (optional, e.g. *.rs)\n\
              web_search   — search the web. Input: search query\n\
              git_status   — show git status. Input: (empty)\n\
              git_diff     — show git diff. Input: (empty or path)\n\
              git_add      — stage files. Input: path or .\n\
              git_commit   — commit staged files. Input: commit message\n\
              git_log      — show git log. Input: (empty)\n\n\
            CONCRETE EXAMPLES — copy this exact format:\n\n\
            Example 1 (read a file):\n\
            <think>I need to read README.md to understand its current content.</think>\n\
            <tool name=\"read_file\">README.md</tool>\n\n\
            Example 2 (run a shell command):\n\
            <think>I will list the project files to understand the structure.</think>\n\
            <tool name=\"shell\">ls -la</tool>\n\n\
            Example 3 (write a file after reading it):\n\
            <think>I have read the file. Now I will write the improved version.</think>\n\
            <tool name=\"write_file\">README.md\n# New Title\nImproved content here.\n</tool>\n\n\
            Example 4 (final answer):\n\
            <think>The task is complete.</think>\n\
            <answer>I have refactored README.md. The changes improve clarity by...</answer>\n\n\
            STRICT RULES:\n\
              1. EVERY response MUST start with <think> and end with either <tool> or <answer>.\n\
              2. NEVER output plain text, function names, or parameters outside XML tags.\n\
              3. NEVER write tool names as plain text (e.g. do NOT write `read_file` or `shell` outside a <tool> tag).\n\
              4. After receiving an <observation>, continue with <think> then <tool> or <answer>.\n\
              5. Only output <answer> when the task is fully complete.",
            self.system_prompt
        );

        // Seed history with prior turns, deduplicating the current task if already present
        let mut history: Vec<(String, String)> = prior_history
            .into_iter()
            .filter(|(role, content)| !(role == "user" && content == &task))
            .collect();
        history.push(("user".to_string(), task.clone()));

        let mut step = 0usize;
        loop {
            step += 1;
            let _ = tx.send(Some(format!("\n---\n🔄 **Step {}**\n", step)));

            let llm_response = self
                .call_llm(&react_system, &history)
                .await
                .unwrap_or_else(|e| format!("<answer>[LLM error: {e}]</answer>"));

            // Try XML tag parsing first; fall back to plain-text tool detection
            let mut steps = parse_steps(&llm_response);

            if steps.is_empty() {
                // Attempt to recover a plain-text tool call emitted by the LLM
                // e.g. the model writes:  read_file\nREADME.md  instead of <tool name="read_file">README.md</tool>
                if let Some(recovered) = try_recover_plain_tool(&llm_response) {
                    let _ = tx.send(Some("⚠️ *Model ignored XML format — auto-recovering tool call.*\n".to_string()));
                    steps.push(recovered);
                } else if looks_like_tool_call_scaffold(&llm_response) {
                    // Avoid leaking malformed tool-call scaffolding to the user.
                    let _ = tx.send(Some(
                        "⚠️ *Model emitted malformed tool-call syntax — retrying with strict XML format.*\n"
                            .to_string(),
                    ));
                    history.push((
                        "user".to_string(),
                        "Your last response used malformed tool-call scaffolding (e.g. [TOOL_CALL]). \
                         Retry now using valid XML only: start with <think> and end with either \
                         <tool name=\"...\">...</tool> or <answer>...</answer>.".to_string(),
                    ));
                    continue;
                } else {
                    // No tags and no recoverable tool call — show response and finish
                    let _ = tx.send(Some(llm_response.clone()));
                    history.push(("assistant".to_string(), llm_response));
                    let _ = tx.send(None);
                    return Ok(());
                }
            }

            history.push(("assistant".to_string(), llm_response.clone()));

            // Check if this response is think-only (no tool call or answer).
            let has_action = steps.iter().any(|s| {
                matches!(s, StepKind::ToolCall { .. } | StepKind::Answer(_))
            });
            if !has_action {
                // Display the thought so the user can see reasoning
                if let Some(inner) = extract_tag(&llm_response, "think") {
                    let _ = tx.send(Some(format!("💭 **Thought:** {}\n\n", inner.trim())));
                } else {
                    let _ = tx.send(Some(format!("💭 {}\n\n", llm_response.trim())));
                }
                // Forceful nudge with a concrete example
                history.push((
                    "user".to_string(),
                    "STOP. You must now output a tool call or answer using XML tags. \
                     Example of correct format:\n\
                     <think>I will read the file.</think>\n\
                     <tool name=\"read_file\">README.md</tool>\n\
                     Do NOT write plain text. Use the XML tags exactly as shown.".to_string(),
                ));
                continue;
            }

            let mut finished = false;
            let mut remaining_resp = llm_response.as_str();
            for parsed_step in steps {
                match parsed_step {
                    StepKind::Thought => {
                        // Extract the next think block from remaining text
                        if let Some(inner) = extract_tag(remaining_resp, "think") {
                            let _ = tx.send(Some(format!("💭 **Thought:** {}\n\n", inner.trim())));
                            // Advance past this think block
                            if let Some(end) = remaining_resp.find("</think>") {
                                remaining_resp = &remaining_resp[end + "</think>".len()..];
                            }
                        }
                    }
                    StepKind::ToolCall { name, input } => {
                        // Don't display "answer" as a tool call — it's a final answer in disguise
                        if name != "answer" {
                            let _ = tx.send(Some(format!("🔧 **Tool `{name}`:** `{}`\n", truncate(&input, 120))));
                        }
                        let observation = tools::dispatch(&name, &input)
                            .await
                            .unwrap_or_else(|e| format!("[error: {e}]"));
                        // Some models emit <tool name="answer"> instead of <answer> — treat as final answer
                        if let Some(ans) = observation.strip_prefix("__AGENT_ANSWER__:") {
                            let _ = tx.send(Some(format!("\n✅ **Answer:**\n{}\n", ans.trim())));
                            let _ = tx.send(None);
                            finished = true;
                            break;
                        }
                        let _ = tx.send(Some(format!("👁 **Observation:**\n```\n{}\n```\n\n", truncate(&observation, 800))));
                        history.push(("user".to_string(), format!("<observation>{observation}</observation>")));
                        // Advance past this tool block
                        if let Some(end) = remaining_resp.find("</tool>") {
                            remaining_resp = &remaining_resp[end + "</tool>".len()..];
                        }
                    }
                    StepKind::Answer(ans) => {
                        let _ = tx.send(Some(format!("\n✅ **Answer:**\n{ans}\n")));
                        let _ = tx.send(None);
                        finished = true;
                        break;
                    }
                    StepKind::Observation(_) => {}
                }
            }

            if finished {
                return Ok(());
            }
        }
    }

    async fn call_llm(&self, system: &str, history: &[(String, String)]) -> Result<String> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(1800))
            .build()?;

        let url = if !self.custom_url.is_empty() && self.provider == Provider::Ollama {
            // For Ollama with a custom server, append the chat endpoint if not already present
            let base = self.custom_url.trim_end_matches('/');
            format!("{}/api/chat", base)
        } else {
            self.provider.api_url().to_string()
        };

        // Build messages array — for Anthropic, system goes top-level;
        // for all others (OpenAI-compatible, Ollama) it goes as a system message.
        let resp_text = match self.provider {
            Provider::Anthropic => {
                let msgs: Vec<serde_json::Value> = history
                    .iter()
                    .map(|(role, content)| json!({ "role": role, "content": content }))
                    .collect();
                let body = json!({
                    "model": self.model,
                    "system": system,
                    "messages": msgs,
                    "stream": false,
                    "max_tokens": 4096,
                });
                client
                    .post(&url)
                    .header("x-api-key", &self.api_token)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body)
                    .send()
                    .await?
                    .text()
                    .await?
            }
            _ => {
                // OpenAI-compatible (Ollama, OpenAI, xAI, GitHubModels)
                let mut msgs: Vec<serde_json::Value> =
                    vec![json!({ "role": "system", "content": system })];
                for (role, content) in history {
                    msgs.push(json!({ "role": role, "content": content }));
                }
                let body = json!({
                    "model": self.model,
                    "messages": msgs,
                    "stream": false,
                    "max_tokens": 4096,
                });
                let mut req = client.post(&url).json(&body);
                if self.provider != Provider::Ollama && !self.api_token.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", self.api_token));
                }
                req.send().await?.text().await?
            }
        };

        let v: serde_json::Value = serde_json::from_str(&resp_text)?;

        // Anthropic format
        if let Some(content) = v["content"][0]["text"].as_str() {
            return Ok(content.to_string());
        }
        // OpenAI-compatible format
        if let Some(content) = v["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
        // Ollama non-streaming format
        if let Some(content) = v["message"]["content"].as_str() {
            return Ok(content.to_string());
        }

        Ok(format!("[unexpected response: {resp_text}]"))
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…(truncated)", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_tool_call_scaffold;

    #[test]
    fn detects_tool_call_scaffold_markers() {
        assert!(looks_like_tool_call_scaffold(
            r#"[TOOL_CALL] {tool => \"answer\", args {}}"#
        ));
        assert!(looks_like_tool_call_scaffold(
            r#"[[tool_use]] {\"tool\":\"shell\"}"#
        ));
        assert!(looks_like_tool_call_scaffold(
            r#"<tool_call>{\"tool\":\"read_file\"}</tool_call>"#
        ));
    }

    #[test]
    fn ignores_regular_text_without_tool_call_scaffold() {
        assert!(!looks_like_tool_call_scaffold("normal assistant answer"));
    }
}
