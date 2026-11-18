import { hotkeyMatchesEvent } from "./page-shortcuts.js";

// Only the explicitly enabled Shift+Enter action may append a Return key.
export function confirmationActionForEvent(event, settings = {}) {
  if (event.isComposing || event.keyCode === 229 || event.repeat) return null;
  if (hotkeyMatchesEvent(settings.confirm_hotkey || "Alt+Enter", event)) return "paste";
  if (settings.paste_and_send_enabled === true && hotkeyMatchesEvent("Shift+Enter", event)) {
    return "paste_and_enter";
  }
  return null;
}

export function confirmationFailureFeedback(payload) {
  if (payload.pasted === true) {
    return {
      restoreInput: false,
      message: `已尝试粘贴，未完成回车：${payload.message}。请检查目标窗口，不要重复粘贴。`,
    };
  }
  return { restoreInput: true, message: `粘贴失败: ${payload.message}` };
}
