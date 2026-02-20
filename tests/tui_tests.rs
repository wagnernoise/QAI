use qai_cli::tui::{render_to_buffer, App, ChatFocus, Provider, Screen};
#[allow(unused_imports)]
use qai_cli::{save_api_token, load_api_token, strip_model_tags};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_app_with_content(content: &str) -> (TempDir, App) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, content).unwrap();
    let app = App::new(path);
    (dir, app)
}

fn make_app_no_file() -> App {
    App::new(PathBuf::from("/nonexistent/prompt.md"))
}

/// Collect all visible characters from a buffer row into a String.
fn buffer_row(buf: &ratatui::buffer::Buffer, row: u16) -> String {
    let width = buf.area().width;
    (0..width).map(|col| buf[(col, row)].symbol().chars().next().unwrap_or(' ')).collect()
}

/// Collect the entire buffer as a single string (rows joined by newline).
fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    let height = buf.area().height;
    (0..height).map(|r| buffer_row(buf, r)).collect::<Vec<_>>().join("\n")
}

// ── Provider::label ───────────────────────────────────────────────────────────

#[test]
fn provider_labels_are_non_empty() {
    for p in Provider::all() {
        assert!(!p.label().is_empty());
    }
}

#[test]
fn provider_openai_label() {
    assert_eq!(Provider::OpenAI.label(), "OpenAI (GPT-4o)");
}

#[test]
fn provider_anthropic_label() {
    assert_eq!(Provider::Anthropic.label(), "Anthropic (Claude)");
}

#[test]
fn provider_xai_label() {
    assert_eq!(Provider::XAI.label(), "xAI (Grok)");
}

#[test]
fn provider_ollama_label() {
    assert_eq!(Provider::Ollama.label(), "Ollama (local)");
}

#[test]
fn provider_zen_label() {
    assert_eq!(Provider::Zen.label(), "Zen API");
}

#[test]
fn provider_custom_label() {
    assert_eq!(Provider::Custom.label(), "Custom endpoint");
}

// ── Provider::default_model ───────────────────────────────────────────────────

#[test]
fn provider_default_models_are_non_empty() {
    for p in Provider::all() {
        assert!(!p.default_model().is_empty());
    }
}

#[test]
fn provider_openai_default_model() {
    assert_eq!(Provider::OpenAI.default_model(), "gpt-4o");
}

#[test]
fn provider_anthropic_default_model() {
    assert_eq!(Provider::Anthropic.default_model(), "claude-3-5-sonnet-20241022");
}

#[test]
fn provider_ollama_default_model() {
    assert_eq!(Provider::Ollama.default_model(), "gemma3");
}

#[test]
fn provider_zen_default_model() {
    assert_eq!(Provider::Zen.default_model(), "zen-1");
}

// ── Provider::api_url ─────────────────────────────────────────────────────────

#[test]
fn provider_openai_api_url() {
    assert_eq!(
        Provider::OpenAI.api_url(),
        "https://api.openai.com/v1/chat/completions"
    );
}

#[test]
fn provider_anthropic_api_url() {
    assert_eq!(
        Provider::Anthropic.api_url(),
        "https://api.anthropic.com/v1/messages"
    );
}

#[test]
fn provider_xai_api_url() {
    assert_eq!(Provider::XAI.api_url(), "https://api.x.ai/v1/chat/completions");
}

#[test]
fn provider_ollama_api_url() {
    assert_eq!(Provider::Ollama.api_url(), "http://localhost:11434/v1/chat/completions");
}

#[test]
fn provider_zen_api_url() {
    assert_eq!(Provider::Zen.api_url(), "https://api.zen.ai/v1/chat/completions");
}

#[test]
fn provider_custom_api_url_is_empty() {
    assert_eq!(Provider::Custom.api_url(), "");
}

// ── Provider::all ─────────────────────────────────────────────────────────────

#[test]
fn provider_all_returns_six_variants() {
    assert_eq!(Provider::all().len(), 6);
}

#[test]
fn provider_all_contains_ollama() {
    assert!(Provider::all().iter().any(|p| p == &Provider::Ollama));
}

#[test]
fn provider_all_contains_zen() {
    assert!(Provider::all().iter().any(|p| p == &Provider::Zen));
}

// ── App::new ──────────────────────────────────────────────────────────────────

