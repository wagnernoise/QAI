use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::{CrosstermBackend, TestBackend},
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use arboard::Clipboard;
use std::{io, path::PathBuf, time::Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// ── Simple text input with cursor ───────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize, // byte position
}

impl TextInput {
    pub fn new() -> Self { Self::default() }

    pub fn lines(&self) -> Vec<String> {
        self.value.lines().map(|l| l.to_string()).collect()
    }

    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor == 0 { return; }
        let prev = self.value[..self.cursor]
            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
        self.value.remove(prev);
        self.cursor = prev;
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor >= self.value.len() { return; }
        self.value.remove(self.cursor);
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 { return; }
        self.cursor = self.value[..self.cursor]
            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.value.len() { return; }
        let ch = self.value[self.cursor..].chars().next().unwrap();
        self.cursor += ch.len_utf8();
    }

    pub fn move_home(&mut self) { self.cursor = 0; }
    pub fn move_end(&mut self) { self.cursor = self.value.len(); }

    pub fn clear(&mut self) { self.value.clear(); self.cursor = 0; }

    /// Returns (text_before_cursor, cursor_char_or_space, text_after_cursor)
    pub fn split_at_cursor(&self) -> (&str, &str, &str) {
        let before = &self.value[..self.cursor];
        if self.cursor >= self.value.len() {
            (before, " ", "")
        } else {
            let ch_end = self.cursor + self.value[self.cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            (before, &self.value[self.cursor..ch_end], &self.value[ch_end..])
        }
    }
}

// ── Strip XML-like tags from model output ────────────────────────────────────

/// Remove XML-like tags (e.g. `<invoke>`, `</answer>`, `<parameter name="x">`) from model responses.
pub fn strip_model_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let start = i;
            let next = if i + 1 < chars.len() { chars[i + 1] } else { '\0' };
            // Only treat as a tag if next char is a letter or '/' (closing tag)
            if next.is_ascii_alphabetic() || next == '/' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '>' && chars[j] != '<' {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '>' {
                    // Valid tag — skip it
                    i = j + 1;
                } else {
                    // Not a valid tag, emit '<' literally
                    out.push(chars[start]);
                    i = start + 1;
                }
            } else {
                // Not a tag (e.g. `x < y`), emit literally
                out.push(chars[i]);
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    // Collapse runs of blank lines (max 1 consecutive blank line)
    let mut result = String::new();
    let mut blank_count = 0u32;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim_end().to_string()
}

// ── Model providers ──────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum Provider {
    OpenAI,
    Anthropic,
    XAI,
    Ollama,
    Zen,
    Custom,
}

impl Provider {
    pub fn label(&self) -> &str {
        match self {
            Provider::OpenAI    => "OpenAI (GPT-4o)",
            Provider::Anthropic => "Anthropic (Claude)",
            Provider::XAI       => "xAI (Grok)",
            Provider::Ollama    => "Ollama (local)",
            Provider::Zen       => "Zen API",
            Provider::Custom    => "Custom endpoint",
        }
    }
    pub fn all() -> Vec<Provider> {
        vec![
            Provider::OpenAI,
            Provider::Anthropic,
            Provider::XAI,
            Provider::Ollama,
            Provider::Zen,
            Provider::Custom,
        ]
    }
    pub fn default_model(&self) -> &str {
        match self {
            Provider::OpenAI    => "gpt-4o",
            Provider::Anthropic => "claude-3-5-sonnet-20241022",
            Provider::XAI       => "grok-3",
            Provider::Ollama    => "gemma3",
            Provider::Zen       => "zen-1",
            Provider::Custom    => "custom-model",
        }
    }
    pub fn api_url(&self) -> &str {
        match self {
            Provider::OpenAI    => "https://api.openai.com/v1/chat/completions",
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
            Provider::XAI       => "https://api.x.ai/v1/chat/completions",
            Provider::Ollama    => "http://localhost:11434/v1/chat/completions",
            Provider::Zen       => "https://api.zen.ai/v1/chat/completions",
            Provider::Custom    => "",
        }
    }
    pub fn description(&self) -> &str {
        match self {
            Provider::OpenAI    => "Cloud · Requires API key · https://platform.openai.com/",
            Provider::Anthropic => "Cloud · Requires API key · https://www.anthropic.com/",
            Provider::XAI       => "Cloud · Requires API key · https://x.ai/",
            Provider::Ollama    => "Local · No API key needed · https://ollama.com/",
            Provider::Zen       => "Cloud · Requires API key · https://zen.ai/",
            Provider::Custom    => "Custom OpenAI-compatible endpoint · Enter URL below",
        }
    }
}

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum Screen {
    Menu,
    Info,
    Show,
    Validate,
    Tools,
    Chat,
}

