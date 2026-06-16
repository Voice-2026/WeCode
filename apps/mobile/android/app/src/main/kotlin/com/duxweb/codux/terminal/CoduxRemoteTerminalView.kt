package com.duxweb.codux.terminal

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.text.InputType
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.VelocityTracker
import android.view.View
import android.view.ViewConfiguration
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.OverScroller
import com.termux.terminal.KeyHandler
import com.termux.terminal.TerminalEmulator
import com.termux.terminal.TerminalOutput
import com.termux.terminal.TerminalSession
import com.termux.terminal.TerminalSessionClient
import com.termux.terminal.TextStyle
import com.termux.view.TerminalRenderer
import java.nio.charset.StandardCharsets
import kotlin.math.max
import kotlin.math.roundToInt

class CoduxRemoteTerminalView(
    context: Context,
    private val callbacks: Callback,
) : View(context), TerminalSessionClient {
    interface Callback {
        fun onInput(data: String)
        fun onSelectionChanged(text: String?)
        fun onResize(cols: Int, rows: Int)
        fun onCursorMetrics(cursorRow: Int, cursorCol: Int, lineHeight: Double)
    }

    private val terminalOutput = object : TerminalOutput() {
        override fun write(data: ByteArray, offset: Int, count: Int) {
            // This view renders remote output only. TerminalEmulator may answer
            // OSC/DSR queries while replaying host output; those replies belong
            // to a local PTY, not to the remote host input stream.
        }

        override fun titleChanged(oldTitle: String?, newTitle: String?) = Unit
        override fun onCopyTextToClipboard(text: String?) = Unit
        override fun onPasteTextFromClipboard() = Unit
        override fun onBell() = Unit
        override fun onColorsChanged() = invalidate()
    }

    private val terminalTypeface: Typeface = chooseNativeTerminalTypeface(context)
    private var terminalTextSizePx = logicalPxToPhysicalPx(14.0)
    private var renderer = TerminalRenderer(terminalTextSizePx, terminalTypeface)
    private var emulator: TerminalEmulator? = null
    private var pendingReplayText: String? = null
    private var topRow = 0
    private var lastCols = 0
    private var lastRows = 0
    private var lastCursorMetrics: TerminalCursorMetrics? = null
    private val scroller = OverScroller(context)
    private val touchSlop = ViewConfiguration.get(context).scaledTouchSlop
    private val maxFlingVelocity = ViewConfiguration.get(context).scaledMaximumFlingVelocity
    private var velocityTracker: VelocityTracker? = null
    private var lastTouchY = 0f
    private var dragging = false
    private var scrollRemainder = 0f
    private var lastScrollerY = 0
    private var selectionActive = false
    private var selectionDragging = false
    private var selectionStartCol = -1
    private var selectionStartRow = -1
    private var selectionEndCol = -1
    private var selectionEndRow = -1
    private var activeSelectionHandle: SelectionHandle? = null
    private var downX = 0f
    private var downY = 0f
    private val selectionStartHandle = loadTermuxSelectionHandle(
        context,
        "text_select_handle_left_material",
    )
    private val selectionEndHandle = loadTermuxSelectionHandle(
        context,
        "text_select_handle_right_material",
    )
    private val longPressRunnable = Runnable {
        startSelection(downX, downY)
    }

    init {
        isFocusable = true
        isFocusableInTouchMode = true
        setBackgroundColor(TERMINAL_BACKGROUND_COLOR)
    }

    fun feed(text: String) {
        if (text.isEmpty()) return
        val terminal = ensureEmulator()
        if (terminal == null) {
            pendingReplayText = pendingReplayText.orEmpty() + text
            return
        }
        val wasAtBottom = topRow == 0
        appendToTerminal(terminal, text)
        val scrollCounter = terminal.getScrollCounter()
        terminal.clearScrollCounter()
        topRow = if (wasAtBottom) {
            0
        } else {
            topRow - scrollCounter
        }
        clampTopRow()
        emitCursorMetrics()
        invalidate()
    }

    fun replace(text: String) {
        pendingReplayText = text
        emulator = null
        topRow = 0
        clearSelection(invalidateView = false)
        requestResize(invalidateView = false)
        applyPendingReplay()
        emitCursorMetrics()
        invalidate()
    }

    fun reset() {
        replace("")
    }

    fun setTerminalFontSize(sizePx: Double) {
        terminalTextSizePx = logicalPxToPhysicalPx(sizePx)
        renderer = TerminalRenderer(terminalTextSizePx, terminalTypeface)
        requestResize()
        lastCursorMetrics = null
        emitCursorMetrics()
        invalidate()
    }

    fun sendKey(name: String) {
        val keyCode = when (name) {
            "enter" -> "\r"
            "backspace" -> "\u007f"
            "escape" -> "\u001b"
            "tab" -> "\t"
            "arrowLeft" -> KeyHandler.getCode(KeyEvent.KEYCODE_DPAD_LEFT, 0, false, false)
            "arrowRight" -> KeyHandler.getCode(KeyEvent.KEYCODE_DPAD_RIGHT, 0, false, false)
            "arrowUp" -> KeyHandler.getCode(KeyEvent.KEYCODE_DPAD_UP, 0, false, false)
            "arrowDown" -> KeyHandler.getCode(KeyEvent.KEYCODE_DPAD_DOWN, 0, false, false)
            else -> null
        }
        if (!keyCode.isNullOrEmpty()) callbacks.onInput(keyCode)
    }

    fun showKeyboard() {
        requestFocusFromTouch()
        post {
            val inputMethodManager =
                context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            inputMethodManager?.restartInput(this)
            val shown = inputMethodManager?.showSoftInput(
                this,
                InputMethodManager.SHOW_IMPLICIT,
            ) == true
            if (!shown) {
                postDelayed({
                    requestFocusFromTouch()
                    inputMethodManager?.restartInput(this)
                    inputMethodManager?.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT)
                }, 80)
            }
        }
    }

    fun hideKeyboard() {
        post {
            val inputMethodManager =
                context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            inputMethodManager?.hideSoftInputFromWindow(windowToken, 0)
        }
    }

    override fun onCheckIsTextEditor(): Boolean = true

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        outAttrs.inputType =
            InputType.TYPE_CLASS_TEXT or
                InputType.TYPE_TEXT_VARIATION_NORMAL or
                InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        outAttrs.imeOptions =
            EditorInfo.IME_ACTION_NONE or
                EditorInfo.IME_FLAG_NO_EXTRACT_UI or
                EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
        return object : BaseInputConnection(this, true) {
            override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
                if (!text.isNullOrEmpty()) callbacks.onInput(text.toString())
                return true
            }

            override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
                callbacks.onInput("\u007f")
                return true
            }

            override fun sendKeyEvent(event: KeyEvent): Boolean {
                if (event.action == KeyEvent.ACTION_DOWN) {
                    handleKeyEvent(event)
                }
                return true
            }
        }
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        handleKeyEvent(event)
        return true
    }

    private fun handleKeyEvent(event: KeyEvent) {
        when (event.keyCode) {
            KeyEvent.KEYCODE_ENTER -> callbacks.onInput("\r")
            KeyEvent.KEYCODE_DEL -> callbacks.onInput("\u007f")
            KeyEvent.KEYCODE_ESCAPE -> callbacks.onInput("\u001b")
            KeyEvent.KEYCODE_TAB -> callbacks.onInput("\t")
            KeyEvent.KEYCODE_DPAD_LEFT,
            KeyEvent.KEYCODE_DPAD_RIGHT,
            KeyEvent.KEYCODE_DPAD_UP,
            KeyEvent.KEYCODE_DPAD_DOWN -> {
                val code = KeyHandler.getCode(
                    event.keyCode,
                    0,
                    emulator?.isCursorKeysApplicationMode ?: false,
                    emulator?.isKeypadApplicationMode ?: false,
                )
                if (!code.isNullOrEmpty()) callbacks.onInput(code)
            }
            else -> {
                val char = event.unicodeChar
                if (char > 0) callbacks.onInput(String(Character.toChars(char)))
            }
        }
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        requestResize()
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        requestFocus()
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                parent?.requestDisallowInterceptTouchEvent(true)
                scroller.forceFinished(true)
                velocityTracker?.recycle()
                velocityTracker = VelocityTracker.obtain().also { it.addMovement(event) }
                lastTouchY = event.y
                downX = event.x
                downY = event.y
                dragging = false
                activeSelectionHandle = hitSelectionHandle(event.x, event.y)
                selectionDragging = activeSelectionHandle != null
                if (selectionDragging) {
                    removeCallbacks(longPressRunnable)
                } else {
                    clearSelection()
                    postDelayed(longPressRunnable, ViewConfiguration.getLongPressTimeout().toLong())
                }
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (selectionDragging) {
                    updateSelection(event.x, event.y)
                    return true
                }
                velocityTracker?.addMovement(event)
                val dy = event.y - lastTouchY
                if (!dragging && kotlin.math.abs(dy) > touchSlop) {
                    dragging = true
                    removeCallbacks(longPressRunnable)
                }
                if (dragging) {
                    scrollByPixels(-dy)
                    lastTouchY = event.y
                }
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                removeCallbacks(longPressRunnable)
                if (selectionDragging) {
                    updateSelection(event.x, event.y)
                } else {
                    velocityTracker?.apply {
                        addMovement(event)
                        computeCurrentVelocity(1000, maxFlingVelocity.toFloat())
                        fling((-yVelocity).roundToInt())
                        recycle()
                    }
                }
                velocityTracker = null
                dragging = false
                selectionDragging = false
                activeSelectionHandle = null
                parent?.requestDisallowInterceptTouchEvent(false)
                return true
            }
        }
        return true
    }

    override fun computeScroll() {
        if (!scroller.computeScrollOffset()) return
        val dy = scroller.currY - lastScrollerY
        if (dy != 0) {
            lastScrollerY = scroller.currY
            scrollByPixels(dy.toFloat())
        }
        postInvalidateOnAnimation()
    }

    override fun onDraw(canvas: Canvas) {
        val terminal = emulator
        if (terminal == null) {
            canvas.drawColor(TERMINAL_BACKGROUND_COLOR)
            return
        }
        applyTerminalBackground(terminal)
        canvas.drawColor(TERMINAL_BACKGROUND_COLOR)
        val selection = normalizedSelection()
        renderer.render(
            terminal,
            canvas,
            topRow,
            selection?.startRow ?: -1,
            selection?.endRow ?: -1,
            selection?.startCol ?: -1,
            selection?.endCol ?: -1,
        )
        if (selection != null) {
            drawSelectionHandles(canvas, selection)
        }
    }

    private fun requestResize(invalidateView: Boolean = true) {
        if (width <= 0 || height <= 0) return
        val lineSpacing = renderer.fontLineSpacing
        val cols = max(4, (width / renderer.fontWidth).toInt())
        val rows = max(
            4,
            (height - fontLineSpacingAndAscent()) / lineSpacing,
        )
        val sizeChanged = cols != lastCols || rows != lastRows
        if (!sizeChanged && emulator != null) return
        lastCols = cols
        lastRows = rows
        val terminal = emulator
        if (terminal == null) {
            emulator = TerminalEmulator(
                terminalOutput,
                cols,
                rows,
                null,
                this,
            )
            emulator?.let(::applyTerminalBackground)
            applyPendingReplay()
        } else {
            terminal.resize(cols, rows)
            applyTerminalBackground(terminal)
        }
        topRow = 0
        if (sizeChanged) callbacks.onResize(cols, rows)
        emitCursorMetrics()
        if (invalidateView) invalidate()
    }

    private fun ensureEmulator(): TerminalEmulator? {
        if (emulator == null) requestResize()
        return emulator
    }

    private fun applyPendingReplay() {
        val text = pendingReplayText ?: return
        val terminal = emulator ?: return
        pendingReplayText = null
        if (text.isNotEmpty()) {
            appendToTerminal(terminal, text)
            terminal.clearScrollCounter()
        }
        topRow = 0
        clampTopRow()
        emitCursorMetrics()
    }

    private fun appendToTerminal(terminal: TerminalEmulator, text: String) {
        val bytes = text.toByteArray(StandardCharsets.UTF_8)
        terminal.append(bytes, bytes.size)
    }

    private fun emitCursorMetrics() {
        val terminal = emulator ?: return
        val metrics = TerminalCursorMetrics(
            row = terminal.getCursorRow(),
            col = terminal.getCursorCol(),
            lineHeight = renderer.fontLineSpacing.toDouble(),
        )
        if (metrics == lastCursorMetrics) return
        lastCursorMetrics = metrics
        callbacks.onCursorMetrics(metrics.row, metrics.col, metrics.lineHeight)
    }

    private fun clampTopRow() {
        val rows = emulator?.screen?.activeTranscriptRows ?: 0
        topRow = topRow.coerceIn(-rows, 0)
    }

    private fun startSelection(x: Float, y: Float) {
        val terminal = emulator ?: return
        val point = terminalCellAt(x, y) ?: return
        selectionActive = true
        selectionDragging = true
        selectionStartCol = point.col
        selectionStartRow = point.row
        selectionEndCol = point.col
        selectionEndRow = point.row
        activeSelectionHandle = SelectionHandle.End
        callbacks.onSelectionChanged(terminal.getSelectedText(point.col, point.row, point.col, point.row))
        invalidate()
    }

    private fun updateSelection(x: Float, y: Float) {
        if (!selectionActive) return
        val point = terminalCellAt(x, y) ?: return
        when (activeSelectionHandle) {
            SelectionHandle.Start -> {
                selectionStartCol = point.col
                selectionStartRow = point.row
            }
            SelectionHandle.End, null -> {
                selectionEndCol = point.col
                selectionEndRow = point.row
            }
        }
        callbacks.onSelectionChanged(selectedText())
        invalidate()
    }

    private fun clearSelection(invalidateView: Boolean = true) {
        if (!selectionActive) return
        selectionActive = false
        selectionDragging = false
        selectionStartCol = -1
        selectionStartRow = -1
        selectionEndCol = -1
        selectionEndRow = -1
        activeSelectionHandle = null
        callbacks.onSelectionChanged(null)
        if (invalidateView) invalidate()
    }

    private fun selectedText(): String? {
        val terminal = emulator ?: return null
        val selection = normalizedSelection() ?: return null
        return terminal.getSelectedText(
            selection.startCol,
            selection.startRow,
            selection.endCol,
            selection.endRow,
        ).takeIf { it.isNotEmpty() }
    }

    private fun normalizedSelection(): TerminalSelection? {
        if (!selectionActive || selectionStartRow < 0 || selectionEndRow < 0) return null
        val startsBeforeEnd =
            selectionStartRow < selectionEndRow ||
                (selectionStartRow == selectionEndRow && selectionStartCol <= selectionEndCol)
        return if (startsBeforeEnd) {
            TerminalSelection(selectionStartCol, selectionStartRow, selectionEndCol, selectionEndRow)
        } else {
            TerminalSelection(selectionEndCol, selectionEndRow, selectionStartCol, selectionStartRow)
        }
    }

    private fun terminalCellAt(x: Float, y: Float): TerminalCell? {
        if (lastCols <= 0 || lastRows <= 0) return null
        val col = (x / renderer.fontWidth).toInt().coerceIn(0, lastCols - 1)
        val row = topRow + (y / renderer.fontLineSpacing).toInt().coerceIn(0, lastRows - 1)
        return TerminalCell(col, row)
    }

    private fun hitSelectionHandle(x: Float, y: Float): SelectionHandle? {
        val selection = normalizedSelection() ?: return null
        val radius = selectionHandleRadius()
        val start = selectionHandleAnchor(selection.startCol, selection.startRow, SelectionHandle.Start)
        val end = selectionHandleAnchor(selection.endCol, selection.endRow, SelectionHandle.End)
        return when {
            end != null && distanceSquared(x, y, end.x, end.y) <= radius * radius * 2.25f ->
                SelectionHandle.End
            start != null && distanceSquared(x, y, start.x, start.y) <= radius * radius * 2.25f ->
                SelectionHandle.Start
            else -> null
        }
    }

    private fun drawSelectionHandles(canvas: Canvas, selection: TerminalSelection) {
        drawSelectionHandle(canvas, selection.startCol, selection.startRow, SelectionHandle.Start)
        drawSelectionHandle(canvas, selection.endCol, selection.endRow, SelectionHandle.End)
    }

    private fun drawSelectionHandle(
        canvas: Canvas,
        col: Int,
        row: Int,
        handle: SelectionHandle,
    ) {
        val anchor = selectionHandleAnchor(col, row, handle) ?: return
        val drawable = when (handle) {
            SelectionHandle.Start -> selectionStartHandle
            SelectionHandle.End -> selectionEndHandle
        } ?: return
        val width = selectionHandleWidth()
        val height = selectionHandleHeight()
        val left = when (handle) {
            SelectionHandle.Start -> (anchor.x - width).roundToInt()
            SelectionHandle.End -> anchor.x.roundToInt()
        }
        val top = when (handle) {
            SelectionHandle.Start -> (anchor.y - height).roundToInt()
            SelectionHandle.End -> anchor.y.roundToInt()
        }
        drawable.setBounds(left, top, left + width, top + height)
        drawable.draw(canvas)
    }

    private fun selectionHandleAnchor(
        col: Int,
        row: Int,
        handle: SelectionHandle,
    ): TerminalPoint? {
        val visibleRow = row - topRow
        if (visibleRow < 0 || visibleRow >= lastRows) return null
        val x = when (handle) {
            SelectionHandle.Start -> (col + 1) * renderer.fontWidth
            SelectionHandle.End -> col * renderer.fontWidth
        }.coerceIn(0f, width.toFloat())
        val handleHeight = selectionHandleHeight().toFloat()
        val y = when (handle) {
            SelectionHandle.Start -> (visibleRow + 1) * renderer.fontLineSpacing + handleHeight * 0.12f
            SelectionHandle.End -> (visibleRow + 1) * renderer.fontLineSpacing - handleHeight * 0.88f
        }.coerceIn(0f, height.toFloat())
        return TerminalPoint(x, y)
    }

    private fun selectionHandleRadius(): Float {
        return max(18f * resources.displayMetrics.density, renderer.fontLineSpacing * 0.5f)
    }

    private fun selectionHandleWidth(): Int {
        return max(28f * resources.displayMetrics.density, renderer.fontLineSpacing * 0.9f)
            .roundToInt()
    }

    private fun selectionHandleHeight(): Int {
        return max(14f * resources.displayMetrics.density, renderer.fontLineSpacing * 0.45f)
            .roundToInt()
    }

    private fun distanceSquared(x1: Float, y1: Float, x2: Float, y2: Float): Float {
        val dx = x1 - x2
        val dy = y1 - y2
        return dx * dx + dy * dy
    }

    private fun scrollByPixels(delta: Float) {
        val terminal = emulator ?: return
        scrollRemainder += delta / renderer.fontLineSpacing
        val rows = scrollRemainder.toInt()
        if (rows == 0) return
        scrollRemainder -= rows
        val minTop = -terminal.screen.activeTranscriptRows
        val next = (topRow + rows).coerceIn(minTop, 0)
        if (next == topRow) {
            scroller.forceFinished(true)
            return
        }
        topRow = next
        invalidate()
    }

    private fun fling(velocityY: Int) {
        val terminal = emulator ?: return
        val minY = -terminal.screen.activeTranscriptRows * renderer.fontLineSpacing
        val startY = topRow * renderer.fontLineSpacing
        lastScrollerY = startY
        scroller.fling(
            0,
            startY,
            0,
            velocityY,
            0,
            0,
            minY,
            0,
        )
        postInvalidateOnAnimation()
    }

    private fun logicalPxToPhysicalPx(value: Double): Int {
        return (value * resources.displayMetrics.density).roundToInt().coerceAtLeast(1)
    }

    private fun fontLineSpacingAndAscent(): Int {
        val paint = Paint()
        paint.typeface = terminalTypeface
        paint.isAntiAlias = true
        paint.textSize = terminalTextSizePx.toFloat()
        return renderer.fontLineSpacing + kotlin.math.ceil(paint.ascent().toDouble()).toInt()
    }

    override fun onTerminalCursorStateChange(state: Boolean) = invalidate()
    override fun getTerminalCursorStyle(): Int? = TerminalEmulator.TERMINAL_CURSOR_STYLE_BLOCK
    override fun onTextChanged(changedSession: TerminalSession?) {
        emitCursorMetrics()
        invalidate()
    }
    override fun onTitleChanged(changedSession: TerminalSession?) = Unit
    override fun onSessionFinished(finishedSession: TerminalSession?) = Unit
    override fun onCopyTextToClipboard(session: TerminalSession?, text: String?) = Unit
    override fun onPasteTextFromClipboard(session: TerminalSession?) = Unit
    override fun onBell(session: TerminalSession?) = Unit
    override fun onColorsChanged(session: TerminalSession?) = invalidate()
    override fun logError(tag: String?, message: String?) = Unit
    override fun logWarn(tag: String?, message: String?) = Unit
    override fun logInfo(tag: String?, message: String?) = Unit
    override fun logDebug(tag: String?, message: String?) = Unit
    override fun logVerbose(tag: String?, message: String?) = Unit
    override fun logStackTraceWithMessage(tag: String?, message: String?, e: Exception?) = Unit
    override fun logStackTrace(tag: String?, e: Exception?) = Unit
}

