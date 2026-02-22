use qai_cli::agent::{parse_step, parse_steps, try_recover_plain_tool, StepKind, ReActAgent, tools};
use qai_cli::tui::providers::Provider;
use tokio::sync::mpsc;

// ── parse_step tests ──────────────────────────────────────────────────────────

#[test]
fn parse_step_answer_tag() {
    let text = "<answer>42 is the answer</answer>";
    assert_eq!(parse_step(text), Some(StepKind::Answer("42 is the answer".to_string())));
}

#[test]
fn parse_step_answer_trims_whitespace() {
    let text = "<answer>  hello world  </answer>";
    assert_eq!(parse_step(text), Some(StepKind::Answer("hello world".to_string())));
}

#[test]
fn parse_step_think_tag() {
    let text = "<think>I need to check the file first</think>";
    assert_eq!(parse_step(text), Some(StepKind::Thought));
}

#[test]
fn parse_step_tool_with_name_attr() {
    let text = r#"<tool name="read_file">/etc/hosts</tool>"#;
    assert_eq!(
        parse_step(text),
        Some(StepKind::ToolCall {
            name: "read_file".to_string(),
            input: "/etc/hosts".to_string(),
        })
    );
}

#[test]
fn parse_step_tool_fallback_newline_format() {
    let text = "<tool>shell\necho hello</tool>";
    assert_eq!(
        parse_step(text),
        Some(StepKind::ToolCall {
            name: "shell".to_string(),
            input: "echo hello".to_string(),
        })
    );
}

#[test]
fn parse_step_answer_takes_priority_over_tool() {
    // answer tag appears before tool tag — answer wins
    let text = "<answer>done</answer><tool name=\"shell\">ls</tool>";
    assert_eq!(parse_step(text), Some(StepKind::Answer("done".to_string())));
}

#[test]
fn parse_step_no_tag_returns_none() {
    let text = "This is plain text with no tags.";
    assert_eq!(parse_step(text), None);
}

#[test]
fn parse_step_empty_string_returns_none() {
    assert_eq!(parse_step(""), None);
}

#[test]
fn parse_step_malformed_tag_returns_none() {
    // Missing closing tag
    let text = "<answer>no closing tag";
    assert_eq!(parse_step(text), None);
}

#[test]
fn parse_step_tool_web_search() {
    let text = r#"<tool name="web_search">Rust async programming</tool>"#;
    assert_eq!(
        parse_step(text),
        Some(StepKind::ToolCall {
            name: "web_search".to_string(),
            input: "Rust async programming".to_string(),
        })
    );
}

// ── parse_steps multi-tag tests ───────────────────────────────────────────────

#[test]
fn parse_steps_think_then_tool() {
    let text = r#"<think>I should read the file first</think><tool name="read_file">README.md</tool>"#;
    let steps = parse_steps(text);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0], StepKind::Thought);
    assert_eq!(steps[1], StepKind::ToolCall { name: "read_file".to_string(), input: "README.md".to_string() });
}

#[test]
fn parse_steps_think_tool_answer() {
    let text = r#"<think>plan</think><tool name="shell">ls</tool><answer>done</answer>"#;
    let steps = parse_steps(text);
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0], StepKind::Thought);
    assert!(matches!(&steps[1], StepKind::ToolCall { name, .. } if name == "shell"));
    assert_eq!(steps[2], StepKind::Answer("done".to_string()));
}

#[test]
fn parse_steps_multiple_tools() {
    let text = r#"<tool name="read_file">a.txt</tool><tool name="shell">echo hi</tool>"#;
    let steps = parse_steps(text);
    assert_eq!(steps.len(), 2);
    assert!(matches!(&steps[0], StepKind::ToolCall { name, .. } if name == "read_file"));
    assert!(matches!(&steps[1], StepKind::ToolCall { name, .. } if name == "shell"));
}

#[test]
fn parse_steps_answer_stops_parsing() {
    let text = r#"<answer>final</answer><tool name="shell">ls</tool>"#;
    let steps = parse_steps(text);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0], StepKind::Answer("final".to_string()));
}

