package com.qai.plugin

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

class QaiSettingsTest {

    // ── State defaults ────────────────────────────────────────────────────────

    @Test
    fun `State default apiToken is empty`() {
        val state = QaiSettings.State()
        assertEquals("", state.apiToken)
    }

    @Test
    fun `State default ollamaUrl is localhost`() {
        val state = QaiSettings.State()
        assertEquals("http://localhost:11434", state.ollamaUrl)
    }

    @Test
    fun `State default provider is Ollama`() {
        val state = QaiSettings.State()
        assertEquals("Ollama", state.provider)
    }

    @Test
    fun `State default mode is Chat`() {
        val state = QaiSettings.State()
        assertEquals("Chat", state.mode)
    }

    @Test
    fun `State fields are mutable`() {
        val state = QaiSettings.State()
        state.apiToken = "tok-123"
        state.ollamaUrl = "http://192.168.1.10:11434"
        state.provider = "OpenAI"
        state.mode = "Agent"
        assertEquals("tok-123", state.apiToken)
        assertEquals("http://192.168.1.10:11434", state.ollamaUrl)
        assertEquals("OpenAI", state.provider)
        assertEquals("Agent", state.mode)
    }

    @Test
    fun `State data class equality works`() {
        val a = QaiSettings.State(apiToken = "x", provider = "OpenAI")
        val b = QaiSettings.State(apiToken = "x", provider = "OpenAI")
        assertEquals(a, b)
    }

    @Test
    fun `State data class copy works`() {
        val original = QaiSettings.State(apiToken = "abc", mode = "Agent")
        val copy = original.copy(mode = "Chat")
        assertEquals("abc", copy.apiToken)
        assertEquals("Chat", copy.mode)
        assertEquals("Agent", original.mode)
    }

    // ── modelsFor ─────────────────────────────────────────────────────────────

