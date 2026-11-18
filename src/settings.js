import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { normalizeContentMode } from "./content-mode.js";
import { normalizeInputFontSize } from "./input-preferences.js";
import { hotkeyFromEvent, initializePageShortcuts } from "./page-shortcuts.js";
import {
  DEFAULT_WINDOW_HEIGHT,
  DEFAULT_WINDOW_WIDTH,
  normalizeWindowHeight,
  normalizeWindowWidth,
} from "./window-preferences.js";

const closeBtn = document.getElementById("close-settings-btn");
const autosaveState = document.getElementById("autosave-state");
const toast = document.getElementById("toast");
const positionSelect = document.getElementById("position-select");
const defaultModeSelect = document.getElementById("default-mode-select");
const restoreClipboard = document.getElementById("restore-clipboard-checkbox");
const autostart = document.getElementById("autostart-checkbox");
const pasteAndSendCheckbox = document.getElementById("paste-and-send-checkbox");
const keepOpenCheckbox = document.getElementById("keep-open-checkbox");
const pureModeCheckbox = document.getElementById("pure-mode-checkbox");
const rememberPositionCheckbox = document.getElementById("remember-position-checkbox");
const windowWidthInput = document.getElementById("window-width-input");
const windowHeightInput = document.getElementById("window-height-input");
const opacitySlider = document.getElementById("opacity-slider");
const opacityValue = document.getElementById("opacity-value");
const inputFontSizeSlider = document.getElementById("input-font-size-slider");
const inputFontSizeValue = document.getElementById("input-font-size-value");
const inputFontSizePreview = document.getElementById("input-font-size-preview");
const recorders = {
  trigger: document.getElementById("hotkey-trigger"),
  content: document.getElementById("hotkey-content"),
  settings: document.getElementById("hotkey-settings"),
  confirm: document.getElementById("hotkey-confirm"),
  cancel: document.getElementById("hotkey-cancel"),
};

const DEFAULT_HOTKEYS = {
  trigger: "Alt+Space",
  content: "Alt+R",
  settings: "Alt+S",
  confirm: "Alt+Enter",
  cancel: "Escape",
};

let settings = {
  trigger_hotkey: DEFAULT_HOTKEYS.trigger,
  content_hotkey: DEFAULT_HOTKEYS.content,
  settings_hotkey: DEFAULT_HOTKEYS.settings,
  confirm_hotkey: DEFAULT_HOTKEYS.confirm,
  cancel_hotkey: DEFAULT_HOTKEYS.cancel,
  window_position: "center",
  default_mode: "plain",
  pure_mode: false,
  input_font_size: 14,
  restore_clipboard: true,
  autostart: false,
  keep_open_on_confirm: false,
  paste_and_send_enabled: false,
  remember_window_position: false,
  window_x: null,
  window_y: null,
  window_width: DEFAULT_WINDOW_WIDTH,
  window_height: DEFAULT_WINDOW_HEIGHT,
  bg_opacity: 0.97,
};

let recordingKey = null;
let saveTimer = null;
let saveChain = Promise.resolve();
let suppressAutosave = false;
let autosaveStateTimer = null;
let toastTimer = null;
let closeInProgress = false;

function formatHotkeyDisplay(hotkey) {
  return hotkey
    .replace("CmdOrCtrl", "⌘/Ctrl")
    .replace("Cmd", "⌘")
    .replace("Alt", "⌥")
    .replace("Shift", "⇧")
    .replace("Escape", "Esc")
    .replace("Enter", "↵");
}

function setAutosaveState(message, isError = false) {
  clearTimeout(autosaveStateTimer);
  autosaveState.textContent = message;
  autosaveState.classList.toggle("error", isError);
  if (message === "已生效") {
    autosaveStateTimer = setTimeout(() => {
      autosaveState.textContent = "";
      autosaveState.classList.remove("error");
    }, 1200);
  }
}

