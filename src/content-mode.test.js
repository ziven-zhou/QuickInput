import assert from "node:assert/strict";
import test from "node:test";

import { normalizeContentMode } from "./content-mode.js";

test("preserves supported markdown mode", () => {
  assert.equal(normalizeContentMode("markdown"), "markdown");
  assert.equal(normalizeContentMode(" MarkDown "), "markdown");
});

test("falls back to plain text for removed or unknown modes", () => {
  assert.equal(normalizeContentMode("plain"), "plain");
  assert.equal(normalizeContentMode("code"), "plain");
  assert.equal(normalizeContentMode(""), "plain");
  assert.equal(normalizeContentMode(undefined), "plain");
});
