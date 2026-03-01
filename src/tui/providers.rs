// ── Model providers ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Provider {
    OpenAI,
    Anthropic,
    XAI,
    Ollama,
    GitHubModels,
}

impl Provider {
    pub fn label(&self) -> &str {
        match self {
            Provider::OpenAI       => "OpenAI (GPT-4o)",
            Provider::Anthropic    => "Anthropic (Claude)",
            Provider::XAI          => "xAI (Grok)",
            Provider::Ollama       => "Ollama (local)",
            Provider::GitHubModels => "GitHub Models",
        }
    }
    pub fn all() -> &'static [Provider] {
        &[
            Provider::OpenAI,
            Provider::Anthropic,
            Provider::XAI,
            Provider::Ollama,
            Provider::GitHubModels,
        ]
    }
    pub fn default_model(&self) -> &str {
        match self {
            Provider::OpenAI       => "gpt-4o",
            Provider::Anthropic    => "claude-3-5-sonnet-20241022",
            Provider::XAI          => "grok-3",
            Provider::Ollama       => "gemma3",
            Provider::GitHubModels => "openai/gpt-4o",
        }
    }
    pub fn api_url(&self) -> &str {
        match self {
            Provider::OpenAI       => "https://api.openai.com/v1/chat/completions",
            Provider::Anthropic    => "https://api.anthropic.com/v1/messages",
            Provider::XAI          => "https://api.x.ai/v1/chat/completions",
            Provider::Ollama       => "http://localhost:11434/api/chat",
            Provider::GitHubModels => "https://models.github.com/v1/chat/completions",
        }
    }
    pub fn description(&self) -> &str {
        match self {
            Provider::OpenAI       => "Cloud · Requires API key · https://platform.openai.com/",
            Provider::Anthropic    => "Cloud · Requires API key · https://www.anthropic.com/",
            Provider::XAI          => "Cloud · Requires API key · https://x.ai/",
            Provider::Ollama       => "Local · No API key needed · https://ollama.com/",
            Provider::GitHubModels => "Cloud · GitHub OAuth token (models:read) · https://github.com/marketplace/models",
        }
    }
}

