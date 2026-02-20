use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::tui::input::TextInput;
use crate::tui::providers::Provider;
use crate::tui::api::load_api_token;

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
    /// Geometry of the message input panel — updated every draw, used for input scrollbar hit-testing.
    pub input_rect: Rect,
    /// Last computed max_scroll for the message input panel — used for input scrollbar click hit-testing.
    pub input_max_scroll_stored: u16,
    /// Mouse selection: start content line index (scroll-independent)
    pub sel_start: Option<usize>,
    /// Mouse selection: end content line index (scroll-independent)
    pub sel_end: Option<usize>,
    /// Vertical scroll offset for the message input box (cursor-line tracking)
    pub input_scroll: u16,
    /// Inner width of the message input box — updated every draw, used for cursor navigation.
    pub input_inner_width: usize,
}

pub const MENU_ITEMS: &[&str] = &["Info", "Show Prompt", "Validate", "Tools", "Chat", "Quit"];

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
            input_rect: Rect::default(),
            input_max_scroll_stored: 0,
            sel_start: None,
            sel_end: None,
            input_scroll: 0,
            input_inner_width: 60,
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