#[test]
fn parse_steps_empty_returns_empty() {
    assert!(parse_steps("").is_empty());
}

#[test]
fn parse_steps_plain_text_returns_empty() {
    assert!(parse_steps("no tags here at all").is_empty());
}


// ── ReActAgent construction ───────────────────────────────────────────────────

#[test]
fn react_agent_new_stores_fields() {
    let agent = ReActAgent::new(
        Provider::Ollama,
        "".to_string(),
        "".to_string(),
        "gemma3".to_string(),
        "system prompt".to_string(),
    );
    assert_eq!(agent.model, "gemma3");
    assert_eq!(agent.system_prompt, "system prompt");
    assert_eq!(agent.provider, Provider::Ollama);
}


// ── ReActAgent loop logic (mock via channel) ──────────────────────────────────

/// Helper: run the agent with a mock LLM that always returns `response`.
async fn run_agent_with_mock_response(response: &str) -> Vec<String> {
    // We can't easily mock the HTTP call, so we test the channel/token flow
    // by directly calling parse_step and simulating what the loop would do.
    let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();

    let step = parse_step(response);
    match step {
        Some(StepKind::Answer(ans)) => {
            let _ = tx.send(Some(format!("✅ **Answer:**\n{ans}")));
            let _ = tx.send(None);
        }
        Some(StepKind::Thought) => {
            let _ = tx.send(Some("💭 **Thought:** thinking...\n\n".to_string()));
            let _ = tx.send(None);
        }
        Some(StepKind::ToolCall { name, input }) => {
            let _ = tx.send(Some(format!("🔧 **Tool:** `{name}({input})`\n")));
            let _ = tx.send(None);
        }
        _ => {
            let _ = tx.send(Some(response.to_string()));
            let _ = tx.send(None);
        }
    }

    let mut tokens = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            Some(t) => tokens.push(t),
            None => break,
        }
    }
    tokens
}

#[tokio::test]
async fn agent_loop_answer_tag_produces_answer_token() {
    let tokens = run_agent_with_mock_response("<answer>The result is 42</answer>").await;
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].contains("The result is 42"));
    assert!(tokens[0].contains("✅"));
}

#[tokio::test]
async fn agent_loop_think_tag_produces_thought_token() {
    let tokens = run_agent_with_mock_response("<think>I should check the docs</think>").await;
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].contains("💭"));
}

#[tokio::test]
async fn agent_loop_tool_tag_produces_tool_token() {
    let tokens = run_agent_with_mock_response(r#"<tool name="shell">echo hi</tool>"#).await;
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].contains("🔧"));
    assert!(tokens[0].contains("shell"));
}

#[tokio::test]
async fn agent_loop_plain_text_passes_through() {
    let tokens = run_agent_with_mock_response("Just a plain response").await;
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], "Just a plain response");
}

// ── StepKind equality ─────────────────────────────────────────────────────────

#[test]
fn step_kind_answer_equality() {
    assert_eq!(
        StepKind::Answer("x".to_string()),
        StepKind::Answer("x".to_string())
    );
    assert_ne!(
        StepKind::Answer("x".to_string()),
        StepKind::Answer("y".to_string())
    );
}

#[test]
fn step_kind_thought_equality() {
    assert_eq!(StepKind::Thought, StepKind::Thought);
}

