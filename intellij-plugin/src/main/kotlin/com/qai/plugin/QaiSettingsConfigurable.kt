package com.qai.plugin

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.ComboBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBPasswordField
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.JBUI
import java.awt.GridBagConstraints
import java.awt.GridBagLayout
import javax.swing.JComboBox
import javax.swing.JComponent
import javax.swing.JPanel

class QaiSettingsConfigurable : Configurable {

    private var apiTokenField: JBPasswordField? = null
    private var ollamaUrlField: JBTextField? = null
    private var providerCombo: JComboBox<String>? = null
    private var modeCombo: JComboBox<String>? = null

    override fun getDisplayName(): String = "QAI"

    override fun createComponent(): JComponent {
        val settings = QaiSettings.getInstance()

        apiTokenField = JBPasswordField().apply {
            text = settings.apiToken
            columns = 40
        }
        ollamaUrlField = JBTextField(settings.ollamaUrl, 40)
        providerCombo =  ComboBox(QaiSettings.PROVIDERS.toTypedArray()).apply {
            selectedItem = settings.provider
        }
        modeCombo = ComboBox(QaiSettings.MODES.toTypedArray()).apply {
            selectedItem = settings.mode
        }

        val panel = JPanel(GridBagLayout())
        val gbc = GridBagConstraints().apply {
            anchor = GridBagConstraints.WEST
            insets = JBUI.insets(4)
        }

        fun addRow(label: String, component: JComponent, row: Int) {
            gbc.gridx = 0; gbc.gridy = row; gbc.fill = GridBagConstraints.NONE
            panel.add(JBLabel(label), gbc)
            gbc.gridx = 1; gbc.fill = GridBagConstraints.HORIZONTAL; gbc.weightx = 1.0
            panel.add(component, gbc)
            gbc.weightx = 0.0
        }

        addRow("Provider:", providerCombo!!, 0)
        addRow("Mode:", modeCombo!!, 1)
        addRow("API Token:", apiTokenField!!, 2)
        addRow("Ollama Server URL:", ollamaUrlField!!, 3)

        // filler row
        gbc.gridx = 0; gbc.gridy = 4; gbc.gridwidth = 2
        gbc.fill = GridBagConstraints.BOTH; gbc.weighty = 1.0
        panel.add(JPanel(), gbc)

        return panel
    }

    override fun isModified(): Boolean {
        val s = QaiSettings.getInstance()
        return String(apiTokenField!!.password) != s.apiToken ||
            ollamaUrlField!!.text != s.ollamaUrl ||
            providerCombo!!.selectedItem as String != s.provider ||
            modeCombo!!.selectedItem as String != s.mode
    }

    override fun apply() {
        val s = QaiSettings.getInstance()
        s.apiToken = String(apiTokenField!!.password)
        s.ollamaUrl = ollamaUrlField!!.text
        s.provider = providerCombo!!.selectedItem as String
        s.mode = modeCombo!!.selectedItem as String
    }

    override fun reset() {
        val s = QaiSettings.getInstance()
        apiTokenField!!.text = s.apiToken
        ollamaUrlField!!.text = s.ollamaUrl
        providerCombo!!.selectedItem = s.provider
        modeCombo!!.selectedItem = s.mode
    }
}