function showToast(message, type = "success") {
  clearTimeout(toastTimer);
  toast.textContent = message;
  toast.classList.toggle("error", type === "error");
  toast.classList.remove("hidden");
  toastTimer = setTimeout(() => {
    toast.classList.add("hidden");
    toast.classList.remove("error");
  }, 2800);
}

function cancelRecording() {
  if (!recordingKey) return;
  const button = recorders[recordingKey];
  button.classList.remove("recording");
  button.textContent = formatHotkeyDisplay(
    button.dataset.hotkey || DEFAULT_HOTKEYS[recordingKey]
  );
  recordingKey = null;
}

function renderHotkeys() {
  for (const [key, button] of Object.entries(recorders)) {
    const hotkey = settings[`${key}_hotkey`] || DEFAULT_HOTKEYS[key];
    button.dataset.hotkey = hotkey;
    button.textContent = formatHotkeyDisplay(hotkey);
  }
}

function applyOpacity(pct) {
  document.documentElement.style.setProperty("--bg-opacity", String(pct / 100));
}

function applyInputFontSizePreview(value) {
  const size = normalizeInputFontSize(value);
  inputFontSizeSlider.value = String(size);
  inputFontSizeValue.textContent = `${size}px`;
  inputFontSizePreview.style.fontSize = `${size}px`;
  return size;
}

function collectSettings() {
  return {
    ...settings,
    trigger_hotkey: recorders.trigger.dataset.hotkey || DEFAULT_HOTKEYS.trigger,
    content_hotkey: recorders.content.dataset.hotkey || DEFAULT_HOTKEYS.content,
    settings_hotkey: recorders.settings.dataset.hotkey || DEFAULT_HOTKEYS.settings,
    confirm_hotkey: recorders.confirm.dataset.hotkey || DEFAULT_HOTKEYS.confirm,
    cancel_hotkey: recorders.cancel.dataset.hotkey || DEFAULT_HOTKEYS.cancel,
    window_position: positionSelect.value,
    default_mode: normalizeContentMode(defaultModeSelect.value),
    pure_mode: pureModeCheckbox.checked,
    input_font_size: normalizeInputFontSize(inputFontSizeSlider.value),
    restore_clipboard: restoreClipboard.checked,
    autostart: autostart.checked,
    keep_open_on_confirm: keepOpenCheckbox.checked,
    paste_and_send_enabled: pasteAndSendCheckbox.checked,
    remember_window_position: rememberPositionCheckbox.checked,
    window_width: normalizeWindowWidth(windowWidthInput.value),
    window_height: normalizeWindowHeight(windowHeightInput.value),
    bg_opacity: parseInt(opacitySlider.value, 10) / 100,
  };
}

async function persistCurrentSettings() {
  if (suppressAutosave) return;
  setAutosaveState("保存中…");
  const updated = collectSettings();
  try {
    await invoke("save_settings", { settings: updated });
    settings = updated;
    windowWidthInput.value = settings.window_width;
    windowHeightInput.value = settings.window_height;
    setAutosaveState("已生效");
  } catch (err) {
    console.error("save settings failed:", err);
    setAutosaveState("保存失败", true);
    showToast(String(err), "error");
    await loadSettings();
  }
}

function enqueueSave() {
  saveChain = saveChain.then(persistCurrentSettings, persistCurrentSettings);
  return saveChain;
}

function scheduleSave(delay = 180) {
  if (suppressAutosave) return;
  clearTimeout(saveTimer);
  setAutosaveState("待保存");
  saveTimer = setTimeout(() => {
    saveTimer = null;
    enqueueSave();
  }, delay);
}