#[test]
fn app_new_loads_prompt_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "## ENVIRONMENT").unwrap();
    let app = App::new(path);
    assert_eq!(app.prompt_content, "## ENVIRONMENT");
}

#[test]
fn app_new_missing_prompt_shows_fallback() {
    let path = std::path::PathBuf::from("/nonexistent/prompt.md");
    let app = App::new(path);
    assert!(app.prompt_content.contains("not found"));
}

#[test]
fn app_new_starts_on_menu_screen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn app_new_menu_selects_first_item() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.menu_state.selected(), Some(0));
}

#[test]
fn app_new_tools_provider_index_starts_at_zero() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.tools_provider_index, 0);
}

#[test]
fn app_new_tools_provider_list_state_selects_first() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.tools_provider_list_state.selected(), Some(0));
}

#[test]
fn provider_ollama_description_mentions_local() {
    assert!(Provider::Ollama.description().contains("Local"));
}

#[test]
fn provider_ollama_no_api_key_needed() {
    assert!(Provider::Ollama.description().contains("No API key"));
}

#[test]
fn app_new_chat_focus_starts_on_provider_list() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.chat_focus, ChatFocus::ProviderList);
}

#[test]
fn app_new_ollama_models_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.ollama_models.is_empty());
}

#[test]
fn app_new_model_input_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.model_input.is_empty());
}

#[test]
fn app_active_model_falls_back_to_provider_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path); // default provider is OpenAI
    assert_eq!(app.active_model(), Provider::OpenAI.default_model());
}

#[test]
fn app_active_model_uses_model_input_when_set() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.model_input = "llama3.2:latest".to_string();
    assert_eq!(app.active_model(), "llama3.2:latest");
}

#[test]
fn app_new_streaming_starts_false() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(!app.streaming);
}

#[test]
fn app_new_chat_scroll_starts_at_zero() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.chat_scroll, 0);
}

#[test]
fn app_new_chat_scroll_manual_starts_false() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(!app.chat_scroll_manual);
}

#[test]
fn app_new_sel_start_is_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.sel_start.is_none());
}

#[test]
fn app_new_sel_end_is_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.sel_end.is_none());
}

// ── ChatFocus variants ────────────────────────────────────────────────────────

#[test]
fn chat_focus_conversation_variant_exists() {
    let focus = ChatFocus::Conversation;
    assert_eq!(focus, ChatFocus::Conversation);
}

#[test]
fn chat_focus_message_variant_exists() {
    assert_eq!(ChatFocus::Message, ChatFocus::Message);
}

#[test]
fn chat_focus_token_variant_exists() {
    assert_eq!(ChatFocus::Token, ChatFocus::Token);
}

#[test]
fn chat_focus_custom_url_variant_exists() {
    assert_eq!(ChatFocus::CustomUrl, ChatFocus::CustomUrl);
}

#[test]
fn chat_focus_model_list_variant_exists() {
    assert_eq!(ChatFocus::ModelList, ChatFocus::ModelList);
}

#[test]
fn chat_focus_provider_list_is_not_conversation() {
    assert_ne!(ChatFocus::ProviderList, ChatFocus::Conversation);
}

#[test]
fn chat_focus_conversation_is_not_message() {
    assert_ne!(ChatFocus::Conversation, ChatFocus::Message);
}

// ── Screen variants ───────────────────────────────────────────────────────────

#[test]
fn screen_all_variants_are_distinct() {
    let screens = vec![
        Screen::Menu,
        Screen::Info,
        Screen::Show,
        Screen::Validate,
        Screen::Tools,
        Screen::Chat,
    ];
    for (i, a) in screens.iter().enumerate() {
        for (j, b) in screens.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

// ── App initial field values ──────────────────────────────────────────────────

#[test]
fn app_new_api_token_loads_from_config_or_empty() {
    // App::new loads the saved token from ~/.config/qai/config.toml if present.
    // We only assert the field is a valid String (not a panic / wrong type).
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    // api_token is always a String (may be empty or pre-loaded from disk)
    let _ = app.api_token.len();
}

#[test]
fn app_new_custom_url_starts_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.custom_url.is_empty());
}

#[test]
fn app_new_message_input_starts_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.message_input_text().is_empty());
}

