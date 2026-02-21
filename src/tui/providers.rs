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
            Provider::Zen       => "anthropic/claude-sonnet-4-5",
            Provider::Custom    => "custom-model",
        }
    }
    pub fn api_url(&self) -> &str {
        match self {
            Provider::OpenAI    => "https://api.openai.com/v1/chat/completions",
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
            Provider::XAI       => "https://api.x.ai/v1/chat/completions",
            Provider::Ollama    => "http://localhost:11434/api/chat",
            Provider::Zen       => "https://api.opencode.ai/v1/chat/completions",
            Provider::Custom    => "",
        }
    }
    pub fn description(&self) -> &str {
        match self {
            Provider::OpenAI    => "Cloud · Requires API key · https://platform.openai.com/",
            Provider::Anthropic => "Cloud · Requires API key · https://www.anthropic.com/",
            Provider::XAI       => "Cloud · Requires API key · https://x.ai/",
            Provider::Ollama    => "Local · No API key needed · https://ollama.com/",
            Provider::Zen       => "Cloud · Requires API key · https://opencode.ai/zen",
            Provider::Custom    => "Custom OpenAI-compatible endpoint · Enter URL below",
        }
    }
}

