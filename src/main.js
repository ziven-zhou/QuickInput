import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { normalizeContentMode } from "./content-mode.js";
import { applyInputPreferences } from "./input-preferences.js";
import { hotkeyMatchesEvent, initializePageShortcuts } from "./page-shortcuts.js";
import { confirmationActionForEvent, confirmationFailureFeedback } from "./confirmation-shortcuts.js";
import { renderMarkdown } from "./markdown.js";

const appElement = document.getElementById("app");
const textarea = document.getElementById("input-area");
const charCount = document.getElementById("char-count");
const modeSelect = document.getElementById("mode-select");
const modeHint = document.getElementById("mode-hint");
const previewArea = document.getElementById("preview-area");
const clearBtn = document.getElementById("clear-btn");
const confirmBtn = document.getElementById("confirm-btn");
const cancelBtn = document.getElementById("cancel-btn");
const settingsBtn = document.getElementById("settings-btn");
const contentBtn = document.getElementById("content-btn");

const MODE_HINTS = {
  plain: "纯文本粘贴",
  markdown: "实时预览，渲染后粘贴",
};

let isWorking = false;
let confirmHotkey = "Alt+Enter";
let cancelHotkey = "Escape";
let defaultContentMode = "plain";
let pasteAndSendEnabled = false;
let messageTimer = null;
let previewTimer = null;
let nextPasteAttemptId = 1;
let lastPasteAttempt = null;

function formatHotkeyDisplay(hotkey) {
  return hotkey
    .replace("CmdOrCtrl", "⌘/Ctrl")
    .replace("Cmd", "⌘")
    .replace("Ctrl", "Ctrl")
    .replace("Alt", "⌥")
    .replace("Shift", "⇧")
    .replace("Escape", "Esc")
    .replace("Enter", "↵");
}

function renderShortcutHints(settings) {
  const values = {
    trigger: settings?.trigger_hotkey || "Alt+Space",
    confirm: settings?.confirm_hotkey || "Alt+Enter",
    cancel: settings?.cancel_hotkey || "Escape",
    content: settings?.content_hotkey || "Alt+R",
    settings: settings?.settings_hotkey || "Alt+S",
  };
  for (const [key, hotkey] of Object.entries(values)) {
    const el = document.querySelector(`[data-hotkey="${key}"]`);
    if (el) el.textContent = formatHotkeyDisplay(hotkey);
  }
  document.getElementById("send-shortcut-hint").hidden = !pasteAndSendEnabled;
  textarea.placeholder = pasteAndSendEnabled
    ? `输入内容，${formatHotkeyDisplay(values.confirm)} 仅粘贴，⇧+↵ 粘贴并发送，Enter 换行…`
    : `输入内容，${formatHotkeyDisplay(values.confirm)} 仅粘贴，Enter 换行…`;
}

function updateCharCount() {
  charCount.textContent = `${textarea.value.length} 字符`;
}

function updateMode() {
  const mode = normalizeContentMode(modeSelect.value);
  modeSelect.value = mode;
  modeHint.textContent = MODE_HINTS[mode] || "";
  previewArea.classList.toggle("hidden", mode !== "markdown");
  const editorArea = previewArea.parentElement;
  editorArea.classList.toggle("markdown-mode", mode === "markdown");
  if (mode === "markdown") {
    previewArea.innerHTML = renderMarkdown(textarea.value);
  }
}

function resetInputForNextOpen() {
  clearTimeout(previewTimer);
  textarea.value = "";
  modeSelect.value = defaultContentMode;
  updateCharCount();
  updateMode();
}

async function confirmInput(action = "paste") {
  if (isWorking) return;
  if (!textarea.value.trim()) {
    showMessage("内容为空，无法粘贴", "error");
    textarea.focus();
    return;
  }
  isWorking = true;
  confirmBtn.disabled = true;

  const text = textarea.value;
  const mode = normalizeContentMode(modeSelect.value);
  const attemptId = nextPasteAttemptId++;
  const renderedHtml = mode === "markdown" ? renderMarkdown(text) : null;
  const appWindow = getCurrentWindow();
  const attempt = { attemptId, text, mode, failed: false };

  try {
    // Retain the local attempt even if an event arrives before invoke resolves.
    lastPasteAttempt = attempt;
    await invoke("confirm_input", { text, mode, renderedHtml, attemptId, action });
    if (!attempt.failed) resetInputForNextOpen();
  } catch (err) {
    lastPasteAttempt = null;
    isWorking = false;
    confirmBtn.disabled = false;
    console.error("confirm failed:", err);
    await appWindow.show().catch(() => {});
    await appWindow.setFocus().catch(() => {});
    showMessage("确认失败: " + err, "error");
  }
}

async function cancelInput() {
  if (isWorking) return;
  isWorking = true;
  try {
    await invoke("cancel_input", {
      text: textarea.value,
      mode: normalizeContentMode(modeSelect.value),
    });
    lastPasteAttempt = null;
    resetInputForNextOpen();
    isWorking = false;
  } catch (err) {
    console.error("cancel failed:", err);
    showMessage("取消失败: " + err, "error");
    isWorking = false;
  }
}

function clearCurrent() {
  textarea.value = "";
  updateCharCount();
  updateMode();
  textarea.focus();
}