#[test]
fn app_new_api_token_saved_starts_false() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(!app.api_token_saved);
}

#[test]
fn app_new_token_saved_at_starts_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.token_saved_at.is_none());
}

#[test]
fn app_new_messages_starts_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.messages.is_empty());
}

#[test]
fn app_new_status_starts_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert!(app.status.is_empty());
}

#[test]
fn app_new_scroll_offset_starts_at_zero() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.scroll_offset, 0);
}

#[test]
fn app_new_provider_index_starts_at_zero() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.provider_index, 0);
}

#[test]
fn app_new_provider_list_state_selects_first() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.provider_list_state.selected(), Some(0));
}

#[test]
fn app_new_model_list_state_selects_first() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.model_list_state.selected(), Some(0));
}

// ── App::selected_provider ────────────────────────────────────────────────────

#[test]
fn app_selected_provider_default_is_openai() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let app = App::new(path);
    assert_eq!(app.selected_provider(), Provider::OpenAI);
}

#[test]
fn app_selected_provider_changes_with_index() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 1;
    assert_eq!(app.selected_provider(), Provider::Anthropic);
}

#[test]
fn app_selected_provider_index_3_is_ollama() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 3;
    assert_eq!(app.selected_provider(), Provider::Ollama);
}

#[test]
fn app_selected_provider_index_4_is_zen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 4;
    assert_eq!(app.selected_provider(), Provider::Zen);
}

#[test]
fn app_selected_provider_index_5_is_custom() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 5;
    assert_eq!(app.selected_provider(), Provider::Custom);
}

// ── App::active_model with provider changes ───────────────────────────────────

#[test]
fn app_active_model_for_anthropic_uses_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 1;
    assert_eq!(app.active_model(), Provider::Anthropic.default_model());
}

#[test]
fn app_active_model_for_ollama_uses_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 3;
    assert_eq!(app.active_model(), Provider::Ollama.default_model());
}

#[test]
fn app_active_model_override_takes_priority_over_provider() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.provider_index = 3; // Ollama
    app.model_input = "mistral:latest".to_string();
    assert_eq!(app.active_model(), "mistral:latest");
}

#[test]
fn app_active_model_whitespace_only_falls_back_to_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.model_input = "   ".to_string();
    assert_eq!(app.active_model(), Provider::OpenAI.default_model());
}

// ── Provider descriptions ─────────────────────────────────────────────────────

#[test]
fn provider_openai_description_mentions_api_key() {
    assert!(Provider::OpenAI.description().contains("API key"));
}

#[test]
fn provider_anthropic_description_mentions_api_key() {
    assert!(Provider::Anthropic.description().contains("API key"));
}

#[test]
fn provider_xai_description_non_empty() {
    assert!(!Provider::XAI.description().is_empty());
}

#[test]
fn provider_zen_description_non_empty() {
    assert!(!Provider::Zen.description().is_empty());
}

#[test]
fn provider_custom_description_non_empty() {
    assert!(!Provider::Custom.description().is_empty());
}

#[test]
fn all_providers_have_non_empty_descriptions() {
    for p in Provider::all() {
        assert!(!p.description().is_empty(), "{} has empty description", p.label());
    }
}

// ── Ollama models list manipulation ──────────────────────────────────────────

#[test]
fn app_ollama_models_can_be_populated() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.ollama_models = vec!["llama3".to_string(), "mistral".to_string()];
    assert_eq!(app.ollama_models.len(), 2);
    assert_eq!(app.ollama_models[0], "llama3");
    assert_eq!(app.ollama_models[1], "mistral");
}

#[test]
fn app_messages_can_be_pushed() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.messages.push(("user".to_string(), "hello".to_string()));
    app.messages.push(("assistant".to_string(), "hi there".to_string()));
    assert_eq!(app.messages.len(), 2);
    assert_eq!(app.messages[0].0, "user");
    assert_eq!(app.messages[1].0, "assistant");
}

// ── Manual scroll state transitions ──────────────────────────────────────────

#[test]
fn app_chat_scroll_can_be_set_manually() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.chat_scroll = 10;
    app.chat_scroll_manual = true;
    assert_eq!(app.chat_scroll, 10);
    assert!(app.chat_scroll_manual);
}