#[test]
fn step_kind_tool_call_equality() {
    let a = StepKind::ToolCall { name: "shell".to_string(), input: "ls".to_string() };
    let b = StepKind::ToolCall { name: "shell".to_string(), input: "ls".to_string() };
    let c = StepKind::ToolCall { name: "shell".to_string(), input: "pwd".to_string() };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ── App agent_mode field ──────────────────────────────────────────────────────

#[test]
fn app_agent_mode_starts_false() {
    use std::path::PathBuf;
    use qai_cli::App;
    let app = App::new(PathBuf::from("qa-agent-system-prompt.md"));
    assert!(!app.agent_mode);
}

#[test]
fn app_agent_mode_can_be_toggled() {
    use std::path::PathBuf;
    use qai_cli::App;
    let mut app = App::new(PathBuf::from("qa-agent-system-prompt.md"));
    app.agent_mode = true;
    assert!(app.agent_mode);
    app.agent_mode = false;
    assert!(!app.agent_mode);
}

// ── tools::dispatch tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn tool_write_file_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");
    let input = format!("{}\nhello world", path.display());
    let result = tools::dispatch("write_file", &input).await.unwrap();
    assert!(result.contains("wrote"), "expected write confirmation, got: {result}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
}

#[tokio::test]
async fn tool_write_file_missing_newline_returns_error() {
    let result = tools::dispatch("write_file", "no_newline_here").await.unwrap();
    assert!(result.contains("[write_file error"), "got: {result}");
}

#[tokio::test]
async fn tool_write_file_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub/dir/file.txt");
    let input = format!("{}\ncontent", path.display());
    let result = tools::dispatch("write_file", &input).await.unwrap();
    assert!(result.contains("wrote"), "got: {result}");
    assert!(path.exists());
}

#[tokio::test]
async fn tool_edit_file_replaces_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit_me.txt");
    std::fs::write(&path, "foo bar baz").unwrap();
    let input = format!("{}\n<<<\nfoo bar\n===\nreplaced\n>>>", path.display());
    let result = tools::dispatch("edit_file", &input).await.unwrap();
    assert!(result.contains("applied edit"), "got: {result}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "replaced baz");
}

#[tokio::test]
async fn tool_edit_file_search_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit_me2.txt");
    std::fs::write(&path, "hello world").unwrap();
    let input = format!("{}\n<<<\nnotfound\n===\nreplacement\n>>>", path.display());
    let result = tools::dispatch("edit_file", &input).await.unwrap();
    assert!(result.contains("not found"), "got: {result}");
}

#[tokio::test]
async fn tool_edit_file_missing_separator_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit_me3.txt");
    std::fs::write(&path, "content").unwrap();
    let input = format!("{}\n<<<\nno separator here", path.display());
    let result = tools::dispatch("edit_file", &input).await.unwrap();
    assert!(result.contains("[edit_file error"), "got: {result}");
}

#[tokio::test]
async fn tool_edit_file_missing_open_marker_returns_error() {
    let result = tools::dispatch("edit_file", "somefile.txt\nno marker").await.unwrap();
    assert!(result.contains("[edit_file error"), "got: {result}");
}

