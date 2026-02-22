use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use tokio_util::sync::CancellationToken;

use crate::tui::input::TextInput;
use crate::tui::providers::Provider;
use crate::tui::state::ChatFocus;

pub struct ChatState {
    pub chat_focus: ChatFocus,
    pub api_token: String,
    pub custom_url: String,
    pub provider_index: usize,
    pub provider_list_state: ListState,
    pub model_input: String,
    pub ollama_models: Vec<String>,
    pub model_list_state: ListState,
    pub message_input: TextInput,
    pub messages: Vec<(String, String)>,
    pub status: String,
    pub scroll_offset: u16,
    pub streaming: bool,
    pub chat_scroll: u16,
    pub chat_scroll_manual: bool,
    pub api_token_saved: bool,
    pub token_saved_at: Option<std::time::Instant>,
    pub last_esc_at: Option<std::time::Instant>,
    pub cancel_token: CancellationToken,
    pub conv_rect: Rect,
    pub conv_max_scroll: u16,
    pub input_rect: Rect,
    pub input_max_scroll_stored: u16,
    pub sel_start: Option<usize>,
    pub sel_end: Option<usize>,
    pub input_scroll: u16,
    pub input_inner_width: usize,
    pub agent_mode: bool,
}

impl ChatState {
    pub fn new() -> Self {
        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        let mut model_list_state = ListState::default();
        model_list_state.select(Some(0));
        let message_input = TextInput::new();

        Self {
            chat_focus: ChatFocus::ProviderList,
            api_token: String::new(),
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
            agent_mode: false,
        }
    }

    pub fn selected_provider(&self) -> Provider {
        Provider::all()[self.provider_index]
    }

    pub fn active_model(&self) -> String {
        let m = self.model_input.trim();
        if m.is_empty() {
            self.selected_provider().default_model().to_string()
        } else {
            m.to_string()
        }
    }

    pub fn message_input_text(&self) -> String {
        self.message_input.value.clone()
    }
}