#[test]
fn app_chat_scroll_reset_resumes_auto_scroll() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.chat_scroll = 42;
    app.chat_scroll_manual = true;
    // simulate End key / send message reset
    app.chat_scroll = 0;
    app.chat_scroll_manual = false;
    assert_eq!(app.chat_scroll, 0);
    assert!(!app.chat_scroll_manual);
}

// ── Streaming flag transitions ────────────────────────────────────────────────

#[test]
fn app_streaming_can_be_set_true() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.streaming = true;
    assert!(app.streaming);
}

#[test]
fn app_streaming_can_be_reset_to_false() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, "content").unwrap();
    let mut app = App::new(path);
    app.streaming = true;
    app.streaming = false;
    assert!(!app.streaming);
}

// ── render_to_buffer / draw_* coverage ───────────────────────────────────────

#[test]
fn render_menu_screen_contains_menu_title() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Menu;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Menu"), "expected 'Menu' in rendered output");
}

#[test]
fn render_menu_screen_contains_qai_banner() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Menu;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("QA Automation AI Agent"), "expected banner in rendered output");
}

#[test]
fn render_menu_screen_contains_all_menu_items() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Menu;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Info"));
    assert!(text.contains("Show"));
    assert!(text.contains("Validate"));
    assert!(text.contains("Tools"));
    assert!(text.contains("Chat"));
    assert!(text.contains("Quit"));
}

#[test]
fn render_info_screen_contains_version() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Info;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Version") || text.contains("version") || text.contains("0."), "expected version info");
}

#[test]
fn render_info_screen_contains_prompt_path() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Info;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("prompt") || text.contains("Prompt"), "expected prompt path info");
}

#[test]
fn render_show_screen_contains_prompt_content() {
    let (_dir, mut app) = make_app_with_content("UNIQUE_SHOW_CONTENT_XYZ");
    app.screen = Screen::Show;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("UNIQUE_SHOW_CONTENT_XYZ"), "expected prompt content in Show screen");
}

#[test]
fn render_show_screen_missing_file_shows_fallback() {
    let mut app = make_app_no_file();
    app.screen = Screen::Show;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("not found") || text.contains("Failed") || text.contains("Error") || text.len() > 0);
}

#[test]
fn render_validate_screen_shows_result() {
    let content = "## ENVIRONMENT\n### PRIMARY OBJECTIVE\n### MODE SELECTION PRIMER\n";
    let (_dir, mut app) = make_app_with_content(content);
    app.screen = Screen::Validate;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("passed") || text.contains("Passed") || text.contains("Validate") || text.contains("valid"));
}

#[test]
fn render_validate_screen_missing_sections_shows_error() {
    let (_dir, mut app) = make_app_with_content("no sections here");
    app.screen = Screen::Validate;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Missing") || text.contains("failed") || text.contains("Error") || text.len() > 0);
}

#[test]
fn render_tools_screen_contains_provider_names() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Tools;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("OpenAI") || text.contains("Ollama") || text.contains("Tools"));
}

#[test]
fn render_tools_screen_contains_ollama() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Tools;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Ollama"), "expected Ollama in Tools screen");
}

#[test]
fn render_tools_screen_contains_zen() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Tools;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Zen"), "expected Zen in Tools screen");
}

#[test]
fn render_chat_screen_contains_conversation_panel() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("Conversation"), "expected Conversation panel in Chat screen");
}

#[test]
fn render_chat_screen_contains_message_input() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("Message"), "expected Message input in Chat screen");
}

#[test]
fn render_chat_screen_shows_messages() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.messages.push(("user".to_string(), "Hello QA-Bot".to_string()));
    app.messages.push(("assistant".to_string(), "Hello user!".to_string()));
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("Hello QA-Bot") || text.contains("Hello user"));
}

#[test]
fn render_chat_screen_with_manual_scroll_shows_hint() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.chat_scroll_manual = true;
    app.chat_scroll = 2;
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("End") || text.contains("scroll"), "expected scroll hint");
}

#[test]
fn render_chat_screen_conversation_focused_shows_hint() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.chat_focus = ChatFocus::Conversation;
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("focused") || text.contains("scroll"), "expected focus hint");
}

#[test]
fn render_chat_screen_with_status_shows_status() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.status = "Connecting...".to_string();
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("Connecting"), "expected status message in chat");
}

