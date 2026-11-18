import test from "node:test";
import assert from "node:assert/strict";

import {
  applyInputPreferences,
  DEFAULT_INPUT_FONT_SIZE,
  normalizeInputFontSize,
} from "./input-preferences.js";

test("normalizes valid input font sizes", () => {
  assert.equal(normalizeInputFontSize(12), 12);
  assert.equal(normalizeInputFontSize(24), 24);
  assert.equal(normalizeInputFontSize(32), 32);
});

test("falls back for invalid input font sizes", () => {
  assert.equal(normalizeInputFontSize(undefined), DEFAULT_INPUT_FONT_SIZE);
  assert.equal(normalizeInputFontSize(11), DEFAULT_INPUT_FONT_SIZE);
  assert.equal(normalizeInputFontSize(33), DEFAULT_INPUT_FONT_SIZE);
  assert.equal(normalizeInputFontSize("large"), DEFAULT_INPUT_FONT_SIZE);
});

test("applies pure mode and font size to the main window", () => {
  const classes = new Set();
  const appElement = {
    classList: {
      toggle(name, enabled) {
        if (enabled) classes.add(name);
        else classes.delete(name);
      },
    },
  };
  const properties = new Map();
  const rootElement = {
    style: {
      setProperty(name, value) {
        properties.set(name, value);
      },
    },
  };

  const result = applyInputPreferences(appElement, rootElement, {
    pure_mode: true,
    input_font_size: 24,
  });

  assert.equal(result.pureMode, true);
  assert.equal(result.fontSize, 24);
  assert.equal(classes.has("pure-mode"), true);
  assert.equal(properties.get("--input-font-size"), "24px");
});

test("uses safe defaults when preferences are missing", () => {
  const classes = new Set(["pure-mode"]);
  const appElement = {
    classList: {
      toggle(name, enabled) {
        if (enabled) classes.add(name);
        else classes.delete(name);
      },
    },
  };
  const properties = new Map();
  const rootElement = {
    style: {
      setProperty(name, value) {
        properties.set(name, value);
      },
    },
  };

  applyInputPreferences(appElement, rootElement, {});

  assert.equal(classes.has("pure-mode"), false);
  assert.equal(properties.get("--input-font-size"), "14px");
});
