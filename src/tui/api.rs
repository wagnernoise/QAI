use anyhow::Result;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::tui::state::App;
use crate::tui::providers::Provider;

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("qai").join("config.toml"))
}

pub fn save_api_token(token: &str) -> Result<()> {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("api_token = \"{}\"\n", token.replace('"', "\\\"")))?;
    }
    Ok(())
}

pub fn load_api_token() -> Option<String> {
    let path = config_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("api_token = \"") {
            let val = rest.trim_end_matches('"');
            return Some(val.replace("\\\"", "\""));
        }
    }
    None
}

// ── Streaming API call ────────────────────────────────────────────────────────

pub struct StreamRequest {
    pub provider: Provider,
    pub api_token: String,
    pub custom_url: String,
    pub model: String,
    pub system_prompt: String,
    pub history: Vec<(String, String)>,
    pub tx: mpsc::UnboundedSender<Option<String>>,
    pub cancel: CancellationToken,
}

pub async fn stream_message(req: StreamRequest) -> Result<()> {
    let StreamRequest { provider, api_token, custom_url, model, system_prompt, history, tx, cancel } = req;
    use reqwest::Client;
    use serde_json::{json, Value};

    let token = api_token.trim().to_string();
    if token.is_empty() && provider != Provider::Ollama {
        anyhow::bail!("API token is empty");
    }

    let client = Client::new();

    match provider {
        Provider::Anthropic => {
            // Anthropic SSE streaming
            let msgs: Vec<Value> = history
                .iter()
                .map(|(r, c)| json!({"role": r, "content": c}))
                .collect();

            let body = json!({
                "model": model,
                "max_tokens": 4096,
                "system": system_prompt,
                "messages": msgs,
                "stream": true
            });

            let resp = client
                .post(provider.api_url())
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            let mut stream = resp.bytes_stream();
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = tx.send(None);
                        return Ok(());
                    }
                    chunk = stream.next() => {
                        match chunk {
                            None => break,
                            Some(Err(e)) => return Err(e.into()),
                            Some(Ok(bytes)) => {
                                let text = String::from_utf8_lossy(&bytes);
                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data == "[DONE]" { break; }
                                        if let Ok(v) = serde_json::from_str::<Value>(data) {
                                            if let Some(delta) = v["delta"]["text"].as_str() {
                                                let _ = tx.send(Some(delta.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(None);
            Ok(())
        }
        _ => {
            // OpenAI-compatible streaming (OpenAI, xAI, Ollama, Custom)
            let url = if provider == Provider::Custom {
                if custom_url.trim().is_empty() {
                    anyhow::bail!("Custom endpoint URL is empty");
                }
                custom_url.trim().to_string()
            } else {
                provider.api_url().to_string()
            };

            let mut msgs: Vec<Value> = vec![json!({"role": "system", "content": system_prompt})];
            for (r, c) in &history {
                msgs.push(json!({"role": r, "content": c}));
            }

            let body = json!({
                "model": model,
                "messages": msgs,
                "stream": true
            });

            let mut req = client.post(&url).json(&body);
            if !token.is_empty() {
                req = req.bearer_auth(token);
            }
            let resp = req.send().await?;

            let mut stream = resp.bytes_stream();
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = tx.send(None);
                        return Ok(());
                    }
                    chunk = stream.next() => {
                        match chunk {
                            None => break,
                            Some(Err(e)) => return Err(e.into()),
                            Some(Ok(bytes)) => {
                                let text = String::from_utf8_lossy(&bytes);
                                for line in text.lines() {
                                    // Ollama NDJSON: each line is a full JSON object
                                    // OpenAI SSE: lines start with "data: "
                                    let data = if let Some(d) = line.strip_prefix("data: ") {
                                        if d == "[DONE]" { continue; }
                                        d
                                    } else {
                                        line
                                    };
                                    if data.is_empty() { continue; }
                                    if let Ok(v) = serde_json::from_str::<Value>(data) {
                                        // OpenAI-style delta
                                        if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                                            if !delta.is_empty() {
                                                let _ = tx.send(Some(delta.to_string()));
                                            }
                                        }
                                        // Ollama NDJSON style
                                        else if let Some(delta) = v["message"]["content"].as_str() {
                                            if !delta.is_empty() {
                                                let _ = tx.send(Some(delta.to_string()));
                                            }
                                        }
                                        // Check if done
                                        if v["done"].as_bool().unwrap_or(false) {
                                            let _ = tx.send(None);
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(None);
            Ok(())
        }
    }
}



pub async fn fetch_ollama_models(app: &mut App) {
    use reqwest::Client;
    use serde_json::Value;

    let base = if app.selected_provider() == Provider::Ollama {
        "http://localhost:11434"
    } else {
        return;
    };

    app.status = "Fetching Ollama models…".to_string();
    let client = Client::new();
    match client
        .get(format!("{base}/api/tags"))
        .send()
        .await
    {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(json) => {
                let models: Vec<String> = json["models"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect();
                if models.is_empty() {
                    app.status = "No Ollama models found. Pull one with: ollama pull <model>".to_string();
                } else {
                    app.status = format!("Found {} model(s). Use ↑/↓ to select.", models.len());
                    if !models.is_empty() {
                        app.model_input = models[0].clone();
                    }
                    app.model_list_state.select(Some(0));
                    app.ollama_models = models;
                }
            }
            Err(e) => {
                app.status = format!("Failed to parse Ollama response: {e}");
            }
        },
        Err(e) => {
            app.status = format!("Cannot reach Ollama at {base}: {e}");
        }
    }
}

// ── API token persistence ─────────────────────────────────────────────────────