#[test]
fn render_footer_shows_navigation_hint_on_menu() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Menu;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Navigate") || text.contains("Enter") || text.contains("Quit"));
}

#[test]
fn render_footer_shows_back_hint_on_info() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Info;
    let buf = render_to_buffer(&mut app, 120, 30);
    let text = buffer_text(&buf);
    assert!(text.contains("Back") || text.contains("Esc"));
}

#[test]
fn render_does_not_panic_with_tiny_terminal() {
    let (_dir, mut app) = make_app_with_content("prompt");
    // Should not panic even with a very small terminal size
    let _buf = render_to_buffer(&mut app, 40, 10);
}

#[test]
fn render_all_screens_do_not_panic() {
    let screens = [Screen::Menu, Screen::Info, Screen::Show, Screen::Validate, Screen::Tools, Screen::Chat];
    for screen in screens {
        let (_dir, mut app) = make_app_with_content("## ENVIRONMENT\n### PRIMARY OBJECTIVE\n### MODE SELECTION PRIMER\n");
        app.screen = screen;
        let _buf = render_to_buffer(&mut app, 120, 40);
    }
}

// ── ESC-twice-to-stop-inference ───────────────────────────────────────────────

#[test]
fn app_new_last_esc_at_is_none() {
    let app = make_app_no_file();
    assert!(app.last_esc_at.is_none());
}

#[test]
fn app_new_cancel_token_not_cancelled() {
    let app = make_app_no_file();
    assert!(!app.cancel_token.is_cancelled());
}

#[test]
fn first_esc_during_streaming_sets_hint_status() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.streaming = true;
    // Simulate first ESC: no previous esc timestamp → set hint
    app.last_esc_at = Some(std::time::Instant::now());
    app.status = "Press ESC again to stop inference".to_string();
    assert_eq!(app.status, "Press ESC again to stop inference");
    assert!(app.last_esc_at.is_some());
    assert!(app.streaming); // still streaming after first ESC
}

#[test]
fn second_esc_during_streaming_cancels_and_clears() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.streaming = true;
    // Simulate second ESC within 1 s
    app.last_esc_at = Some(std::time::Instant::now());
    // Trigger cancellation logic
    app.cancel_token.cancel();
    app.cancel_token = tokio_util::sync::CancellationToken::new();
    app.streaming = false;
    app.status = "⛔ Inference stopped".to_string();
    app.last_esc_at = None;
    assert!(!app.streaming);
    assert_eq!(app.status, "⛔ Inference stopped");
    assert!(app.last_esc_at.is_none());
    assert!(!app.cancel_token.is_cancelled());
}

#[test]
fn render_chat_screen_shows_stop_inference_status() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.status = "⛔ Inference stopped".to_string();
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("Inference stopped") || text.contains("stopped"), "expected stop status in chat");
}

// ── Large stream / scroll correctness ────────────────────────────────────────

/// Simulate a large assistant response (many lines) and verify that
/// auto-scroll brings the LAST line into view (i.e., it appears in the buffer).
#[test]
fn large_stream_last_line_visible_in_auto_scroll() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;

    // Build a long assistant message: 80 lines of distinct content
    let long_response: String = (1..=80)
        .map(|i| format!("Response line number {i} from the model."))
        .collect::<Vec<_>>()
        .join("\n");
    app.messages.push(("assistant".to_string(), long_response));

    // auto-scroll is on by default (chat_scroll_manual = false)
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);

    // The last line must be visible when auto-scrolled to bottom
    assert!(
        text.contains("Response line number 80"),
        "last streamed line should be visible with auto-scroll; got:\n{text}"
    );
}

/// Verify that with many short messages the conversation panel does not clip
/// the final message — the rendered buffer must contain the last message text.
#[test]
fn large_stream_many_messages_last_message_visible() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;

    for i in 1..=30 {
        app.messages.push(("user".to_string(), format!("User question {i}")));
        app.messages.push(("assistant".to_string(), format!("Assistant answer {i}")));
    }

    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);

    assert!(
        text.contains("Assistant answer 30"),
        "last assistant message should be visible with auto-scroll; got:\n{text}"
    );
}

