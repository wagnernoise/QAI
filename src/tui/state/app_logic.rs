use crate::tui::state::App;
use crate::tui::providers::Provider;

impl App {
    /// Returns the current message input text as a single string.
    pub fn message_input_text(&self) -> String {
        self.message_input.value.clone()
    }

    /// Returns the currently selected provider based on provider_index.
    pub fn selected_provider(&self) -> Provider {
        Provider::all()[self.provider_index].clone()
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

    /// Returns true if the current provider is Ollama.
    pub fn is_ollama_provider(&self) -> bool {
        self.selected_provider() == Provider::Ollama
    }

    /// Returns true if the current provider is Custom.
    pub fn is_custom_provider(&self) -> bool {
        self.selected_provider() == Provider::Custom
    }
}