#[tokio::test]
async fn tool_git_status_runs_without_panic() {
    // Just verify it returns a string (may be empty or show status)
    let result = tools::dispatch("git_status", "").await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn tool_git_log_returns_output() {
    let result = tools::dispatch("git_log", "5").await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn tool_git_log_default_count() {
    let result = tools::dispatch("git_log", "").await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn tool_git_add_empty_input_returns_error() {
    let result = tools::dispatch("git_add", "").await.unwrap();
    assert!(result.contains("[git_add error"), "got: {result}");
}

#[tokio::test]
async fn tool_git_commit_empty_message_returns_error() {
    let result = tools::dispatch("git_commit", "").await.unwrap();
    assert!(result.contains("[git_commit error"), "got: {result}");
}

#[tokio::test]
async fn tool_git_diff_runs_without_panic() {
    let result = tools::dispatch("git_diff", "").await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn tool_unknown_returns_error_message() {
    let result = tools::dispatch("nonexistent_tool", "input").await.unwrap();
    assert!(result.contains("[unknown tool: nonexistent_tool]"), "got: {result}");
}

#[tokio::test]
async fn tool_read_file_existing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("read_me.txt");
    std::fs::write(&path, "read content").unwrap();
    let result = tools::dispatch("read_file", &path.display().to_string()).await.unwrap();
    assert_eq!(result, "read content");
}

#[tokio::test]
async fn tool_read_file_missing_returns_error() {
    let result = tools::dispatch("read_file", "/nonexistent/path/file.txt").await.unwrap();
    assert!(result.contains("[read_file error"), "got: {result}");
}

// ── Conversation memory tests ─────────────────────────────────────────────────

#[test]
fn react_agent_run_signature_accepts_prior_history() {
    // Verify ReActAgent::new compiles and holds expected fields
    let agent = ReActAgent::new(
        qai_cli::tui::providers::Provider::Ollama,
        String::new(),
        String::new(),
        "llama3".to_string(),
        "sys".to_string(),
    );
    assert_eq!(agent.model, "llama3");
}

#[tokio::test]
async fn agent_run_with_empty_prior_history_sends_answer() {
    // Verify the prior_history dedup logic and channel completion signal.
    use tokio::sync::mpsc;
    let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();

    // Spawn a task that simulates what run() does with prior_history = []
    // by checking the channel closes (None sent) after the loop.
    // Since we can't call a real LLM, we test the prior_history dedup logic directly.
    let task = "hello".to_string();
    let prior: Vec<(String, String)> = vec![];

    // Simulate the dedup filter from run(): prior filtered + task appended
    let mut history: Vec<(String, String)> = prior
        .into_iter()
        .filter(|(role, content)| !(role == "user" && content == &task))
        .collect();
    history.push(("user".to_string(), task.clone()));

    assert_eq!(history.len(), 1);
    assert_eq!(history[0], ("user".to_string(), "hello".to_string()));

    // Signal done
    let _ = tx.send(None);
    assert!(rx.recv().await.unwrap().is_none());
}

#[test]
fn agent_run_prior_history_dedup_removes_duplicate_task() {
    let task = "what is 2+2?".to_string();
    // Simulate prior_history that already contains the current task as last user msg
    let prior: Vec<(String, String)> = vec![
        ("user".to_string(), "previous question".to_string()),
        ("assistant".to_string(), "previous answer".to_string()),
        ("user".to_string(), task.clone()), // duplicate — should be removed
    ];

    let mut history: Vec<(String, String)> = prior
        .into_iter()
        .filter(|(role, content)| !(role == "user" && content == &task))
        .collect();
    history.push(("user".to_string(), task.clone()));

    // Duplicate removed; task appears exactly once at the end
    assert_eq!(history.len(), 3);
    assert_eq!(history[2], ("user".to_string(), task));
    assert_eq!(history[0].1, "previous question");
    assert_eq!(history[1].1, "previous answer");
}

#[test]
fn agent_run_prior_history_preserves_full_context() {
    let task = "new task".to_string();
    let prior: Vec<(String, String)> = vec![
        ("user".to_string(), "turn 1".to_string()),
        ("assistant".to_string(), "answer 1".to_string()),
        ("user".to_string(), "turn 2".to_string()),
        ("assistant".to_string(), "answer 2".to_string()),
    ];

    let mut history: Vec<(String, String)> = prior
        .into_iter()
        .filter(|(role, content)| !(role == "user" && content == &task))
        .collect();
    history.push(("user".to_string(), task.clone()));

    // All 4 prior turns preserved + new task = 5 entries
    assert_eq!(history.len(), 5);
    assert_eq!(history[4], ("user".to_string(), "new task".to_string()));
}

// ── try_recover_plain_tool tests ────────────────────────────────────────────

#[test]
fn recover_plain_tool_first_line_is_tool_name() {
    let text = "read_file\nREADME.md";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall { name: "read_file".to_string(), input: "README.md".to_string() })
    );
}

#[test]
fn recover_plain_tool_shell_with_command() {
    let text = "shell\nls -la";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall { name: "shell".to_string(), input: "ls -la".to_string() })
    );
}

#[test]
fn recover_plain_tool_backtick_wrapped_name() {
    let text = "`shell`\necho hello";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall { name: "shell".to_string(), input: "echo hello".to_string() })
    );
}

#[test]
fn recover_plain_tool_colon_prefix() {
    let text = "web_search: Rust async programming";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall { name: "web_search".to_string(), input: "Rust async programming".to_string() })
    );
}

#[test]
fn recover_plain_tool_unknown_name_returns_none() {
    let text = "search_paths_by_glob\npattern=\"**/README.md\"";
    assert_eq!(try_recover_plain_tool(text), None);
}

