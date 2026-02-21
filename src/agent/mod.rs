pub mod tools;

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

use crate::tui::providers::Provider;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of Reason→Act→Observe iterations before giving up.
pub const MAX_STEPS: usize = 10;

// ── ReAct step types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    Thought,
    ToolCall { name: String, input: String },
    Observation(String),
    Answer(String),
}

// ── Tag parsing ───────────────────────────────────────────────────────────────

/// Parse the first recognised tag from an LLM response string.
/// Returns `(StepKind, remainder)` or `None` if no tag is found.
pub fn parse_step(text: &str) -> Option<StepKind> {
    // <answer>…</answer>
    if let Some(inner) = extract_tag(text, "answer") {
        return Some(StepKind::Answer(inner.trim().to_string()));
    }
    // <tool name="…">…</tool>  or  <tool>name\ninput</tool>
    if let Some(inner) = extract_tag(text, "tool") {
        // Try attribute form: <tool name="read_file">path</tool>
        // The attribute lives in the original text's opening tag, not in `inner`.
        let name_attr = extract_attr(text, "name");
        if let Some(name) = name_attr {
            // `inner` is already the content between > and </tool>
            return Some(StepKind::ToolCall { name, input: inner.trim().to_string() });
        }
        // Fallback: first line = tool name, rest = input
        let mut lines = inner.trim().splitn(2, '\n');
        let name = lines.next().unwrap_or("").trim().to_string();
        let input = lines.next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            return Some(StepKind::ToolCall { name, input });
        }
    }
    // <think>…</think>
    if extract_tag(text, "think").is_some() {
        return Some(StepKind::Thought);
    }
    None
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = text.find(&open)?;
    // skip to end of opening tag
    let tag_end = text[start..].find('>')?  + start + 1;
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

// ── ReActAgent ────────────────────────────────────────────────────────────────

pub struct ReActAgent {
    pub provider: Provider,
    pub api_token: String,
    pub custom_url: String,
    pub model: String,
    pub system_prompt: String,
    pub max_steps: usize,
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
            max_steps: MAX_STEPS,
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
        // Seed history with all prior turns, then append the current task
        // (skip the last entry if it's already the current user message)
        let mut history: Vec<(String, String)> = prior_history
            .into_iter()
            .filter(|(role, content)| !(role == "user" && content == &task))
            .collect();
        history.push(("user".to_string(), task.clone()));

        for step in 0..self.max_steps {
            // Build the LLM prompt with ReAct instructions appended
            let react_system = format!(
                "{}\n\n\
                You are operating in ReAct mode. For each step you MUST output exactly one of:\n\
                  <think>your reasoning</think>\n\
                  <tool name=\"TOOL_NAME\">tool input</tool>  (available: read_file, shell, web_search)\n\
                  <answer>final answer to the user</answer>\n\
                Do NOT output plain text outside these tags.",
                self.system_prompt
            );

            let llm_response = self
                .call_llm(&react_system, &history)
                .await
                .unwrap_or_else(|e| format!("<answer>[LLM error: {e}]</answer>"));

            match parse_step(&llm_response) {
                Some(StepKind::Thought) => {
                    if let Some(inner) = extract_tag(&llm_response, "think") {
                        let _ = tx.send(Some(format!("💭 **Thought:** {}\n\n", inner.trim())));
                    }
                    history.push(("assistant".to_string(), llm_response));
                }
                Some(StepKind::ToolCall { name, input }) => {
                    let _ = tx.send(Some(format!("🔧 **Tool:** `{name}({input})`\n")));
                    let observation = tools::dispatch(&name, &input).await.unwrap_or_else(|e| format!("[error: {e}]"));
                    let _ = tx.send(Some(format!("👁 **Observation:** {observation}\n\n")));
                    history.push(("assistant".to_string(), llm_response));
                    history.push(("user".to_string(), format!("<observation>{observation}</observation>")));
                }
                Some(StepKind::Answer(ans)) => {
                    let _ = tx.send(Some(format!("✅ **Answer:**\n{ans}")));
                    let _ = tx.send(None);
                    return Ok(());
                }
                _ => {
                    // No recognised tag — treat the whole response as the answer
                    let _ = tx.send(Some(llm_response.clone()));
                    history.push(("assistant".to_string(), llm_response));
                    let _ = tx.send(None);
                    return Ok(());
                }
            }

            // Safety: if we're on the last step, force an answer
            if step == self.max_steps - 1 {
                let _ = tx.send(Some(
                    "\n⚠️ **Max steps reached.** Stopping agent loop.\n".to_string(),
                ));
            }
        }

        let _ = tx.send(None);
        Ok(())
    }

    async fn call_llm(&self, system: &str, history: &[(String, String)]) -> Result<String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let msgs: Vec<serde_json::Value> = history
            .iter()
            .map(|(role, content)| json!({ "role": role, "content": content }))
            .collect();

        let url = if self.provider == Provider::Custom && !self.custom_url.is_empty() {
            self.custom_url.clone()
        } else {
            self.provider.api_url().to_string()
        };

        let body = json!({
            "model": self.model,
            "system": system,
            "messages": msgs,
            "stream": false,
            "max_tokens": 2048,
        });

        let mut req = client.post(&url).json(&body);

        match self.provider {
            Provider::Anthropic => {
                req = req
                    .header("x-api-key", &self.api_token)
                    .header("anthropic-version", "2023-06-01");
            }
            Provider::Ollama => {}
            _ => {
                req = req.header("Authorization", format!("Bearer {}", self.api_token));
            }
        }

        let resp = req.send().await?.text().await?;
        let v: serde_json::Value = serde_json::from_str(&resp)?;

        // Anthropic format
        if let Some(content) = v["content"][0]["text"].as_str() {
            return Ok(content.to_string());
        }
        // OpenAI-compatible format
        if let Some(content) = v["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
        Ok(format!("[unexpected response: {resp}]"))
    }
}
