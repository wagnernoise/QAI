package com.qai.plugin

import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.Gray
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.components.JBTextArea
import com.intellij.ui.content.ContentFactory
import com.intellij.util.ui.JBUI
import org.json.JSONArray
import org.json.JSONObject
import java.awt.BorderLayout
import java.awt.Dimension
import java.awt.FlowLayout
import java.awt.Font
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URI
import javax.swing.*

class QaiToolWindowFactory : ToolWindowFactory {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = buildChatPanel(project)
        val content = ContentFactory.getInstance().createContent(panel, "", false)
        toolWindow.contentManager.addContent(content)
    }

    private fun buildChatPanel(@Suppress("UNUSED_PARAMETER") project: Project): JPanel {
        val settings = QaiSettings.getInstance()

        // ── Colours ──────────────────────────────────────────────────────────
        val bgDark   = Gray._30
        val bgMid    = Gray._45
        val fgLight  = Gray._220
        val monoFont = Font(Font.MONOSPACED, Font.PLAIN, 13)

        // ── Conversation area ─────────────────────────────────────────────────
        val conversation = JBTextArea().apply {
            isEditable    = false
            background    = bgDark
            foreground    = fgLight
            font          = monoFont
            lineWrap      = true
            wrapStyleWord = true
            border        = JBUI.Borders.empty(6, 8)
        }
        val scrollPane = JBScrollPane(conversation).apply {
            preferredSize = Dimension(600, 400)
            border        = BorderFactory.createLineBorder(Gray._70)
        }

        // ── Provider combo ────────────────────────────────────────────────────
        val providerCombo = ComboBox(QaiSettings.PROVIDERS.toTypedArray()).apply {
            selectedItem = settings.provider
            toolTipText  = "Select AI provider"
        }

        // ── Mode combo ────────────────────────────────────────────────────────
        val modeCombo = ComboBox(QaiSettings.MODES.toTypedArray()).apply {
            selectedItem = settings.mode
            toolTipText  = "Chat or Agent mode"
        }

        // ── Model combo (editable) ────────────────────────────────────────────
        val modelCombo = ComboBox<String>().apply {
            isEditable  = true
            toolTipText = "Select or type a model name"
            preferredSize = Dimension(200, preferredSize.height)
            (editor.editorComponent as? JTextField)?.apply {
                background = bgMid
                foreground = fgLight
                caretColor = fgLight
                font       = monoFont
            }
        }

        fun populateModels(provider: String) {
            modelCombo.removeAllItems()
            QaiSettings.modelsFor(provider).forEach { modelCombo.addItem(it) }
            modelCombo.selectedItem = QaiSettings.defaultModelFor(provider)
        }

        fun fetchOllamaModels(ollamaUrl: String) {
            Thread {
                try {
                    val url  = "${ollamaUrl.trimEnd('/')}/api/tags"
                    val conn = (URI.create(url).toURL().openConnection() as HttpURLConnection).apply {
                        requestMethod  = "GET"
                        connectTimeout = 5_000
                        readTimeout    = 10_000
                    }
                    val text = BufferedReader(InputStreamReader(conn.inputStream)).use { it.readText() }
                    val models = org.json.JSONObject(text)
                        .getJSONArray("models")
                        .let { arr -> (0 until arr.length()).map { arr.getJSONObject(it).getString("name") } }
                    SwingUtilities.invokeLater {
                        modelCombo.removeAllItems()
                        models.forEach { modelCombo.addItem(it) }
                        if (models.isNotEmpty()) modelCombo.selectedIndex = 0
                    }
                } catch (_: Exception) {
                    // keep static list on error
                }
            }.also { it.isDaemon = true }.start()
        }

        populateModels(settings.provider)
        if (settings.provider == "Ollama") fetchOllamaModels(settings.ollamaUrl)

        providerCombo.addActionListener {
            val p = providerCombo.selectedItem as String
            settings.provider = p
            populateModels(p)
            if (p == "Ollama") fetchOllamaModels(settings.ollamaUrl)
        }
        modeCombo.addActionListener { settings.mode = modeCombo.selectedItem as String }

        // ── Top toolbar ───────────────────────────────────────────────────────
        val toolbar = JPanel(FlowLayout(FlowLayout.LEFT, 6, 4)).apply {
            background = bgMid
            add(JLabel("Provider:").apply { foreground = fgLight; font = monoFont })
            add(providerCombo)
            add(JLabel("Mode:").apply { foreground = fgLight; font = monoFont })
            add(modeCombo)
            add(JLabel("Model:").apply { foreground = fgLight; font = monoFont })
            add(modelCombo)
        }

        // ── Message input ─────────────────────────────────────────────────────
        val inputArea = JTextArea(3, 40).apply {
            background    = bgMid
            foreground    = fgLight
            caretColor    = fgLight
            font          = monoFont
            lineWrap      = true
            wrapStyleWord = true
            border        = JBUI.Borders.empty(4, 6)
        }
        val inputScroll = JBScrollPane(inputArea).apply {
            border = BorderFactory.createLineBorder(Gray._70)
        }

        val sendButton  = JButton("Send ↵").apply {
            toolTipText = "Send message (Ctrl+Enter)"
            font        = Font(Font.SANS_SERIF, Font.BOLD, 12)
        }
        val clearButton = JButton("Clear").apply {
            toolTipText = "Clear conversation"
            font        = Font(Font.SANS_SERIF, Font.PLAIN, 12)
        }

        val buttonPanel = JPanel(FlowLayout(FlowLayout.RIGHT, 4, 0)).apply {
            background = bgMid
            add(clearButton)
            add(sendButton)
        }

        val inputPanel = JPanel(BorderLayout(0, 4)).apply {
            background = bgMid
            border     = JBUI.Borders.empty(4)
            add(inputScroll, BorderLayout.CENTER)
            add(buttonPanel, BorderLayout.SOUTH)
        }

        // ── Root panel ────────────────────────────────────────────────────────
        val root = JPanel(BorderLayout(0, 4)).apply {
            background = bgDark
            border     = JBUI.Borders.empty(4)
            add(toolbar,    BorderLayout.NORTH)
            add(scrollPane, BorderLayout.CENTER)
            add(inputPanel, BorderLayout.SOUTH)
        }

        // ── Conversation history ──────────────────────────────────────────────
        val history = mutableListOf<Pair<String, String>>()

        fun appendLine(role: String, text: String) {
            val prefix = if (role == "user") "You" else "QA-Bot"
            SwingUtilities.invokeLater {
                conversation.append("\n$prefix:\n$text\n")
                conversation.caretPosition = conversation.document.length
            }
        }

        // ── HTTP call (background thread) ─────────────────────────────────────
        fun sendMessage(userText: String) {
            val provider  = providerCombo.selectedItem as String
            val model     = (modelCombo.editor.item as? String)?.trim() ?: modelCombo.selectedItem as? String ?: ""
            val token     = settings.apiToken
            val ollamaUrl = settings.ollamaUrl
            val apiUrl    = QaiSettings.apiUrlFor(provider, ollamaUrl)

            history.add("user" to userText)

            val systemPrompt = "You are QA-Bot, a QA automation AI assistant. " +
                "Help the user with testing, quality assurance, and software development tasks."

            val messages = JSONArray()
            if (provider != "Anthropic") {
                messages.put(JSONObject().put("role", "system").put("content", systemPrompt))
            }
            for ((role, content) in history) {
                messages.put(JSONObject().put("role", role).put("content", content))
            }

            try {
                val conn = (URI.create(apiUrl).toURL().openConnection() as HttpURLConnection).apply {
                    requestMethod  = "POST"
                    connectTimeout = 10_000
                    readTimeout    = 120_000
                    doOutput       = true
                    setRequestProperty("Content-Type", "application/json")
                    if (token.isNotBlank() && provider != "Anthropic") {
                        setRequestProperty("Authorization", "Bearer $token")
                    }
                    if (provider == "Anthropic") {
                        setRequestProperty("x-api-key", token)
                        setRequestProperty("anthropic-version", "2023-06-01")
                    }
                }

                val body = when (provider) {
                    "Anthropic" -> JSONObject()
                        .put("model", model)
                        .put("max_tokens", 4096)
                        .put("system", systemPrompt)
                        .put("messages", messages)
                        .toString()
                    else -> JSONObject()
                        .put("model", model)
                        .put("messages", messages)
                        .put("stream", false)
                        .toString()
                }

                conn.outputStream.use { it.write(body.toByteArray()) }

                val responseText = BufferedReader(InputStreamReader(conn.inputStream)).use { it.readText() }
                val json = JSONObject(responseText)

                val reply = when (provider) {
                    "Anthropic" -> json.getJSONArray("content")
                        .getJSONObject(0).getString("text")
                    "Ollama"    -> json.getJSONObject("message").getString("content")
                    else        -> json.getJSONArray("choices")
                        .getJSONObject(0).getJSONObject("message").getString("content")
                }

                history.add("assistant" to reply)
                appendLine("assistant", reply)

            } catch (e: Exception) {
                val err = "[Error: ${e.message}]"
                history.add("assistant" to err)
                appendLine("assistant", err)
            }
        }

        // ── Wire up actions ───────────────────────────────────────────────────
        val sendAction = {
            val text = inputArea.text.trim()
            if (text.isNotEmpty()) {
                inputArea.text = ""
                appendLine("user", text)
                SwingUtilities.invokeLater {
                    conversation.append("\n⏳ Thinking…\n")
                    conversation.caretPosition = conversation.document.length
                    sendButton.isEnabled = false
                }
                Thread({
                    sendMessage(text)
                    SwingUtilities.invokeLater { sendButton.isEnabled = true }
                }, "qai-http").apply { isDaemon = true; start() }
            }
        }

        sendButton.addActionListener { sendAction() }
        clearButton.addActionListener { history.clear(); conversation.text = "" }

        // Ctrl+Enter to send
        inputArea.inputMap.put(
            KeyStroke.getKeyStroke(java.awt.event.KeyEvent.VK_ENTER, java.awt.event.InputEvent.CTRL_DOWN_MASK),
            "send"
        )
        inputArea.actionMap.put("send", object : AbstractAction() {
            override fun actionPerformed(e: java.awt.event.ActionEvent) { sendAction() }
        })

        return root
    }
}