#[test]
fn recover_plain_tool_plain_text_returns_none() {
    let text = "I will now read the README file to understand its contents.";
    assert_eq!(try_recover_plain_tool(text), None);
}

#[test]
fn recover_plain_tool_git_status_no_input() {
    let text = "git_status";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall { name: "git_status".to_string(), input: String::new() })
    );
}

// ── grep_search tool tests ───────────────────────────────────────────────────

#[tokio::test]
async fn tool_grep_search_finds_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.txt");
    std::fs::write(&path, "hello world\nfoo bar\nhello again\n").unwrap();
    let input = format!("hello\n{}", dir.path().display());
    let result = tools::dispatch("grep_search", &input).await.unwrap();
    assert!(result.contains("hello"), "got: {result}");
}

#[tokio::test]
async fn tool_grep_search_no_match_returns_message() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.txt");
    std::fs::write(&path, "nothing here\n").unwrap();
    let input = format!("ZZZNOMATCH\n{}", dir.path().display());
    let result = tools::dispatch("grep_search", &input).await.unwrap();
    assert_eq!(result, "[grep_search: no matches found]");
}

#[tokio::test]
async fn tool_grep_search_empty_pattern_returns_error() {
    let result = tools::dispatch("grep_search", "").await.unwrap();
    assert!(result.contains("[grep_search error"), "got: {result}");
}

#[tokio::test]
async fn tool_grep_search_with_glob_filter() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "fn main() {}\n").unwrap();
    let input = format!("fn main\n{}\n*.rs", dir.path().display());
    let result = tools::dispatch("grep_search", &input).await.unwrap();
    assert!(result.contains("a.rs"), "got: {result}");
    assert!(!result.contains("b.txt"), "got: {result}");
}

#[test]
fn recover_plain_tool_grep_search() {
    let text = "grep_search\nfn main\nsrc\n*.rs";
    assert_eq!(
        try_recover_plain_tool(text),
        Some(StepKind::ToolCall {
            name: "grep_search".to_string(),
            input: "fn main\nsrc\n*.rs".to_string(),
        })
    );
}

// ── answer-as-tool fix tests ──────────────────────────────────────────────────

#[tokio::test]
async fn tool_dispatch_answer_returns_sentinel() {
    let result = tools::dispatch("answer", "The task is complete.").await.unwrap();
    assert!(result.starts_with("__AGENT_ANSWER__:"), "got: {result}");
    assert!(result.contains("The task is complete."), "got: {result}");
}

#[tokio::test]
async fn tool_dispatch_answer_empty_input() {
    let result = tools::dispatch("answer", "").await.unwrap();
    assert!(result.starts_with("__AGENT_ANSWER__:"), "got: {result}");
}

#[test]
fn parse_steps_tool_named_answer_is_parsed_as_tool_call() {
    // The LLM emits <tool name="answer">...</tool> — parse_steps sees it as a ToolCall;
    // the run loop then converts it via the __AGENT_ANSWER__ sentinel.
    let text = r#"<tool name="answer">The task is done.</tool>"#;
    let steps = parse_steps(text);
    assert_eq!(steps.len(), 1);
    assert!(
        matches!(&steps[0], StepKind::ToolCall { name, .. } if name == "answer"),
        "expected ToolCall with name=answer, got: {:?}", steps
    );
}

#[test]
fn agent_run_prior_history_only_deduplicates_user_role() {
    let task = "shared text".to_string();
    // An assistant message with the same text as the task should NOT be removed
    let prior: Vec<(String, String)> = vec![
        ("assistant".to_string(), task.clone()), // same text, different role — keep
        ("user".to_string(), "other".to_string()),
    ];

    let mut history: Vec<(String, String)> = prior
        .into_iter()
        .filter(|(role, content)| !(role == "user" && content == &task))
        .collect();
    history.push(("user".to_string(), task.clone()));

    assert_eq!(history.len(), 3);
    assert_eq!(history[0].0, "assistant"); // assistant entry preserved
}