async function loadSettings() {
  suppressAutosave = true;
  try {
    const loaded = await invoke("get_settings");
    if (loaded) settings = { ...settings, ...loaded };
  } catch (err) {
    console.error("load settings failed:", err);
    showToast("读取设置失败", "error");
  } finally {
    suppressAutosave = false;
  }

  positionSelect.value = settings.window_position || "center";
  defaultModeSelect.value = normalizeContentMode(settings.default_mode);
  pureModeCheckbox.checked = !!settings.pure_mode;
  restoreClipboard.checked = settings.restore_clipboard !== false;
  autostart.checked = !!settings.autostart;
  keepOpenCheckbox.checked = !!settings.keep_open_on_confirm;
  pasteAndSendCheckbox.checked = settings.paste_and_send_enabled === true;
  rememberPositionCheckbox.checked = !!settings.remember_window_position;
  windowWidthInput.value = normalizeWindowWidth(settings.window_width);
  windowHeightInput.value = normalizeWindowHeight(settings.window_height);
  const opacityPct = Math.round((settings.bg_opacity ?? 0.97) * 100);
  opacitySlider.value = opacityPct;
  opacityValue.textContent = `${opacityPct}%`;
  applyOpacity(opacityPct);
  applyInputFontSizePreview(settings.input_font_size);
  renderHotkeys();
}

function closeSettings() {
  if (closeInProgress) return;
  closeInProgress = true;
  cancelRecording();
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = null;
    enqueueSave();
  }
  getCurrentWindow()
    .hide()
    .catch((err) => console.error("hide settings failed:", err))
    .finally(() => {
      closeInProgress = false;
    });
}

for (const [key, button] of Object.entries(recorders)) {
  button.addEventListener("click", () => {
    cancelRecording();
    recordingKey = key;
    button.classList.add("recording");
    button.textContent = "按下快捷键...";
  });
}

window.addEventListener("keydown", (event) => {
  if (!recordingKey) return;
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    cancelRecording();
    return;
  }

  const combo = hotkeyFromEvent(event);
  if (!combo) return;

  const normalized = combo
    .replace("Cmd+Ctrl", "CmdOrCtrl")
    .replace("Ctrl+Cmd", "CmdOrCtrl");
  recorders[recordingKey].dataset.hotkey = normalized;
  recorders[recordingKey].classList.remove("recording");
  recorders[recordingKey].textContent = formatHotkeyDisplay(normalized);
  recordingKey = null;
  scheduleSave(0);
});

document.addEventListener("pointerdown", (event) => {
  if (recordingKey && !event.target.closest(".hotkey-recorder")) {
    cancelRecording();
  }
});

opacitySlider.addEventListener("input", () => {
  const pct = parseInt(opacitySlider.value, 10);
  opacityValue.textContent = `${pct}%`;
  applyOpacity(pct);
  scheduleSave(220);
});

inputFontSizeSlider.addEventListener("input", () => {
  applyInputFontSizePreview(inputFontSizeSlider.value);
  scheduleSave(220);
});

for (const select of [positionSelect, defaultModeSelect]) {
  select.addEventListener("change", () => scheduleSave(0));
}

for (const checkbox of [
  restoreClipboard,
  autostart,
  keepOpenCheckbox,
  pasteAndSendCheckbox,
  pureModeCheckbox,
  rememberPositionCheckbox,
]) {
  checkbox.addEventListener("change", () => scheduleSave(0));
}

for (const input of [windowWidthInput, windowHeightInput]) {
  input.addEventListener("change", () => scheduleSave(0));
}

for (const slider of [opacitySlider, inputFontSizeSlider]) {
  slider.addEventListener("change", () => scheduleSave(0));
}

closeBtn.addEventListener("click", closeSettings);
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || recordingKey) return;
  event.preventDefault();
  closeSettings();
});

loadSettings();
initializePageShortcuts();

const dragRegion = document.getElementById("drag-region");
if (dragRegion) {
  dragRegion.addEventListener("mousedown", (event) => {
    if (event.target.closest("button, select, input, textarea, a, label")) return;
    getCurrentWindow().startDragging().catch(() => {});
  });
}
