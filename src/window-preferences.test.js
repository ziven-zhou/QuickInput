import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULT_WINDOW_HEIGHT,
  DEFAULT_WINDOW_WIDTH,
  normalizeWindowHeight,
  normalizeWindowWidth,
} from "./window-preferences.js";

test("keeps valid window dimensions", () => {
  assert.equal(normalizeWindowWidth(400), 400);
  assert.equal(normalizeWindowWidth(960), 960);
  assert.equal(normalizeWindowWidth(1200), 1200);
  assert.equal(normalizeWindowHeight(240), 240);
  assert.equal(normalizeWindowHeight(600), 600);
  assert.equal(normalizeWindowHeight(800), 800);
});

test("clamps window dimensions to the supported range", () => {
  assert.equal(normalizeWindowWidth(399), 400);
  assert.equal(normalizeWindowWidth(1201), 1200);
  assert.equal(normalizeWindowHeight(239), 240);
  assert.equal(normalizeWindowHeight(801), 800);
});

test("falls back for missing or invalid window dimensions", () => {
  assert.equal(normalizeWindowWidth(undefined), DEFAULT_WINDOW_WIDTH);
  assert.equal(normalizeWindowWidth(""), DEFAULT_WINDOW_WIDTH);
  assert.equal(normalizeWindowWidth("large"), DEFAULT_WINDOW_WIDTH);
  assert.equal(normalizeWindowHeight(undefined), DEFAULT_WINDOW_HEIGHT);
  assert.equal(normalizeWindowHeight(""), DEFAULT_WINDOW_HEIGHT);
  assert.equal(normalizeWindowHeight("tall"), DEFAULT_WINDOW_HEIGHT);
});
