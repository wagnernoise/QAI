// State management module
// Handles all state transitions and business logic

use crate::tui::state::{App, Screen, ChatFocus};
use crate::tui::providers::Provider;

pub struct StateManager {
    app: App,
}

impl StateManager {
    pub fn new(app: App) -> Self {
        Self { app }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    // Screen navigation methods
    pub fn navigate_to_menu(&mut self) {
        self.app.screen = Screen::Menu;
    }

    pub fn navigate_to_chat(&mut self) {
        self.app.screen = Screen::Chat;
    }

    pub fn navigate_to_info(&mut self) {
        self.app.screen = Screen::Info;
    }

    pub fn navigate_to_show(&mut self) {
        self.app.scroll_offset = 0;
        self.app.screen = Screen::Show;
    }

    pub fn navigate_to_validate(&mut self) {
        self.app.screen = Screen::Validate;
    }

    pub fn navigate_to_tools(&mut self) {
        self.app.screen = Screen::Tools;
    }

    // Chat focus management
    pub fn cycle_chat_focus(&mut self, forward: bool) {
        let is_custom = self.app.selected_provider() == Provider::Custom;
        let is_ollama = self.app.selected_provider() == Provider::Ollama;

        if forward {
            self.app.chat_focus = match self.app.chat_focus {
                ChatFocus::ProviderList => {
                    if is_ollama {
                        ChatFocus::ModelList
                    } else if is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::Token
                    }
                }
                ChatFocus::ModelList => ChatFocus::CustomUrl,
                ChatFocus::Token => ChatFocus::Message,
                ChatFocus::CustomUrl => ChatFocus::Token,
                ChatFocus::Message => ChatFocus::Conversation,
                ChatFocus::Conversation => ChatFocus::ProviderList,
            };
        } else {
            self.app.chat_focus = match self.app.chat_focus {
                ChatFocus::ProviderList => ChatFocus::Conversation,
                ChatFocus::ModelList => ChatFocus::ProviderList,
                ChatFocus::Token => {
                    if is_ollama || is_custom {
                        ChatFocus::CustomUrl
                    } else {
                        ChatFocus::ProviderList
                    }
                }
                ChatFocus::CustomUrl => {
                    if is_ollama {
                        ChatFocus::ModelList
                    } else {
                        ChatFocus::ProviderList
                    }
                }
                ChatFocus::Conversation => ChatFocus::Message,
                ChatFocus::Message => ChatFocus::Token,
            };
        }
    }

    // Provider management
    pub fn select_previous_provider(&mut self) {
        let i = self.app.provider_index.saturating_sub(1);
        self.app.provider_index = i;
        self.app.provider_list_state.select(Some(i));
        // Reset model list when provider changes
        self.app.ollama_models.clear();
        self.app.model_input.clear();
        self.app.model_list_state.select(Some(0));
    }

    pub fn select_next_provider(&mut self) {
        let i = (self.app.provider_index + 1).min(Provider::all().len() - 1);
        self.app.provider_index = i;
        self.app.provider_list_state.select(Some(i));
        // Reset model list when provider changes
        self.app.ollama_models.clear();
        self.app.model_input.clear();
        self.app.model_list_state.select(Some(0));
    }

    // Scroll management
    pub fn scroll_up(&mut self) {
        if self.app.screen == Screen::Chat {
            self.app.chat_scroll = self.app.chat_scroll.saturating_sub(3);
            self.app.chat_scroll_manual = true;
        } else if self.app.screen == Screen::Show {
            self.app.scroll_offset = self.app.scroll_offset.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if self.app.screen == Screen::Chat {
            self.app.chat_scroll = self.app.chat_scroll.saturating_add(3);
            self.app.chat_scroll_manual = true;
        } else if self.app.screen == Screen::Show {
            self.app.scroll_offset = self.app.scroll_offset.saturating_add(1);
        }
    }

    pub fn page_up(&mut self) {
        if self.app.screen == Screen::Chat {
            self.app.chat_scroll = self.app.chat_scroll.saturating_sub(5);
            self.app.chat_scroll_manual = true;
        }
    }

    pub fn page_down(&mut self) {
        if self.app.screen == Screen::Chat {
            self.app.chat_scroll = self.app.chat_scroll.saturating_add(5);
            self.app.chat_scroll_manual = true;
        }
    }

    // Message management
    pub fn add_user_message(&mut self, message: String) {
        self.app.messages.push(("user".to_string(), message));
        self.app.message_input = crate::tui::input::TextInput::new();
        self.app.input_scroll = 0;
        self.app.status = "Thinking…".to_string();
        self.app.streaming = true;
        self.app.chat_scroll_manual = false;
        self.app.chat_scroll = 0;
    }

    // Token management
    pub fn add_token_char(&mut self, c: char) {
        self.app.api_token.push(c);
        // Save token on every keystroke
        let _ = crate::tui::api::save_api_token(&self.app.api_token);
        self.app.api_token_saved = true;
        self.app.token_saved_at = Some(std::time::Instant::now());
        self.app.status = "✓ API token saved".to_string();
    }

    pub fn remove_token_char(&mut self) {
        self.app.api_token.pop();
    }

    // URL management
    pub fn add_url_char(&mut self, c: char) {
        self.app.custom_url.push(c);
    }

    pub fn remove_url_char(&mut self) {
        self.app.custom_url.pop();
    }

    // Agent mode toggle
    pub fn toggle_agent_mode(&mut self) {
        self.app.agent_mode = !self.app.agent_mode;
        self.app.status = if self.app.agent_mode {
            "🤖 Agent Mode ON (ReAct loop) — F2 to toggle".to_string()
        } else {
            "💬 Chat Mode — F2 to enable Agent Mode".to_string()
        };
    }
}