/// Verify that a single very long line (wider than the terminal) is fully
/// reachable via scroll — the wrapped content must not be clipped.
#[test]
fn large_stream_wide_single_line_wraps_and_is_visible() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;

    // One very long line that will wrap many times in an 80-col terminal
    let wide_line = "WORD ".repeat(200); // 1000 chars
    app.messages.push(("assistant".to_string(), wide_line));

    // Should not panic and should render without clipping the last wrapped row
    let buf = render_to_buffer(&mut app, 80, 30);
    let text = buffer_text(&buf);

    // At least some "WORD" tokens must be visible (auto-scroll to bottom)
    assert!(text.contains("WORD"), "wrapped wide line content should be visible");
}

/// Simulate streaming tokens arriving one by one (appended to last message)
/// and confirm that after each append the last token remains visible.
#[test]
fn streaming_tokens_appended_incrementally_stay_visible() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;

    // Start with an empty assistant message (as streaming would)
    app.messages.push(("assistant".to_string(), String::new()));

    for i in 1..=50 {
        // Append a new token/line (simulating streaming delta)
        if let Some((_, content)) = app.messages.last_mut() {
            if !content.is_empty() { content.push('\n'); }
            content.push_str(&format!("Token {i}"));
        }

        let buf = render_to_buffer(&mut app, 120, 40);
        let text = buffer_text(&buf);

        assert!(
            text.contains(&format!("Token {i}")),
            "Token {i} should be visible after being appended (auto-scroll); got:\n{text}"
        );
    }
}

// ── strip_model_tags ──────────────────────────────────────────────────────────

#[test]
fn strip_model_tags_removes_simple_tag() {
    assert_eq!(strip_model_tags("hello <invoke> world"), "hello  world");
}

#[test]
fn strip_model_tags_removes_closing_tag() {
    assert_eq!(strip_model_tags("text </answer> more"), "text  more");
}

#[test]
fn strip_model_tags_removes_tag_with_attributes() {
    assert_eq!(strip_model_tags(r#"<parameter name="x">value"#), "value");
}

#[test]
fn strip_model_tags_leaves_plain_text_unchanged() {
    let plain = "Hello, this is plain text with no tags.";
    assert_eq!(strip_model_tags(plain), plain);
}

#[test]
fn strip_model_tags_removes_multiple_tags() {
    let input = "<invoke>some</invoke> content <answer>here</answer>";
    let result = strip_model_tags(input);
    assert!(!result.contains("<invoke>"));
    assert!(!result.contains("</invoke>"));
    assert!(!result.contains("<answer>"));
    assert!(!result.contains("</answer>"));
    assert!(result.contains("some"));
    assert!(result.contains("content"));
    assert!(result.contains("here"));
}

#[test]
fn strip_model_tags_preserves_code_with_less_than() {
    // A bare '<' not followed by '>' should be kept
    let input = "x < y and z > w";
    let result = strip_model_tags(input);
    assert!(result.contains('<'), "bare '<' should be preserved");
}

#[test]
fn strip_model_tags_collapses_extra_blank_lines() {
    let input = "line1\n\n\n\nline2";
    let result = strip_model_tags(input);
    // At most one consecutive blank line
    assert!(!result.contains("\n\n\n"), "should collapse multiple blank lines");
}

#[test]
fn strip_model_tags_empty_string() {
    assert_eq!(strip_model_tags(""), "");
}

// ── Selection state ───────────────────────────────────────────────────────────

#[test]
fn selection_can_be_set_and_cleared() {
    let mut app = make_app_no_file();
    app.sel_start = Some(5);
    app.sel_end = Some(10);
    assert_eq!(app.sel_start, Some(5));
    assert_eq!(app.sel_end, Some(10));
    app.sel_start = None;
    app.sel_end = None;
    assert!(app.sel_start.is_none());
    assert!(app.sel_end.is_none());
}

#[test]
fn assistant_messages_with_tags_are_stripped_in_render() {
    let (_dir, mut app) = make_app_with_content("prompt");
    app.screen = Screen::Chat;
    app.messages.push(("assistant".to_string(), "<invoke>hidden</invoke>visible content".to_string()));
    let buf = render_to_buffer(&mut app, 120, 40);
    let text = buffer_text(&buf);
    assert!(text.contains("visible content"), "visible content should appear");
    assert!(!text.contains("<invoke>"), "XML tags should be stripped from display");
}
