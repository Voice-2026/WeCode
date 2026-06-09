package com.codux.mobile.terminal

import com.termux.terminal.TerminalEmulator
import com.termux.terminal.TerminalOutput
import com.termux.terminal.TerminalSessionClient

class RemoteTerminalSession(
    private val termuxClient: TerminalSessionClient,
    private val remoteClient: RemoteClient,
    private val transcriptRows: Int = 10000,
) : TerminalOutput() {
    private var emulator: TerminalEmulator? = null

    val columns: Int get() = emulator?.mColumns ?: 0
    val rows: Int get() = emulator?.mRows ?: 0

    fun updateSize(columns: Int, rows: Int, cellWidthPixels: Int, cellHeightPixels: Int) {
        val current = emulator
        if (current == null) {
            emulator = TerminalEmulator(this, columns, rows, transcriptRows, termuxClient)
        } else if (current.mColumns != columns || current.mRows != rows) {
            current.resize(columns, rows)
        }
        remoteClient.onResize(columns, rows)
        remoteClient.onEmulatorReady(this)
    }

    fun getEmulator(): TerminalEmulator? = emulator

    fun appendRemote(data: ByteArray) {
        val current = emulator ?: return
        current.append(data, data.size)
        remoteClient.onTextChanged(this)
    }

    fun reset() {
        emulator?.reset()
        remoteClient.onTextChanged(this)
    }

    fun writeCodePoint(altDown: Boolean, codePoint: Int) {
        val text = String(Character.toChars(codePoint))
        val data = if (altDown) "\u001B$text" else text
        CoduxTerminalLog.d("CoduxTerminalInput", "codePoint=$codePoint data=${debugData(data)}")
        remoteClient.onUserInput(data)
    }

    override fun write(data: ByteArray, offset: Int, count: Int) {
        if (count <= 0) return
        val text = String(data, offset, count, Charsets.UTF_8)
        CoduxTerminalLog.d("CoduxTerminalResponse", "write=${debugData(text)}")
        remoteClient.onTerminalResponse(text)
    }

    override fun titleChanged(oldTitle: String?, newTitle: String?) = Unit
    override fun onCopyTextToClipboard(text: String?) = Unit
    override fun onPasteTextFromClipboard() = Unit
    override fun onBell() = Unit
    override fun onColorsChanged() = Unit

    interface RemoteClient {
        fun onTextChanged(session: RemoteTerminalSession)
        fun onUserInput(data: String)
        fun onTerminalResponse(data: String)
        fun onResize(columns: Int, rows: Int)
        fun onEmulatorReady(session: RemoteTerminalSession)
    }

    private fun debugData(data: String): String {
        val maxLength = 160
        val text = if (data.length > maxLength) data.take(maxLength) + "…" else data
        return text
            .replace("\u001B", "<ESC>")
            .replace("\r", "<CR>")
            .replace("\n", "<LF>")
            .replace("\t", "<TAB>")
    }
}
