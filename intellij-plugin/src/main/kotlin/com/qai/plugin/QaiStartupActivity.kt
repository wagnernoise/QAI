package com.qai.plugin

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity

/**
 * Runs once when a project is opened.
 *
 * Checks whether `qai-cli` is available on PATH and shows a balloon
 * notification with build instructions if it is missing.
 */
class QaiStartupActivity : ProjectActivity {

    override suspend fun execute(project: Project) {
        if (QaiBinaryLocator.find() != null) return

        val message =
            "<b>qai-cli</b> was not found on your PATH.<br>" +
            "Build and install it:<br>" +
            "<code>cargo build --release</code><br>" +
            "<code>sudo cp target/release/qai-cli /usr/local/bin/</code><br>" +
            "Then reopen the project to activate the QAI tool window."

        NotificationGroupManager.getInstance()
            .getNotificationGroup("QAI Notifications")
            .createNotification("QAI: binary not found", message, NotificationType.WARNING)
            .notify(project)
    }
}