private const val TERMINAL_FONT_ASSET = "flutter_assets/assets/fonts/MapleMono-NF-CN-Regular.ttf"
private const val TERMINAL_BACKGROUND_COLOR = -15920873

private fun chooseNativeTerminalTypeface(context: Context): Typeface {
    return runCatching {
        Typeface.Builder(context.assets, TERMINAL_FONT_ASSET).build()
    }.getOrElse {
        Typeface.create(Typeface.MONOSPACE, Typeface.NORMAL)
    }
}

private fun loadTermuxSelectionHandle(context: Context, name: String): Drawable? {
    val id = context.resources.getIdentifier(
        name,
        "drawable",
        context.packageName,
    )
    if (id == 0) return null
    return context.getDrawable(id)
}

private fun applyTerminalBackground(terminal: TerminalEmulator) {
    terminal.mColors.mCurrentColors[TextStyle.COLOR_INDEX_BACKGROUND] = TERMINAL_BACKGROUND_COLOR
}

private data class TerminalCell(val col: Int, val row: Int)

private data class TerminalPoint(val x: Float, val y: Float)

private data class TerminalCursorMetrics(
    val row: Int,
    val col: Int,
    val lineHeight: Double,
)

private data class TerminalSelection(
    val startCol: Int,
    val startRow: Int,
    val endCol: Int,
    val endRow: Int,
)

private enum class SelectionHandle {
    Start,
    End,
}
