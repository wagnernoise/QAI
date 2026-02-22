package com.qai.plugin

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

/**
 * Unit tests for logic that can be exercised without an IntelliJ Platform runtime.
 *
 * QaiToolWindowFactory delegates provider/model/URL resolution entirely to
 * QaiSettings companion helpers, so these tests verify that the factory's
 * assumptions about those helpers hold — i.e. that every provider the factory
 * puts in the combo box has a valid API URL and a non-blank default model.
 */
class QaiToolWindowFactoryTest {

    // ── Every provider in the combo has a non-blank API URL ───────────────────

    @Test
    fun `apiUrlFor every provider returns non-blank url`() {
        val ollamaUrl = "http://localhost:11434"
        for (provider in QaiSettings.PROVIDERS) {
            val url = QaiSettings.apiUrlFor(provider, ollamaUrl)
            assertTrue(url.isNotBlank(), "Expected non-blank URL for provider '$provider'")
        }
    }

    @Test
    fun `apiUrlFor every provider returns an https or http url`() {
        val ollamaUrl = "http://localhost:11434"
        for (provider in QaiSettings.PROVIDERS) {
            val url = QaiSettings.apiUrlFor(provider, ollamaUrl)
            assertTrue(
                url.startsWith("http://") || url.startsWith("https://"),
                "URL for '$provider' should start with http(s)://, got: $url"
            )
        }
    }

    // ── Every provider in the combo has a non-blank default model ─────────────

    @Test
    fun `defaultModelFor every provider returns non-blank model`() {
        for (provider in QaiSettings.PROVIDERS) {
            val model = QaiSettings.defaultModelFor(provider)
            assertTrue(model.isNotBlank(), "Expected non-blank model for provider '$provider'")
        }
    }

    // ── Provider combo initialisation: selected item matches settings ─────────

    @Test
    fun `default provider from fresh settings is in PROVIDERS list`() {
        val settings = QaiSettings()
        assertTrue(
            QaiSettings.PROVIDERS.contains(settings.provider),
            "Default provider '${settings.provider}' must be in PROVIDERS list"
        )
    }

    @Test
    fun `default mode from fresh settings is in MODES list`() {
        val settings = QaiSettings()
        assertTrue(
            QaiSettings.MODES.contains(settings.mode),
            "Default mode '${settings.mode}' must be in MODES list"
        )
    }

    // ── Provider change updates model field (logic mirror) ────────────────────

    @Test
    fun `switching provider to OpenAI gives gpt-4o model`() {
        val settings = QaiSettings()
        settings.provider = "OpenAI"
        val model = QaiSettings.defaultModelFor(settings.provider)
        assertEquals("gpt-4o", model)
    }

    @Test
    fun `switching provider to Ollama gives llama3 model`() {
        val settings = QaiSettings()
        settings.provider = "Ollama"
        val model = QaiSettings.defaultModelFor(settings.provider)
        assertEquals("llama3", model)
    }

    @Test
    fun `switching provider to Anthropic gives claude model`() {
        val settings = QaiSettings()
        settings.provider = "Anthropic"
        val model = QaiSettings.defaultModelFor(settings.provider)
        assertTrue(model.startsWith("claude"), "Expected claude model, got: $model")
    }

    @Test
    fun `switching provider to xAI gives grok model`() {
        val settings = QaiSettings()
        settings.provider = "xAI"
        val model = QaiSettings.defaultModelFor(settings.provider)
        assertTrue(model.startsWith("grok"), "Expected grok model, got: $model")
    }

    @Test
    fun `switching provider to Zen gives claude model`() {
        val settings = QaiSettings()
        settings.provider = "Zen"
        val model = QaiSettings.defaultModelFor(settings.provider)
        assertTrue(model.contains("claude"), "Expected claude in Zen model, got: $model")
    }

    // ── Ollama URL routing ────────────────────────────────────────────────────

    @Test
    fun `Ollama with default localhost url routes to api-chat`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://localhost:11434")
        assertEquals("http://localhost:11434/api/chat", url)
    }

    @Test
    fun `Ollama with remote url routes to remote api-chat`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://10.0.0.5:11434")
        assertEquals("http://10.0.0.5:11434/api/chat", url)
    }

    @Test
    fun `Ollama url with trailing slash is normalised`() {
        val url = QaiSettings.apiUrlFor("Ollama", "http://10.0.0.5:11434/")
        assertFalse(url.contains("//api/chat"), "Double slash should not appear in URL: $url")
        assertTrue(url.endsWith("/api/chat"))
    }

    // ── Mode persistence ──────────────────────────────────────────────────────

    @Test
    fun `mode can be switched to Agent and back to Chat`() {
        val settings = QaiSettings()
        settings.mode = "Agent"
        assertEquals("Agent", settings.mode)
        settings.mode = "Chat"
        assertEquals("Chat", settings.mode)
    }

    // ── Settings state survives loadState round-trip ──────────────────────────

    @Test
    fun `factory reads provider from settings after loadState`() {
        val settings = QaiSettings()
        settings.loadState(QaiSettings.State(provider = "xAI", mode = "Agent"))
        assertEquals("xAI", settings.provider)
        assertEquals("Agent", settings.mode)
    }

    @Test
    fun `factory reads ollamaUrl from settings after loadState`() {
        val settings = QaiSettings()
        settings.loadState(QaiSettings.State(ollamaUrl = "http://gpu-server:11434"))
        val url = QaiSettings.apiUrlFor("Ollama", settings.ollamaUrl)
        assertEquals("http://gpu-server:11434/api/chat", url)
    }

    // ── PROVIDERS order (combo box order matters for UX) ──────────────────────

    @Test
    fun `PROVIDERS first entry is OpenAI`() {
        assertEquals("OpenAI", QaiSettings.PROVIDERS.first())
    }

    @Test
    fun `PROVIDERS last entry is Custom`() {
        assertEquals("Custom", QaiSettings.PROVIDERS.last())
    }

    @Test
    fun `MODES first entry is Chat`() {
        assertEquals("Chat", QaiSettings.MODES.first())
    }

    @Test
    fun `MODES last entry is Agent`() {
        assertEquals("Agent", QaiSettings.MODES.last())
    }
}
