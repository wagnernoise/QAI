package com.qai.plugin

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBScrollPane
import java.awt.BorderLayout
import java.awt.Color
import java.awt.Dimension
import java.awt.Font
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.PrintWriter
import javax.swing.JButton
import javax.swing.JPanel
import javax.swing.JTextArea
import javax.swing.JTextField
import javax.swing.SwingUtilities
import javax.swing.border.EmptyBorder

/**
 * Tool window factory that embeds a qai-cli session inside the IDE.
 *
 * The panel runs the compiled `qai-cli` binary as a child process and pipes
 * stdin/stdout so the user can interact with the TUI from within IntelliJ.
 * When the binary is not found, a friendly error panel is shown instead.
 */
class QaiToolWindowFactory : ToolWindowFactory {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val binary = QaiBinaryLocator.find()
        val panel = if (binary != null) {
            buildTerminalPanel(binary, project)
        } else {
            buildMissingBinaryPanel()
        }

        val content = ContentFactory.getInstance()
            .createContent(panel, "", false)
        toolWindow.contentManager.addContent(content)
    }

    // -------------------------------------------------------------------------
    // Terminal panel — wraps qai-cli in a simple Swing pseudo-terminal
    // -------------------------------------------------------------------------

    private fun buildTerminalPanel(binary: String, project: Project): JPanel {
        val output = JTextArea().apply {
            isEditable = false
            background = Color(30, 30, 30)
            foreground = Color(220, 220, 220)
            font = Font(Font.MONOSPACED, Font.PLAIN, 13)
            lineWrap = true
            wrapStyleWord = false
            border = EmptyBorder(4, 6, 4, 6)
        }

        val input = JTextField().apply {
            background = Color(45, 45, 45)
            foreground = Color(220, 220, 220)
            caretColor = Color(220, 220, 220)
            font = Font(Font.MONOSPACED, Font.PLAIN, 13)
            border = EmptyBorder(4, 6, 4, 6)
        }

        val sendButton = JButton("Send").apply {
            toolTipText = "Send message (Enter)"
        }

        val scrollPane = JBScrollPane(output).apply {
            preferredSize = Dimension(600, 400)
        }

        val inputPanel = JPanel(BorderLayout(4, 0)).apply {
            background = Color(45, 45, 45)
            add(input, BorderLayout.CENTER)
            add(sendButton, BorderLayout.EAST)
        }

        val root = JPanel(BorderLayout(0, 4)).apply {
            background = Color(30, 30, 30)
            border = EmptyBorder(4, 4, 4, 4)
            add(scrollPane, BorderLayout.CENTER)
            add(inputPanel, BorderLayout.SOUTH)
        }

        // Launch qai-cli as a child process
        val workDir = project.basePath?.let { java.io.File(it) }
        val process = ProcessBuilder(binary)
            .apply { workDir?.let { directory(it) } }
            .redirectErrorStream(true)
            .start()

        val writer = PrintWriter(process.outputStream, true)

        // Read stdout in a background thread and append to the text area
        Thread({
            BufferedReader(InputStreamReader(process.inputStream)).use { reader ->
                var line: String?
                while (reader.readLine().also { line = it } != null) {
                    val text = line + "\n"
                    SwingUtilities.invokeLater { output.append(text) }
                }
            }
        }, "qai-cli-reader").apply {
            isDaemon = true
            start()
        }

        val sendAction = {
            val text = input.text.trim()
            if (text.isNotEmpty()) {
                writer.println(text)
                output.append("> $text\n")
                input.text = ""
            }
        }

        input.addActionListener { sendAction() }
        sendButton.addActionListener { sendAction() }

        return root
    }

    // -------------------------------------------------------------------------
    // Fallback panel shown when qai-cli binary is not found
    // -------------------------------------------------------------------------

    private fun buildMissingBinaryPanel(): JPanel {
        val label = JBLabel(
            "<html><b>qai-cli not found on PATH.</b><br><br>" +
                "Build and install it with:<br>" +
                "<code>cargo build --release</code><br>" +
                "<code>sudo cp target/release/qai-cli /usr/local/bin/</code><br><br>" +
                "Then restart the IDE.</html>"
        )

        return JPanel(BorderLayout()).apply {
            border = EmptyBorder(16, 16, 16, 16)
            add(label, BorderLayout.NORTH)
        }
    }
}