// ── Chat input focus ──────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ChatFocus {
    Token,
    CustomUrl,
    ModelList,
    Message,
    ProviderList,
    Conversation,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub menu_state: ListState,
    pub prompt_path: PathBuf,
    pub prompt_content: String,

    // tools screen state
    pub tools_provider_index: usize,
    pub tools_provider_list_state: ListState,

    // chat state
    pub chat_focus: ChatFocus,
    pub api_token: String,
    pub custom_url: String,
    pub provider_index: usize,
    pub provider_list_state: ListState,
    // model selection
    pub model_input: String,           // typed / selected model name
    pub ollama_models: Vec<String>,    // fetched from Ollama /api/tags
    pub model_list_state: ListState,
    pub message_input: TextInput,
    pub messages: Vec<(String, String)>, // (role, content)
    pub status: String,
    pub scroll_offset: u16,
    pub streaming: bool,  // true while a streaming response is in progress
    pub chat_scroll: u16,         // manual scroll offset for the conversation panel
    pub chat_scroll_manual: bool, // true when user has scrolled up manually
    pub api_token_saved: bool,
    pub token_saved_at: Option<Instant>,
    pub last_esc_at: Option<Instant>,
    pub cancel_token: CancellationToken,
    /// Geometry of the conversation panel — updated every draw, used for scrollbar hit-testing.
    pub conv_rect: Rect,
    /// Last computed max_scroll for the conversation panel — used for scrollbar click hit-testing.
    pub conv_max_scroll: u16,
    /// Mouse selection: start row within conv_rect (terminal row)
    pub sel_start: Option<u16>,
    /// Mouse selection: end row within conv_rect (terminal row)
    pub sel_end: Option<u16>,
}

const MENU_ITEMS: &[&str] = &["Info", "Show Prompt", "Validate", "Tools", "Chat", "Quit"];

