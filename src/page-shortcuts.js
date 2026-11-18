import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

let contentHotkey = "Alt+R";
let settingsHotkey = "Alt+S";

function normalizeEventKey(event) {
  if (event.code?.startsWith("Key")) return event.code.slice(3);
  if (event.code?.startsWith("Digit")) return event.code.slice(5);

  const keyMap = {
    " ": "Space",
    Enter: "Enter",
    Escape: "Escape",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };

  return keyMap[event.key] || (event.key?.length === 1 ? event.key.toUpperCase() : event.key);
}

export function hotkeyFromEvent(event) {
  if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) return null;

  const parts = [];
  if (event.metaKey) parts.push("Cmd");
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");

  const key = normalizeEventKey(event);
  if (!key) return null;

  parts.push(key);
  return parts.join("+");
}

export function hotkeyMatchesEvent(hotkey, event) {
  const parts = hotkey.split("+");
  const has = (name) => parts.includes(name) || (name === "Cmd" && parts.includes("CmdOrCtrl"));
  const key = parts[parts.length - 1];

  return (
    event.metaKey === has("Cmd") &&
    event.ctrlKey === has("Ctrl") &&
    event.altKey === has("Alt") &&
    event.shiftKey === has("Shift") &&
    normalizeEventKey(event) === key
  );
}

function isRecordingHotkey() {
  const active = document.activeElement;
  return active instanceof HTMLElement && active.closest(".hotkey-recorder") !== null;
}

function refreshPageShortcuts() {
  invoke("get_settings")
    .then((settings) => {
      contentHotkey = settings?.content_hotkey || contentHotkey;
      settingsHotkey = settings?.settings_hotkey || settingsHotkey;
    })
    .catch((err) => console.error("load page shortcuts failed:", err));
}

export function initializePageShortcuts() {
  window.addEventListener("keydown", (event) => {
    if (isRecordingHotkey()) return;

    if (hotkeyMatchesEvent(contentHotkey, event)) {
      event.preventDefault();
      invoke("open_content").catch((err) => console.error("open content failed:", err));
    } else if (hotkeyMatchesEvent(settingsHotkey, event)) {
      event.preventDefault();
      invoke("open_settings").catch((err) => console.error("open settings failed:", err));
    }
  });

  refreshPageShortcuts();
  listen("page-shortcuts-updated", refreshPageShortcuts);
}
