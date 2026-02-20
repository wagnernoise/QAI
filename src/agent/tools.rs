use anyhow::Result;
use std::process::Command;

// ── Built-in tool dispatcher ──────────────────────────────────────────────────

/// Dispatch a tool call by name with the given input string.
/// Returns the tool output as a string.
pub async fn dispatch(tool: &str, input: &str) -> Result<String> {
    match tool {
        "read_file" => read_file(input),
        "shell" => shell(input),
        "web_search" => web_search(input).await,
        _ => Ok(format!("[unknown tool: {tool}]")),
    }
}

/// Read a file from the local filesystem.
fn read_file(path: &str) -> Result<String> {
    let path = path.trim();
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Ok(format!("[read_file error: {e}]")),
    }
}

/// Run a shell command and return its stdout + stderr.
fn shell(cmd: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = format!("{stdout}{stderr}").trim().to_string();
            Ok(if combined.is_empty() { "(no output)".to_string() } else { combined })
        }
        Err(e) => Ok(format!("[shell error: {e}]")),
    }
}

/// Perform a simple web search using DuckDuckGo instant-answer API.
async fn web_search(query: &str) -> Result<String> {
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding::encode(query.trim())
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client.get(&url).send().await;
    match resp {
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            // Extract AbstractText from JSON
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let abstract_text = v["AbstractText"].as_str().unwrap_or("").trim().to_string();
                if !abstract_text.is_empty() {
                    return Ok(abstract_text);
                }
                let answer = v["Answer"].as_str().unwrap_or("").trim().to_string();
                if !answer.is_empty() {
                    return Ok(answer);
                }
            }
            Ok("[web_search: no result found]".to_string())
        }
        Err(e) => Ok(format!("[web_search error: {e}]")),
    }
}