impl App {
    pub fn new(prompt_path: PathBuf) -> Self {
        let prompt_content = std::fs::read_to_string(&prompt_path)
            .unwrap_or_else(|_| "(prompt file not found)".to_string());
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));
        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        let mut tools_provider_list_state = ListState::default();
        tools_provider_list_state.select(Some(0));
        let mut model_list_state = ListState::default();
        model_list_state.select(Some(0));
        // Load saved API token from config file if present
        let saved_token = load_api_token().unwrap_or_default();
        let message_input = TextInput::new();
        App {
            screen: Screen::Menu,
            menu_state,
            prompt_path,
            prompt_content,
            tools_provider_index: 0,
            tools_provider_list_state,
            chat_focus: ChatFocus::ProviderList,
            api_token: saved_token,
            custom_url: String::new(),
            provider_index: 0,
            provider_list_state,
            model_input: String::new(),
            ollama_models: Vec::new(),
            model_list_state,
            message_input,
            messages: Vec::new(),
            status: String::new(),
            scroll_offset: 0,
            streaming: false,
            chat_scroll: 0,
            chat_scroll_manual: false,
            api_token_saved: false,
            token_saved_at: None,
            last_esc_at: None,
            cancel_token: CancellationToken::new(),
            conv_rect: Rect::default(),
            conv_max_scroll: 0,
            sel_start: None,
            sel_end: None,
        }
    }

    /// Returns the current message input text as a single string.
    pub fn message_input_text(&self) -> String {
        self.message_input.value.clone()
    }

    pub fn selected_provider(&self) -> Provider {
        Provider::all().remove(self.provider_index)
    }

    /// Returns the model name to use: typed override, or provider default.
    pub fn active_model(&self) -> String {
        let m = self.model_input.trim();
        if m.is_empty() {
            self.selected_provider().default_model().to_string()
        } else {
            m.to_string()
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(prompt_path: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(prompt_path);
    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut event_stream = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
    // Channel for streaming token chunks: sender given to spawn, receiver polled here
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<Option<String>>();

    loop {
        terminal.draw(|f| draw(f, app))?;

        tokio::select! {
            // 50 ms tick — redraws and clears timed status messages
            _ = tick.tick() => {
                if let Some(saved_at) = app.token_saved_at {
                    if saved_at.elapsed() >= std::time::Duration::from_secs(3) {
                        if app.status == "✓ API token saved" {
                            app.status = String::new();
                        }
                        app.token_saved_at = None;
                    }
                }
            }

            // Incoming streaming token chunk
            Some(chunk) = stream_rx.recv() => {
                match chunk {
                    Some(token) => {
                        // Append token to last assistant message
                        if let Some((role, content)) = app.messages.last_mut() {
                            if role == "assistant" {
                                content.push_str(&token);
                            } else {
                                app.messages.push(("assistant".to_string(), token));
                            }
                        } else {
                            app.messages.push(("assistant".to_string(), token));
                        }
                        app.status = String::new();
                    }
                    None => {
                        // Stream finished
                        app.streaming = false;
                        app.status = String::new();
                    }
                }
            }

            // Keyboard / terminal events
            Some(Ok(event)) = event_stream.next() => {
                // Trackpad / mouse scroll — works on any screen
                if let Event::Mouse(mouse) = &event {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if app.screen == Screen::Chat {
                                app.chat_scroll = app.chat_scroll.saturating_sub(3);
                                app.chat_scroll_manual = true;
                            } else if app.screen == Screen::Show {
                                app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.screen == Screen::Chat {
                                app.chat_scroll = app.chat_scroll.saturating_add(3);
                                app.chat_scroll_manual = true;
                            } else if app.screen == Screen::Show {
                                app.scroll_offset = app.scroll_offset.saturating_add(3);
                            }
                        }
                        // Scrollbar click or drag — hit-test against the right edge of conv_rect
                        // Also track mouse selection inside the conversation area
                        MouseEventKind::Down(_) => {
                            if app.screen == Screen::Chat {
                                let r = app.conv_rect;
                                let scrollbar_col = r.x + r.width.saturating_sub(1);
                                if mouse.column == scrollbar_col && r.height > 2 {
                                    let track_top = r.y + 1;
                                    let track_bottom = r.y + r.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 && mouse.row >= track_top && mouse.row <= track_bottom {
                                        let ratio = (mouse.row - track_top) as f32 / track_len as f32;
                                        let new_scroll = (ratio * app.conv_max_scroll as f32).round() as u16;
                                        app.chat_scroll = new_scroll;
                                        app.chat_scroll_manual = true;
                                    }
                                } else if mouse.column >= r.x && mouse.column < r.x + r.width
                                    && mouse.row >= r.y && mouse.row < r.y + r.height {
                                    // Start a new text selection
                                    app.sel_start = Some(mouse.row);
                                    app.sel_end = Some(mouse.row);
                                }
                            }
                        }
                        MouseEventKind::Drag(_) => {
                            if app.screen == Screen::Chat {
                                let r = app.conv_rect;
                                let scrollbar_col = r.x + r.width.saturating_sub(1);
                                if mouse.column == scrollbar_col && r.height > 2 {
                                    let track_top = r.y + 1;
                                    let track_bottom = r.y + r.height.saturating_sub(2);
                                    let track_len = track_bottom.saturating_sub(track_top) as usize;
                                    if track_len > 0 && mouse.row >= track_top && mouse.row <= track_bottom {
                                        let ratio = (mouse.row - track_top) as f32 / track_len as f32;
                                        let new_scroll = (ratio * app.conv_max_scroll as f32).round() as u16;
                                        app.chat_scroll = new_scroll;
                                        app.chat_scroll_manual = true;
                                    }
                                } else if app.sel_start.is_some()
                                    && mouse.column >= r.x && mouse.column < r.x + r.width
                                    && mouse.row >= r.y && mouse.row < r.y + r.height {
                                    // Extend selection
                                    app.sel_end = Some(mouse.row);
                                }
                            }
                        }
                        MouseEventKind::Up(_) => {
                            // Selection finalized on mouse-up; do NOT auto-copy.
                            // User copies explicitly with Ctrl+C / Cmd+C.
                        }
                        _ => {}
                    }
                }
                if let Event::Key(key) = event {
                    match &app.screen {
                        Screen::Menu => match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                            KeyCode::Up => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                app.menu_state.select(Some(i.saturating_sub(1)));
                            }
                            KeyCode::Down => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                app.menu_state.select(Some((i + 1).min(MENU_ITEMS.len() - 1)));
                            }
                            KeyCode::Enter => {
                                let i = app.menu_state.selected().unwrap_or(0);
                                match i {
                                    0 => app.screen = Screen::Info,
                                    1 => { app.scroll_offset = 0; app.screen = Screen::Show; }
                                    2 => app.screen = Screen::Validate,
                                    3 => app.screen = Screen::Tools,
                                    4 => app.screen = Screen::Chat,
                                    5 => return Ok(()),
                                    _ => {}
                                }
                            }
                            _ => {}
                        },
                        Screen::Info | Screen::Validate => {
                            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                                app.screen = Screen::Menu;
                            }
                        }
                        Screen::Tools => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Menu,
                            KeyCode::Up | KeyCode::Char('k') => {
                                let i = app.tools_provider_index.saturating_sub(1);
                                app.tools_provider_index = i;
                                app.tools_provider_list_state.select(Some(i));
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let i = (app.tools_provider_index + 1).min(Provider::all().len() - 1);
                                app.tools_provider_index = i;
                                app.tools_provider_list_state.select(Some(i));
                            }
                            KeyCode::Enter => {
                                app.provider_index = app.tools_provider_index;
                                app.provider_list_state.select(Some(app.tools_provider_index));
                                app.screen = Screen::Chat;
                            }
                            _ => {}
                        },
                        Screen::Show => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Menu,
                            KeyCode::Down | KeyCode::Char('j') => app.scroll_offset += 1,
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.scroll_offset = app.scroll_offset.saturating_sub(1)
                            }
                            _ => {}
                        },
                        Screen::Chat => {
                            handle_chat_key(app, key, stream_tx.clone()).await?;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_chat_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    stream_tx: mpsc::UnboundedSender<Option<String>>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            if app.streaming {
                if let Some(t) = app.last_esc_at {
                    if t.elapsed() <= std::time::Duration::from_secs(1) {
                        // Second ESC within 1 s — cancel inference
                        app.cancel_token.cancel();
                        app.cancel_token = CancellationToken::new();
                        app.streaming = false;
                        app.status = "⛔ Inference stopped".to_string();
                        app.last_esc_at = None;
                        return Ok(());
                    }
                }
                app.last_esc_at = Some(Instant::now());
                app.status = "Press ESC again to stop inference".to_string();
            } else {
                app.screen = Screen::Menu;
            }
        }
        KeyCode::Tab => {
            let is_custom = app.selected_provider() == Provider::Custom;
            let is_ollama = app.selected_provider() == Provider::Ollama;
            app.chat_focus = match app.chat_focus {
                ChatFocus::ProviderList => {
                    if is_ollama {
                        // fetch models when entering model list
                        if app.ollama_models.is_empty() {
                            fetch_ollama_models(app).await;
                        }
                        ChatFocus::ModelList
                    } else if is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::Token
                    }
                }
                ChatFocus::ModelList => ChatFocus::Token,
                ChatFocus::Token => ChatFocus::Message,
                ChatFocus::CustomUrl => ChatFocus::Message,
                ChatFocus::Message => ChatFocus::Conversation,
                ChatFocus::Conversation => ChatFocus::ProviderList,
            };
        }
        KeyCode::BackTab => {
            let is_custom = app.selected_provider() == Provider::Custom;
            let is_ollama = app.selected_provider() == Provider::Ollama;
            app.chat_focus = match app.chat_focus {
                ChatFocus::ProviderList => ChatFocus::Conversation,
                ChatFocus::ModelList => ChatFocus::ProviderList,
                ChatFocus::Token => {
                    if is_ollama {
                        ChatFocus::ModelList
                    } else {
                        ChatFocus::ProviderList
                    }
                }
                ChatFocus::CustomUrl => ChatFocus::ProviderList,
                ChatFocus::Conversation => ChatFocus::Message,
                ChatFocus::Message => {
                    if is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::Token
                    }
                }
            };
        }
        // Conversation scroll: PageUp / PageDown always, Alt+↑/↓ always
        KeyCode::PageUp => {
            app.chat_scroll = app.chat_scroll.saturating_sub(5);
            app.chat_scroll_manual = true;
        }
        KeyCode::PageDown => {
            app.chat_scroll = app.chat_scroll.saturating_add(5);
            // if user scrolled back to bottom, disable manual mode
            // (exact bottom check happens in draw_chat; here just keep manual=true)
            app.chat_scroll_manual = true;
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            app.chat_scroll = app.chat_scroll.saturating_sub(1);
            app.chat_scroll_manual = true;
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            app.chat_scroll = app.chat_scroll.saturating_add(1);
            app.chat_scroll_manual = true;
        }
        KeyCode::End => {
            // Jump to bottom and re-enable auto-scroll
            app.chat_scroll_manual = false;
        }
        KeyCode::Up => match app.chat_focus {
            ChatFocus::Conversation => {
                app.chat_scroll = app.chat_scroll.saturating_sub(3);
                app.chat_scroll_manual = true;
            }
            ChatFocus::ProviderList => {
                let i = app.provider_index.saturating_sub(1);
                app.provider_index = i;
                app.provider_list_state.select(Some(i));
                // reset model list when provider changes
                app.ollama_models.clear();
                app.model_input.clear();
                app.model_list_state.select(Some(0));
            }
            ChatFocus::ModelList => {
                let max = app.ollama_models.len().saturating_sub(1);
                let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                let i = i.min(max);
                app.model_list_state.select(Some(i));
                if let Some(m) = app.ollama_models.get(i) {
                    app.model_input = m.clone();
                }
            }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key); }
            _ => {}
        },
        KeyCode::Down => match app.chat_focus {
            ChatFocus::Conversation => {
                app.chat_scroll = app.chat_scroll.saturating_add(3);
                app.chat_scroll_manual = true;
            }
            ChatFocus::ProviderList => {
                let i = (app.provider_index + 1).min(Provider::all().len() - 1);
                app.provider_index = i;
                app.provider_list_state.select(Some(i));
                // reset model list when provider changes
                app.ollama_models.clear();
                app.model_input.clear();
                app.model_list_state.select(Some(0));
            }
            ChatFocus::ModelList => {
                let max = app.ollama_models.len().saturating_sub(1);
                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(max);
                app.model_list_state.select(Some(i));
                if let Some(m) = app.ollama_models.get(i) {
                    app.model_input = m.clone();
                }
            }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key); }
            _ => {}
        },
        KeyCode::Enter => match app.chat_focus {
            ChatFocus::ProviderList => {
                // confirm provider; if Ollama fetch models
                if app.selected_provider() == Provider::Ollama {
                    fetch_ollama_models(app).await;
                    app.chat_focus = ChatFocus::ModelList;
                } else {
                    app.chat_focus = ChatFocus::Message;
                }
            }
            ChatFocus::ModelList => {
                // confirm model selection, move to message
                app.chat_focus = ChatFocus::Message;
            }
            ChatFocus::Message => {
                if app.streaming { return Ok(()); }
                let msg = app.message_input_text();
                let msg = msg.trim().to_string();
                if !msg.is_empty() {
                    app.messages.push(("user".to_string(), msg.clone()));
                    app.message_input = TextInput::new();
                    app.status = "Streaming…".to_string();
                    app.streaming = true;
                    app.chat_scroll_manual = false;
                    app.chat_scroll = 0;
                    // Spawn streaming task; sends tokens via channel
                    let provider = app.selected_provider();
                    let token = app.api_token.clone();
                    let custom_url = app.custom_url.clone();
                    let model = app.active_model();
                    let system_prompt = app.prompt_content.clone();
                    let history: Vec<(String, String)> = app.messages.clone();
                    let tx = stream_tx.clone();
                    let cancel = app.cancel_token.clone();
                    tokio::spawn(async move {
                        if let Err(e) = stream_message(
                            provider, token, custom_url, model, system_prompt, history, tx.clone(), cancel,
                        ).await {
                            let _ = tx.send(Some(format!("\n[Error: {e}]")));
                            let _ = tx.send(None);
                        }
                    });
                }
            }
            _ => {}
        },
        KeyCode::Backspace => match app.chat_focus {
            ChatFocus::Token => { app.api_token.pop(); }
            ChatFocus::CustomUrl => { app.custom_url.pop(); }
            ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key); }
            _ => {}
        },
        KeyCode::Char(c) => {
            let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c';
            let cmd_c  = key.modifiers.contains(KeyModifiers::SUPER)   && c == 'c';
            if ctrl_c || cmd_c {
                // If there is an active selection, copy it; otherwise go back to menu (Ctrl+C only)
                if let (Some(start_row), Some(end_row)) = (app.sel_start, app.sel_end) {
                    let r = app.conv_rect;
                    let inner_top = r.y + 1;
                    let effective_scroll = if app.chat_scroll_manual {
                        app.chat_scroll.min(app.conv_max_scroll)
                    } else {
                        app.conv_max_scroll
                    };
                    let conv_inner_width = r.width.saturating_sub(3) as usize;
                    let mut rendered: Vec<String> = Vec::new();
                    for (role, content) in &app.messages {
                        let label = if role == "user" { "You" } else { "QA-Bot" };
                        rendered.push(format!(" {label}: "));
                        let display = if role == "assistant" {
                            std::borrow::Cow::Owned(strip_model_tags(content))
                        } else {
                            std::borrow::Cow::Borrowed(content.as_str())
                        };
                        for line in display.lines() {
                            let text = format!("   {line}");
                            if conv_inner_width > 0 && text.chars().count() > conv_inner_width {
                                let chars: Vec<char> = text.chars().collect();
                                for chunk in chars.chunks(conv_inner_width) {
                                    rendered.push(chunk.iter().collect());
                                }
                            } else {
                                rendered.push(text);
                            }
                        }
                        rendered.push(String::new());
                    }
                    let (r0, r1) = if start_row <= end_row { (start_row, end_row) } else { (end_row, start_row) };
                    let first_line = (r0.saturating_sub(inner_top) as usize).saturating_add(effective_scroll as usize);
                    let last_line  = (r1.saturating_sub(inner_top) as usize).saturating_add(effective_scroll as usize);
                    let selected: Vec<&str> = rendered.iter().enumerate()
                        .filter(|(i, _)| *i >= first_line && *i <= last_line)
                        .map(|(_, l)| l.as_str())
                        .collect();
                    if !selected.is_empty() {
                        let text = selected.join("\n");
                        if let Ok(mut cb) = Clipboard::new() {
                            let _ = cb.set_text(text);
                            app.status = "📋 Copied to clipboard".to_string();
                        }
                    }
                    app.sel_start = None;
                    app.sel_end = None;
                } else if ctrl_c {
                    app.screen = Screen::Menu;
                    return Ok(());
                }
                return Ok(());
            }
            match app.chat_focus {
                ChatFocus::Token => {
                    app.api_token.push(c);
                    // Save token on every keystroke
                    let _ = save_api_token(&app.api_token);
                    app.api_token_saved = true;
                    app.token_saved_at = Some(Instant::now());
                    app.status = "✓ API token saved".to_string();
                }
                ChatFocus::CustomUrl => app.custom_url.push(c),
                ChatFocus::Message => { handle_text_input_key(&mut app.message_input, key); }
                _ => {}
            }
        }
        _ => {
            // Forward any other key events to the message TextInput when focused
            if app.chat_focus == ChatFocus::Message {
                handle_text_input_key(&mut app.message_input, key);
            }
        }
    }
    Ok(())
}

