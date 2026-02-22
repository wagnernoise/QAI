use anyhow::Result;
use std::process::Command;

// ── Built-in tool dispatcher ──────────────────────────────────────────────────

/// Dispatch a tool call by name with the given input string.
/// Returns the tool output as a string.
pub async fn dispatch(tool: &str, input: &str) -> Result<String> {
    match tool {
        "read_file"   => read_file(input),
        "write_file"  => write_file(input),
        "edit_file"   => edit_file(input),
        "git_status"  => git_status(input),
        "git_diff"    => git_diff(input),
        "git_add"     => git_add(input),
        "git_commit"  => git_commit(input),
        "git_log"     => git_log(input),
        "shell"       => shell(input),
        "grep_search" => grep_search(input),
        "web_search"  => web_search(input).await,
        // Some models wrap their final answer in <tool name="answer"> instead of <answer>
        "answer" => Ok(format!("__AGENT_ANSWER__:{input}")),
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

/// Write content to a file, creating it (and parent dirs) if needed.
/// Input format: `<path>\n<content>`
fn write_file(input: &str) -> Result<String> {
    let input = input.trim_start();
    if let Some((path, content)) = input.split_once('\n') {
        let path = path.trim();
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("[write_file error creating dirs: {e}]"))?;
            }
        }
        match std::fs::write(path, content) {
            Ok(_) => Ok(format!("[write_file: wrote {} bytes to {path}]", content.len())),
            Err(e) => Ok(format!("[write_file error: {e}]")),
        }
    } else {
        Ok("[write_file error: input must be '<path>\\n<content>']".to_string())
    }
}

/// Edit a file by replacing the first occurrence of a search string with a replacement.
/// Input format: `<path>\n<<<\n<search>\n===\n<replacement>\n>>>`
fn edit_file(input: &str) -> Result<String> {
    let input = input.trim_start();
    // Expect: first line = path, then <<<\n<search>\n===\n<replacement>\n>>>
    let mut lines = input.splitn(2, '\n');
    let path = lines.next().unwrap_or("").trim();
    let rest = lines.next().unwrap_or("");

    let (search_part, replace_part) = if let Some(after_open) = rest.strip_prefix("<<<\n") {
        if let Some(mid) = after_open.find("\n===\n") {
            let search = &after_open[..mid];
            let after_eq = &after_open[mid + 5..]; // skip "\n===\n"
            let replacement = after_eq
                .strip_suffix("\n>>>")
                .or_else(|| after_eq.strip_suffix(">>>"))
                .unwrap_or(after_eq);
            (search, replacement)
        } else {
            return Ok("[edit_file error: missing '===' separator]".to_string());
        }
    } else {
        return Ok("[edit_file error: input must start with path then '<<<']".to_string());
    };

    match std::fs::read_to_string(path) {
        Err(e) => Ok(format!("[edit_file error reading {path}: {e}]")),
        Ok(original) => {
            if !original.contains(search_part) {
                return Ok(format!("[edit_file error: search string not found in {path}]"));
            }
            let updated = original.replacen(search_part, replace_part, 1);
            match std::fs::write(path, &updated) {
                Ok(_) => Ok(format!("[edit_file: applied edit to {path}]")),
                Err(e) => Ok(format!("[edit_file error writing {path}: {e}]")),
            }
        }
    }
}

/// Run `git status` in the given directory (or cwd if input is empty).
fn git_status(input: &str) -> Result<String> {
    git_run(&["status", "--short"], input)
}

/// Run `git diff` — optionally pass a file path or ref as input.
fn git_diff(input: &str) -> Result<String> {
    let arg = input.trim();
    if arg.is_empty() {
        git_run(&["diff"], "")
    } else {
        git_run(&["diff", arg], "")
    }
}

/// Stage files with `git add`. Input: space-separated paths (or `.` for all).
fn git_add(input: &str) -> Result<String> {
    let arg = input.trim();
    if arg.is_empty() {
        return Ok("[git_add error: provide path(s) to stage, e.g. '.' or 'src/main.rs']".to_string());
    }
    git_run(&["add", arg], "")
}

/// Commit staged changes. Input: commit message.
fn git_commit(input: &str) -> Result<String> {
    let msg = input.trim();
    if msg.is_empty() {
        return Ok("[git_commit error: commit message must not be empty]".to_string());
    }
    git_run(&["commit", "-m", msg], "")
}

/// Show recent git log. Input: optional number of entries (default 10).
fn git_log(input: &str) -> Result<String> {
    let n = input.trim().parse::<usize>().unwrap_or(10);
    let n_str = format!("-{n}");
    git_run(&["log", "--oneline", &n_str], "")
}

/// Helper: run a git sub-command, optionally in a specific working directory.
fn git_run(args: &[&str], workdir: &str) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    let wd = workdir.trim();
    if !wd.is_empty() {
        cmd.current_dir(wd);
    }
    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = format!("{stdout}{stderr}").trim().to_string();
            Ok(if combined.is_empty() { "(no output)".to_string() } else { combined })
        }
        Err(e) => Ok(format!("[git error: {e}]")),
    }
}

/// Search file contents using a regex pattern.
/// Input format: `<pattern>\n<path>` (path is optional, defaults to `.`)
/// Optionally add a third line with a file glob filter, e.g. `*.rs`
fn grep_search(input: &str) -> Result<String> {
    let input = input.trim_start();
    let mut lines = input.splitn(3, '\n');
    let pattern = lines.next().unwrap_or("").trim();
    let path = lines.next().unwrap_or(".").trim();
    let path = if path.is_empty() { "." } else { path };
    let glob = lines.next().unwrap_or("").trim();

    if pattern.is_empty() {
        return Ok("[grep_search error: pattern must not be empty]".to_string());
    }

    let mut cmd = Command::new("grep");
    cmd.args(["-rn", "--color=never", pattern, path]);
    if !glob.is_empty() {
        cmd.arg(format!("--include={glob}"));
    }

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !stderr.is_empty() {
                return Ok(format!("[grep_search error: {stderr}]"));
            }
            if stdout.is_empty() {
                Ok("[grep_search: no matches found]".to_string())
            } else {
                // Limit output to 200 lines to avoid flooding the context
                let lines: Vec<&str> = stdout.lines().take(200).collect();
                let truncated = lines.len() < stdout.lines().count();
                let mut result = lines.join("\n");
                if truncated {
                    result.push_str("\n[... output truncated to 200 lines]");
                }
                Ok(result)
            }
        }
        Err(e) => Ok(format!("[grep_search error: {e}]")),
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
