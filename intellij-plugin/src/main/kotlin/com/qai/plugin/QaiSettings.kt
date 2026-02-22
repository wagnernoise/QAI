package com.qai.plugin

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage

@State(
    name = "QaiSettings",
    storages = [Storage("qai-settings.xml")]
)
@Service(Service.Level.APP)
class QaiSettings : PersistentStateComponent<QaiSettings.State> {

    data class State(
        var apiToken: String = "",
        var ollamaUrl: String = "http://localhost:11434",
        var provider: String = "Ollama",
        var mode: String = "Chat"
    )

    private var myState = State()

    override fun getState(): State = myState

    override fun loadState(state: State) {
        myState = state
    }

    var apiToken: String
        get() = myState.apiToken
        set(value) { myState.apiToken = value }

    var ollamaUrl: String
        get() = myState.ollamaUrl
        set(value) { myState.ollamaUrl = value }

    var provider: String
        get() = myState.provider
        set(value) { myState.provider = value }

    var mode: String
        get() = myState.mode
        set(value) { myState.mode = value }

    companion object {
        fun getInstance(): QaiSettings =
            ApplicationManager.getApplication().getService(QaiSettings::class.java)

        val PROVIDERS = listOf("OpenAI", "Anthropic", "xAI", "Ollama", "Zen", "Custom")
        val MODES = listOf("Chat", "Agent")

        fun apiUrlFor(provider: String, ollamaUrl: String): String = when (provider) {
            "OpenAI" -> "https://api.openai.com/v1/chat/completions"
            "Anthropic" -> "https://api.anthropic.com/v1/messages"
            "xAI" -> "https://api.x.ai/v1/chat/completions"
            "Ollama" -> "${ollamaUrl.trimEnd('/')}/api/chat"
            "Zen" -> "https://api.opencode.ai/v1/chat/completions"
            else -> "https://api.openai.com/v1/chat/completions"
        }

        fun defaultModelFor(provider: String): String = when (provider) {
            "OpenAI" -> "gpt-4o"
            "Anthropic" -> "claude-opus-4-5"
            "xAI" -> "grok-3"
            "Ollama" -> "llama3"
            "Zen" -> "anthropic/claude-sonnet-4-5"
            else -> "gpt-4o"
        }

        fun modelsFor(provider: String): List<String> = when (provider) {
            "OpenAI"    -> listOf("gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-mini", "o3-mini")
            "Anthropic" -> listOf("claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-3-5", "claude-3-opus-20240229")
            "xAI"       -> listOf("grok-3", "grok-3-mini", "grok-2", "grok-beta")
            "Ollama"    -> listOf("llama3", "llama3:8b", "llama3:70b", "mistral", "gemma3", "qwen2.5-coder", "phi3")
            "Zen"       -> listOf("anthropic/claude-sonnet-4-5", "anthropic/claude-opus-4-5", "openai/gpt-4o", "google/gemini-2.0-flash")
            else        -> listOf("gpt-4o")
        }
    }
}