// ── TextInput key handler ─────────────────────────────────────────────────────

fn handle_text_input_key(input: &mut TextInput, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Char(c) => input.insert_char(c),
        KeyCode::Backspace => input.delete_char_before(),
        KeyCode::Delete => input.delete_char_after(),
        KeyCode::Left => input.move_left(),
        KeyCode::Right => input.move_right(),
        KeyCode::Home => input.move_home(),
        KeyCode::End => input.move_end(),
        _ => {}
    }
}

// ── Ollama model fetcher ──────────────────────────────────────────────────────

async fn fetch_ollama_models(app: &mut App) {
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

async fn stream_message(
    provider: Provider,
    api_token: String,
    custom_url: String,
    model: String,
    system_prompt: String,
    history: Vec<(String, String)>,
    tx: mpsc::UnboundedSender<Option<String>>,
    cancel: CancellationToken,
) -> Result<()> {
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

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(15, 15, 25))),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_header(f, chunks[0]);

    match app.screen {
        Screen::Menu => draw_menu(f, chunks[1], app),
        Screen::Info => draw_info(f, chunks[1], app),
        Screen::Show => draw_show(f, chunks[1], app),
        Screen::Validate => draw_validate(f, chunks[1], app),
        Screen::Tools => draw_tools(f, chunks[1], app),
        Screen::Chat => draw_chat(f, chunks[1], app),
    }

    draw_footer(f, chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let banner = vec![
        Line::from(vec![
            Span::styled("  ██████╗  █████╗ ██╗", Style::default().fg(Color::Cyan)),
            Span::styled("  ", Style::default()),
            Span::styled("QA Automation AI Agent", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(" ██╔═══██╗██╔══██╗██║", Style::default().fg(Color::Cyan)),
            Span::styled("  v", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(" ██║   ██║███████║██║", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" ██║▄▄ ██║██╔══██║██║", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" ╚██████╔╝██║  ██║██║", Style::default().fg(Color::Cyan))),
    ];

    let header = Paragraph::new(banner)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .alignment(Alignment::Left);
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let hint = match &app.screen {
        Screen::Menu => " ↑↓ Navigate   Enter Select   q Quit ",
        Screen::Show => " ↑↓/j/k Scroll   q/Esc Back ",
        Screen::Chat => " Tab Next field   Enter Send   Esc Back ",
        _ => " q/Esc Back ",
    };
    let footer = Paragraph::new(hint)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(15, 15, 25)))
        .alignment(Alignment::Center);
    f.render_widget(footer, area);
}

fn draw_menu(f: &mut Frame, area: Rect, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let icon = match i {
                0 => "  ",
                1 => "  ",
                2 => "  ",
                3 => "  ",
                4 => "  ",
                5 => "  ",
                _ => "  ",
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, Style::default().fg(Color::Cyan)),
                Span::raw(*label),
            ]))
        })
        .collect();

    let mut state = app.menu_state.clone();
    let list = List::new(items)
        .block(
            Block::default()
                .title(" Menu ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, outer[0], &mut state);

    // Right panel: welcome text
    let welcome = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Welcome to QAI",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  QA Automation AI Agent manager.",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Use the menu to inspect, validate,",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  or chat with the QA-Bot via API.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
    )
    .wrap(Wrap { trim: false });
    f.render_widget(welcome, outer[1]);
}