async function openSettings() {
  try { await invoke("open_settings"); } catch (err) { console.error("open settings failed:", err); }
}

async function openContent() {
  try { await invoke("open_content"); } catch (err) { console.error("open content failed:", err); }
}

textarea.addEventListener("input", () => {
  updateCharCount();
  if (modeSelect.value === "markdown") {
    clearTimeout(previewTimer);
    previewTimer = setTimeout(() => {
      previewArea.innerHTML = renderMarkdown(textarea.value);
    }, 150);
  }
});

document.addEventListener("keydown", (e) => {
  if (e.isComposing || e.keyCode === 229) return;
  const action = confirmationActionForEvent(e, {
    confirm_hotkey: confirmHotkey,
    paste_and_send_enabled: pasteAndSendEnabled,
  });
  const isConfirmChord = hotkeyMatchesEvent(confirmHotkey, e)
    || (pasteAndSendEnabled && hotkeyMatchesEvent("Shift+Enter", e));
  if (isConfirmChord) {
    e.preventDefault();
    e.stopImmediatePropagation();
    if (action) confirmInput(action);
  } else if (hotkeyMatchesEvent(cancelHotkey, e)) { e.preventDefault(); cancelInput(); }
  else if (hotkeyMatchesEvent("Alt+Backspace", e)) { e.preventDefault(); clearCurrent(); }
}, true);

modeSelect.addEventListener("change", updateMode);
clearBtn.addEventListener("click", clearCurrent);
confirmBtn.addEventListener("click", () => confirmInput("paste"));
cancelBtn.addEventListener("click", cancelInput);
settingsBtn.addEventListener("click", openSettings);
contentBtn?.addEventListener("click", openContent);

async function init() {
  try {
    const settings = await invoke("get_settings");
    defaultContentMode = normalizeContentMode(settings?.default_mode);
    modeSelect.value = defaultContentMode;
    if (settings?.confirm_hotkey) confirmHotkey = settings.confirm_hotkey;
    if (settings?.cancel_hotkey) cancelHotkey = settings.cancel_hotkey;
    pasteAndSendEnabled = settings?.paste_and_send_enabled === true;
    applyInputPreferences(appElement, document.documentElement, settings);
    renderShortcutHints(settings);
    if (settings?.bg_opacity != null) {
      document.documentElement.style.setProperty("--bg-opacity", String(settings.bg_opacity));
    }
  } catch (err) { console.error("load settings failed:", err); }
  updateMode();
  updateCharCount();
  const appWindow = getCurrentWindow();
  appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      setTimeout(() => textarea.focus(), 80);
    }
  });
  listen("paste-error", (event) => {
    const payload = event.payload;
    const attemptId = payload && typeof payload === "object" ? payload.attempt_id : null;
    const message = payload && typeof payload === "object" ? payload.message : payload;
    if (attemptId != null && attemptId !== lastPasteAttempt?.attemptId) {
      return;
    }
    if (lastPasteAttempt) lastPasteAttempt.failed = true;
    const feedback = confirmationFailureFeedback(
      payload && typeof payload === "object" ? payload : { message },
    );
    if (lastPasteAttempt && feedback.restoreInput) {
      textarea.value = lastPasteAttempt.text;
      modeSelect.value = lastPasteAttempt.mode;
      updateCharCount();
      updateMode();
    }
    showMessage(feedback.message, "error");
  });
  listen("paste-complete", ({ payload }) => {
    if (payload.attempt_id !== lastPasteAttempt?.attemptId) return;
    lastPasteAttempt = null;
    isWorking = false;
    confirmBtn.disabled = false;
  });
  listen("page-shortcuts-updated", async () => {
    try {
      const settings = await invoke("get_settings");
      confirmHotkey = settings?.confirm_hotkey || confirmHotkey;
      cancelHotkey = settings?.cancel_hotkey || cancelHotkey;
      defaultContentMode = normalizeContentMode(settings?.default_mode);
      modeSelect.value = defaultContentMode;
      updateMode();
      pasteAndSendEnabled = settings?.paste_and_send_enabled === true;
      applyInputPreferences(appElement, document.documentElement, settings);
      renderShortcutHints(settings);
      if (settings?.bg_opacity != null) {
        document.documentElement.style.setProperty("--bg-opacity", String(settings.bg_opacity));
      }
    } catch (err) { console.error("refresh settings failed:", err); }
  });
  initializePageShortcuts();
  textarea.focus();
}

const statusMsg = document.getElementById("status-msg");
function showMessage(msg, type = "success") {
  if (messageTimer) clearTimeout(messageTimer);
  statusMsg.textContent = msg;
  statusMsg.classList.toggle("error", type === "error");
  statusMsg.classList.remove("hidden");
  messageTimer = setTimeout(() => {
    statusMsg.classList.add("hidden");
    statusMsg.classList.remove("error");
    messageTimer = null;
  }, 4000);
}

const resizeHandle = document.querySelector(".resize-handle");
if (resizeHandle) {
  resizeHandle.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    getCurrentWindow().startResizeDragging("SouthEast").catch(() => {});
  });
}

const dragRegion = document.getElementById("drag-region");
if (dragRegion) {
  dragRegion.addEventListener("mousedown", (e) => {
    if (e.target.closest("button, select, input, textarea, a, label")) return;
    getCurrentWindow().startDragging().catch(() => {});
  });
}

init();
