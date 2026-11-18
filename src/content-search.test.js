import assert from "node:assert/strict";
import test from "node:test";

import { filterContentEntries, nextSelectionIndex } from "./content-search.js";

const entries = [
  { id: "1", title: "Disk", text: "df -h" },
  { id: "2", title: "Git", text: "git status" },
  { id: "3", title: null, text: "lsof -i" },
];

test("filters content by title or text", () => {
  assert.deepEqual(filterContentEntries(entries, "git").map((entry) => entry.id), ["2"]);
  assert.deepEqual(filterContentEntries(entries, "LSOF").map((entry) => entry.id), ["3"]);
  assert.equal(filterContentEntries(entries, "").length, 3);
});

test("moves selection through visible entries without wrapping", () => {
  assert.equal(nextSelectionIndex(-1, 4, 1), 0);
  assert.equal(nextSelectionIndex(-1, 4, -1), 3);
  assert.equal(nextSelectionIndex(0, 4, 1), 1);
  assert.equal(nextSelectionIndex(3, 4, 1), 3);
  assert.equal(nextSelectionIndex(0, 4, -1), 0);
  assert.equal(nextSelectionIndex(0, 0, 1), -1);
});