fn draw_info(f: &mut Frame, area: Rect, app: &App) {
    let exists = app.prompt_path.exists();
    let size = std::fs::metadata(&app.prompt_path)
        .map(|m| format!("{} bytes", m.len()))
        .unwrap_or_else(|_| "N/A".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Prompt path : ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.prompt_path.display().to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Exists      : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if exists { "yes" } else { "no" },
                Style::default().fg(if exists { Color::Green } else { Color::Red }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size        : ", Style::default().fg(Color::DarkGray)),
            Span::styled(size, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Version     : ", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  README      : ", Style::default().fg(Color::DarkGray)),
            Span::styled("README.md", Style::default().fg(Color::White)),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Info ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_show(f: &mut Frame, area: Rect, app: &App) {
    let p = Paragraph::new(app.prompt_content.as_str())
        .block(
            Block::default()
                .title(" System Prompt ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    f.render_widget(p, area);
}

fn draw_validate(f: &mut Frame, area: Rect, app: &App) {
    let required = ["## ENVIRONMENT", "### PRIMARY OBJECTIVE", "### MODE SELECTION PRIMER"];
    let mut lines = vec![Line::from("")];
    let mut all_ok = true;
    for marker in required {
        let found = app.prompt_content.contains(marker);
        if !found { all_ok = false; }
        lines.push(Line::from(vec![
            Span::styled(
                if found { "  ✔ " } else { "  ✘ " },
                Style::default().fg(if found { Color::Green } else { Color::Red }),
            ),
            Span::raw(marker),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        if all_ok { "  Validation passed." } else { "  Validation failed." },
        Style::default()
            .fg(if all_ok { Color::Green } else { Color::Red })
            .add_modifier(Modifier::BOLD),
    )));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Validate ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_tools(f: &mut Frame, area: Rect, app: &mut App) {
    let providers = Provider::all();

    // Layout: left list | right detail
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // ── Left: provider list ───────────────────────────────────────────────────
    let items: Vec<ListItem> = providers
        .iter()
        .map(|p| ListItem::new(p.label()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title(" AI Providers ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, cols[0], &mut app.tools_provider_list_state);

    // ── Right: detail panel ───────────────────────────────────────────────────
    let selected = &providers[app.tools_provider_index];
    let detail_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", selected.label()),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Info       : ", Style::default().fg(Color::DarkGray)),
            Span::styled(selected.description(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  API URL    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected.api_url().is_empty() { "(enter custom URL in Chat screen)" } else { selected.api_url() },
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Model      : ", Style::default().fg(Color::DarkGray)),
            Span::styled(selected.default_model(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to open Chat with this provider selected.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ↑/↓ or j/k: navigate   Enter: open Chat   q/Esc: back",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .title(" Provider Details ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(detail, cols[1]);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &mut App) {
    let providers = Provider::all();
    let is_custom = app.selected_provider() == Provider::Custom;
    let is_ollama = app.selected_provider() == Provider::Ollama;

    // Layout: left sidebar (config) | right (conversation)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // ── Left: config panel ────────────────────────────────────────────────────
    // Rows: provider list | model list (Ollama only) | custom url (Custom only) | token | model display
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),                                      // provider list
            Constraint::Length(if is_ollama { 6 } else { 0 }),       // model list (Ollama)
            Constraint::Length(if is_custom { 3 } else { 0 }),       // custom url
            Constraint::Length(3),                                   // token
            Constraint::Length(3),                                   // active model display
        ])
        .split(cols[0]);

    // Provider list
    let provider_focused = app.chat_focus == ChatFocus::ProviderList;
    let provider_items: Vec<ListItem> = providers
        .iter()
        .map(|p| ListItem::new(p.label()))
        .collect();
    let mut pstate = app.provider_list_state.clone();
    let provider_list = List::new(provider_items)
        .block(
            Block::default()
                .title(" Provider (↑/↓ Enter) ")
                .title_style(Style::default().fg(if provider_focused { Color::Yellow } else { Color::DarkGray }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if provider_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(provider_list, left_rows[0], &mut pstate);

    // Ollama model list
    if is_ollama {
        let model_focused = app.chat_focus == ChatFocus::ModelList;
        if app.ollama_models.is_empty() {
            let hint = Paragraph::new(Span::styled(
                " Press Enter or Tab to fetch models",
                Style::default().fg(Color::DarkGray),
            ))
            .block(
                Block::default()
                    .title(" Model (↑/↓ Enter) ")
                    .title_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::DarkGray }))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
            );
            f.render_widget(hint, left_rows[1]);
        } else {
            let model_items: Vec<ListItem> = app
                .ollama_models
                .iter()
                .map(|m| ListItem::new(m.as_str()))
                .collect();
            let mut mstate = app.model_list_state.clone();
            let model_list = List::new(model_items)
                .block(
                    Block::default()
                        .title(" Model (↑/↓ Enter) ")
                        .title_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::DarkGray }))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(if model_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
                )
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(model_list, left_rows[1], &mut mstate);
        }
    }

    // Custom URL field
    if is_custom {
        let url_focused = app.chat_focus == ChatFocus::CustomUrl;
        let url_block = Block::default()
            .title(" Endpoint URL ")
            .title_style(Style::default().fg(if url_focused { Color::Yellow } else { Color::DarkGray }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if url_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) }));
        let url_p = Paragraph::new(app.custom_url.as_str())
            .block(url_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(url_p, left_rows[2]);
    }

    // Token field
    let token_display: String = if app.api_token.is_empty() {
        String::new()
    } else {
        "•".repeat(app.api_token.len().min(20))
    };
    let token_focused = app.chat_focus == ChatFocus::Token;
    let token_title = if is_ollama { " API Token (optional) " } else { " API Token " };
    let token_block = Block::default()
        .title(token_title)
        .title_style(Style::default().fg(if token_focused { Color::Yellow } else { Color::DarkGray }))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if token_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) }));
    let token_p = Paragraph::new(token_display.as_str())
        .block(token_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(token_p, left_rows[3]);

    // Active model display
    let active_model = app.active_model();
    let model_display = Paragraph::new(Span::styled(
        format!(" {active_model}"),
        Style::default().fg(Color::Green),
    ))
    .block(
        Block::default()
            .title(" Active Model ")
            .title_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(50, 50, 80))),
    );
    f.render_widget(model_display, left_rows[4]);

    // ── Right: conversation + input ───────────────────────────────────────────
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3), Constraint::Length(1)])
        .split(cols[1]);

    // Conversation history
    let mut conv_lines: Vec<Line> = Vec::new();
    for (role, content) in &app.messages {
        let (label, color) = if role == "user" {
            ("You", Color::Cyan)
        } else {
            ("QA-Bot", Color::Green)
        };
        conv_lines.push(Line::from(Span::styled(
            format!(" {label}: "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        let display_content = if role == "assistant" {
            std::borrow::Cow::Owned(strip_model_tags(content))
        } else {
            std::borrow::Cow::Borrowed(content.as_str())
        };
        for line in display_content.lines() {
            conv_lines.push(Line::from(Span::styled(
                format!("   {line}"),
                Style::default().fg(Color::White),
            )));
        }
        conv_lines.push(Line::from(""));
    }
    if !app.status.is_empty() {
        conv_lines.push(Line::from(Span::styled(
            format!(" {}", app.status),
            Style::default().fg(Color::Yellow),
        )));
    }

    // Scroll logic: manual overrides auto-scroll to bottom
    let conv_area_height = right_rows[0].height.saturating_sub(2) as usize; // subtract borders
    // The available width for text inside the bordered Paragraph (subtract 2 for borders, 1 for scrollbar)
    let conv_inner_width = right_rows[0].width.saturating_sub(3) as usize;
    // Count rendered lines accounting for word-wrap: each Line whose text width exceeds
    // conv_inner_width wraps into ceil(width / inner_width) rendered rows.
    let total_lines: usize = conv_lines.iter().map(|line| {
        let text_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if conv_inner_width == 0 || text_width == 0 {
            1
        } else {
            text_width.div_ceil(conv_inner_width)
        }
    }).sum::<usize>().max(1);
    let max_scroll = if total_lines > conv_area_height {
        (total_lines - conv_area_height) as u16
    } else {
        0
    };
    let effective_scroll = if app.chat_scroll_manual {
        app.chat_scroll.min(max_scroll)
    } else {
        max_scroll
    };

    // Apply selection highlight to visible lines
    let conv_inner_top = right_rows[0].y + 1;
    if let (Some(s), Some(e)) = (app.sel_start, app.sel_end) {
        let (row_min, row_max) = if s <= e { (s, e) } else { (e, s) };
        let effective_scroll_for_sel = if app.chat_scroll_manual {
            app.chat_scroll.min(max_scroll)
        } else {
            max_scroll
        };
        // Map terminal rows to logical line indices
        let first_sel = (row_min.saturating_sub(conv_inner_top) as usize)
            .saturating_add(effective_scroll_for_sel as usize);
        let last_sel = (row_max.saturating_sub(conv_inner_top) as usize)
            .saturating_add(effective_scroll_for_sel as usize);
        // Walk rendered lines (accounting for wrap) to find which logical indices to highlight
        let mut rendered_idx: usize = 0;
        for line in conv_lines.iter_mut() {
            let text_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            let wrapped = if conv_inner_width == 0 || text_width == 0 { 1 } else { text_width.div_ceil(conv_inner_width) };
            for _ in 0..wrapped {
                if rendered_idx >= first_sel && rendered_idx <= last_sel {
                    // Highlight this line
                    for span in line.spans.iter_mut() {
                        span.style = span.style.bg(Color::Rgb(60, 80, 120));
                    }
                }
                rendered_idx += 1;
            }
        }
    }

    let conv_focused = app.chat_focus == ChatFocus::Conversation;
    let conv_title = if app.chat_scroll_manual {
        " Conversation  [↑/↓ scroll — End to resume auto-scroll] "
    } else if conv_focused {
        " Conversation  [focused — ↑/↓ to scroll] "
    } else {
        " Conversation  [Tab to focus] "
    };

    let conv = Paragraph::new(conv_lines)
        .block(
            Block::default()
                .title(conv_title)
                .title_style(Style::default().fg(if conv_focused { Color::Yellow } else { Color::Cyan }).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if conv_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        )
        .wrap(Wrap { trim: false })
        .scroll((effective_scroll, 0));
    app.conv_rect = right_rows[0];
    app.conv_max_scroll = max_scroll;
    f.render_widget(conv, right_rows[0]);

    // Scrollbar
    if total_lines > conv_area_height {
        let mut scrollbar_state = ScrollbarState::new(max_scroll as usize)
            .position(effective_scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        f.render_stateful_widget(scrollbar, right_rows[0], &mut scrollbar_state);
    }

    // Message input with visible cursor
    let msg_focused = app.chat_focus == ChatFocus::Message;
    let (before, cursor_ch, after) = app.message_input.split_at_cursor();
    let cursor_style = if msg_focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input_line = Line::from(vec![
        Span::styled(before.to_string(), Style::default().fg(Color::White)),
        Span::styled(cursor_ch.to_string(), cursor_style),
        Span::styled(after.to_string(), Style::default().fg(Color::White)),
    ]);
    let input_widget = Paragraph::new(input_line)
        .block(
            Block::default()
                .title(" Message (Enter to send) ")
                .title_style(Style::default().fg(if msg_focused { Color::Yellow } else { Color::DarkGray }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if msg_focused { Color::Yellow } else { Color::Rgb(50, 50, 80) })),
        );
    f.render_widget(input_widget, right_rows[1]);

    // Cursor hint
    let hint = Paragraph::new(Span::styled(
        " Tab: cycle focus   ↑/↓: scroll (Conversation) or navigate   Enter: select/send   End: auto-scroll   Esc: menu ",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(hint, right_rows[2]);
}

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Render the current app state into an in-memory buffer using `TestBackend`.
/// Useful for unit tests that need to assert on rendered output without a real terminal.
pub fn render_to_buffer(app: &mut App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    terminal.draw(|f| draw(f, app)).expect("draw");
    terminal.backend().buffer().clone()
}