    @Test
    fun `modelsFor OpenAI is non-empty and contains gpt-4o`() {
        val models = QaiSettings.modelsFor("OpenAI")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("gpt-4o"))
    }

    @Test
    fun `modelsFor OpenAI default model is in list`() {
        val models = QaiSettings.modelsFor("OpenAI")
        assertTrue(models.contains(QaiSettings.defaultModelFor("OpenAI")))
    }

    @Test
    fun `modelsFor Anthropic is non-empty and contains claude-opus-4-5`() {
        val models = QaiSettings.modelsFor("Anthropic")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("claude-opus-4-5"))
    }

    @Test
    fun `modelsFor Anthropic default model is in list`() {
        val models = QaiSettings.modelsFor("Anthropic")
        assertTrue(models.contains(QaiSettings.defaultModelFor("Anthropic")))
    }

    @Test
    fun `modelsFor xAI is non-empty and contains grok-3`() {
        val models = QaiSettings.modelsFor("xAI")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("grok-3"))
    }

    @Test
    fun `modelsFor xAI default model is in list`() {
        val models = QaiSettings.modelsFor("xAI")
        assertTrue(models.contains(QaiSettings.defaultModelFor("xAI")))
    }

    @Test
    fun `modelsFor Ollama is non-empty and contains llama3`() {
        val models = QaiSettings.modelsFor("Ollama")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("llama3"))
    }

    @Test
    fun `modelsFor Ollama default model is in list`() {
        val models = QaiSettings.modelsFor("Ollama")
        assertTrue(models.contains(QaiSettings.defaultModelFor("Ollama")))
    }

    @Test
    fun `modelsFor Zen is non-empty and contains claude-sonnet model`() {
        val models = QaiSettings.modelsFor("Zen")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("anthropic/claude-sonnet-4-5"))
    }

    @Test
    fun `modelsFor Zen default model is in list`() {
        val models = QaiSettings.modelsFor("Zen")
        assertTrue(models.contains(QaiSettings.defaultModelFor("Zen")))
    }

    @Test
    fun `modelsFor Custom falls back to gpt-4o list`() {
        val models = QaiSettings.modelsFor("Custom")
        assertTrue(models.isNotEmpty())
        assertTrue(models.contains("gpt-4o"))
    }

    @Test
    fun `modelsFor unknown provider returns non-empty fallback`() {
        val models = QaiSettings.modelsFor("Unknown")
        assertTrue(models.isNotEmpty())
    }

    @Test
    fun `modelsFor all providers have at least 1 model`() {
        QaiSettings.PROVIDERS.forEach { provider ->
            assertTrue(QaiSettings.modelsFor(provider).isNotEmpty(), "modelsFor($provider) should not be empty")
        }
    }

    @Test
    fun `modelsFor all providers default model is in their list`() {
        QaiSettings.PROVIDERS.filter { it != "Custom" }.forEach { provider ->
            val models = QaiSettings.modelsFor(provider)
            val default = QaiSettings.defaultModelFor(provider)
            assertTrue(models.contains(default), "modelsFor($provider) should contain default model '$default'")
        }
    }

    // ── PROVIDERS list ────────────────────────────────────────────────────────

    @Test
    fun `PROVIDERS contains exactly 6 entries`() {
        assertEquals(6, QaiSettings.PROVIDERS.size)
    }

    @Test
    fun `PROVIDERS contains OpenAI`() {
        assertTrue(QaiSettings.PROVIDERS.contains("OpenAI"))
    }

    @Test
    fun `PROVIDERS contains Anthropic`() {
        assertTrue(QaiSettings.PROVIDERS.contains("Anthropic"))
    }

    @Test
    fun `PROVIDERS contains xAI`() {
        assertTrue(QaiSettings.PROVIDERS.contains("xAI"))
    }

    @Test
    fun `PROVIDERS contains Ollama`() {
        assertTrue(QaiSettings.PROVIDERS.contains("Ollama"))
    }

    @Test
    fun `PROVIDERS contains Zen`() {
        assertTrue(QaiSettings.PROVIDERS.contains("Zen"))
    }

    @Test
    fun `PROVIDERS contains Custom`() {
        assertTrue(QaiSettings.PROVIDERS.contains("Custom"))
    }

    // ── MODES list ────────────────────────────────────────────────────────────

    @Test
    fun `MODES contains exactly 2 entries`() {
        assertEquals(2, QaiSettings.MODES.size)
    }

    @Test
    fun `MODES contains Chat`() {
        assertTrue(QaiSettings.MODES.contains("Chat"))
    }

    @Test
    fun `MODES contains Agent`() {
        assertTrue(QaiSettings.MODES.contains("Agent"))
    }

    // ── apiUrlFor ─────────────────────────────────────────────────────────────

    @Test
    fun `apiUrlFor OpenAI returns OpenAI endpoint`() {
        assertEquals(
            "https://api.openai.com/v1/chat/completions",
            QaiSettings.apiUrlFor("OpenAI", "")
        )
    }

    @Test
    fun `apiUrlFor Anthropic returns Anthropic endpoint`() {
        assertEquals(
            "https://api.anthropic.com/v1/messages",
            QaiSettings.apiUrlFor("Anthropic", "")
        )
    }

    @Test
    fun `apiUrlFor xAI returns xAI endpoint`() {
        assertEquals(
            "https://api.x.ai/v1/chat/completions",
            QaiSettings.apiUrlFor("xAI", "")
        )
    }

    @Test
    fun `apiUrlFor Ollama uses default localhost when ollamaUrl is empty`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://localhost:11434")
        assertEquals("http://localhost:11434/api/chat", url)
    }

    @Test
    fun `apiUrlFor Ollama uses custom url`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://192.168.1.10:11434")
        assertEquals("http://192.168.1.10:11434/api/chat", url)
    }

    @Test
    fun `apiUrlFor Ollama trims trailing slash from custom url`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://192.168.1.10:11434/")
        assertEquals("http://192.168.1.10:11434/api/chat", url)
    }

    @Test
    fun `apiUrlFor Zen returns opencode endpoint`() {
        assertEquals(
            "https://api.opencode.ai/v1/chat/completions",
            QaiSettings.apiUrlFor("Zen", "")
        )
    }

    @Test
    fun `apiUrlFor Custom falls back to OpenAI endpoint`() {
        assertEquals(
            "https://api.openai.com/v1/chat/completions",
            QaiSettings.apiUrlFor("Custom", "")
        )
    }

    @Test
    fun `apiUrlFor unknown provider falls back to OpenAI endpoint`() {
        assertEquals(
            "https://api.openai.com/v1/chat/completions",
            QaiSettings.apiUrlFor("Unknown", "")
        )
    }

    // ── defaultModelFor ───────────────────────────────────────────────────────

    @Test
    fun `defaultModelFor OpenAI returns gpt-4o`() {
        assertEquals("gpt-4o", QaiSettings.defaultModelFor("OpenAI"))
    }

    @Test
    fun `defaultModelFor Anthropic returns claude model`() {
        val model = QaiSettings.defaultModelFor("Anthropic")
        assertTrue(model.startsWith("claude"), "Expected claude model, got: $model")
    }

    @Test
    fun `defaultModelFor xAI returns grok model`() {
        val model = QaiSettings.defaultModelFor("xAI")
        assertTrue(model.startsWith("grok"), "Expected grok model, got: $model")
    }

    @Test
    fun `defaultModelFor Ollama returns llama3`() {
        assertEquals("llama3", QaiSettings.defaultModelFor("Ollama"))
    }

    @Test
    fun `defaultModelFor Zen returns anthropic claude model`() {
        val model = QaiSettings.defaultModelFor("Zen")
        assertTrue(model.contains("claude"), "Expected claude in Zen model, got: $model")
    }

    @Test
    fun `defaultModelFor Custom falls back to gpt-4o`() {
        assertEquals("gpt-4o", QaiSettings.defaultModelFor("Custom"))
    }

    @Test
    fun `defaultModelFor unknown provider falls back to gpt-4o`() {
        assertEquals("gpt-4o", QaiSettings.defaultModelFor("Unknown"))
    }

    // ── loadState / getState round-trip ───────────────────────────────────────

    @Test
    fun `loadState then getState returns same values`() {
        val settings = QaiSettings()
        val loaded = QaiSettings.State(
            apiToken = "my-token",
            ollamaUrl = "http://remote:11434",
            provider = "xAI",
            mode = "Agent"
        )
        settings.loadState(loaded)
        val retrieved = settings.state
        assertEquals("my-token", retrieved.apiToken)
        assertEquals("http://remote:11434", retrieved.ollamaUrl)
        assertEquals("xAI", retrieved.provider)
        assertEquals("Agent", retrieved.mode)
    }

    @Test
    fun `property accessors delegate to state`() {
        val settings = QaiSettings()
        settings.apiToken = "tok-abc"
        settings.ollamaUrl = "http://gpu-box:11434"
        settings.provider = "Anthropic"
        settings.mode = "Agent"
        assertEquals("tok-abc", settings.apiToken)
        assertEquals("http://gpu-box:11434", settings.ollamaUrl)
        assertEquals("Anthropic", settings.provider)
        assertEquals("Agent", settings.mode)
    }

    @Test
    fun `fresh QaiSettings has empty apiToken`() {
        val settings = QaiSettings()
        assertEquals("", settings.apiToken)
    }

    @Test
    fun `fresh QaiSettings has default Ollama provider`() {
        val settings = QaiSettings()
        assertEquals("Ollama", settings.provider)
    }

    @Test
    fun `fresh QaiSettings has Chat mode`() {
        val settings = QaiSettings()
        assertEquals("Chat", settings.mode)
    }
}
