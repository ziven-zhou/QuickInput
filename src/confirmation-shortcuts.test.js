import test from "node:test";
import assert from "node:assert/strict";
import { confirmationActionForEvent, confirmationFailureFeedback } from "./confirmation-shortcuts.js";

const key = (overrides = {}) => ({ key: "Enter", code: "Enter", altKey: false,
  shiftKey: false, ctrlKey: false, metaKey: false, repeat: false, isComposing: false, ...overrides });
const settings = { confirm_hotkey: "Alt+Enter", paste_and_send_enabled: true };

test("Option+Enter always pastes, even with sending enabled", () => {
  assert.equal(confirmationActionForEvent(key({ altKey: true }), settings), "paste");
  assert.equal(confirmationActionForEvent(key({ altKey: true }), {}), "paste");
});
test("Shift+Enter sends only when explicitly enabled", () => {
  assert.equal(confirmationActionForEvent(key({ shiftKey: true }), settings), "paste_and_enter");
  assert.equal(confirmationActionForEvent(key({ shiftKey: true }), {}), null);
  assert.equal(confirmationActionForEvent(key({ shiftKey: true }), { paste_and_send_enabled: false }), null);
});
test("Enter and extra modifiers do not send", () => {
  for (const overrides of [{}, { shiftKey: true, altKey: true }, { shiftKey: true, ctrlKey: true }, { shiftKey: true, metaKey: true }]) {
    assert.equal(confirmationActionForEvent(key(overrides), settings), null);
  }
});
test("IME composition, legacy composition keyCode and repeats never confirm", () => {
  for (const overrides of [{ isComposing: true }, { keyCode: 229 }, { repeat: true }]) {
    for (const modifier of [{ altKey: true }, { shiftKey: true }]) {
      assert.equal(confirmationActionForEvent(key({ ...overrides, ...modifier }), settings), null);
    }
  }
});
test("custom confirm shortcut remains paste-only", () => {
  assert.equal(confirmationActionForEvent(key({ ctrlKey: true }), { ...settings, confirm_hotkey: "Ctrl+Enter" }), "paste");
  assert.equal(confirmationActionForEvent(key({ altKey: true }), { ...settings, confirm_hotkey: "Ctrl+Enter" }), null);
});
test("post-paste failure never restores text for an accidental resend", () => {
  assert.deepEqual(confirmationFailureFeedback({ pasted: true, message: "焦点变化" }), {
    restoreInput: false, message: "已尝试粘贴，未完成回车：焦点变化。请检查目标窗口，不要重复粘贴。",
  });
  assert.deepEqual(confirmationFailureFeedback({ pasted: false, message: "焦点变化" }), {
    restoreInput: true, message: "粘贴失败: 焦点变化",
  });
});
