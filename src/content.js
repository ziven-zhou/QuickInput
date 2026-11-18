import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { normalizeContentMode } from "./content-mode.js";
import { filterContentEntries, nextSelectionIndex } from "./content-search.js";
import { initializePageShortcuts } from "./page-shortcuts.js";
import { renderMarkdown } from "./markdown.js";

const list = document.getElementById("content-list");
const empty = document.getElementById("content-empty");
const count = document.getElementById("content-count");
const searchInput = document.getElementById("search-input");
const addContentBtn = document.getElementById("add-content-btn");
const closeBtn = document.getElementById("close-content-btn");
const toast = document.getElementById("toast");

const editorModal = document.getElementById("editor-modal");
const editorModalTitle = document.getElementById("editor-modal-title");
const closeEditorBtn = document.getElementById("close-editor-btn");
const cancelEditorBtn = document.getElementById("cancel-editor-btn");
const saveEditorBtn = document.getElementById("save-editor-btn");
const editorTitleInput = document.getElementById("editor-title-input");
const editorTextarea = document.getElementById("editor-textarea");
const editorModeSelect = document.getElementById("editor-mode-select");

const previewModal = document.getElementById("preview-modal");
const previewModalTitle = document.getElementById("preview-modal-title");
const previewModalContent = document.getElementById("preview-modal-content");
const previewModalMeta = document.getElementById("preview-modal-meta");
const closePreviewBtn = document.getElementById("close-preview-btn");
const previewEditBtn = document.getElementById("preview-edit-btn");
const previewPasteBtn = document.getElementById("preview-paste-btn");

const confirmModal = document.getElementById("confirm-modal");
const confirmText = document.getElementById("confirm-text");
const confirmCancelBtn = document.getElementById("confirm-cancel-btn");
const confirmOkBtn = document.getElementById("confirm-ok-btn");

const MODE_LABELS = {
  plain: "纯文本",
  markdown: "Markdown",
};

let entries = [];
let searchQuery = "";
let editingId = null;
let pendingAction = null;
let toastTimer = null;
let selectedEntryId = null;

function formatTime(millis) {
  const number = Number(millis);
  if (!Number.isFinite(number)) return "";
  const date = new Date(number);
  const pad = (value) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function displayTitle(entry) {
  const title = (entry.title || "").trim();
  if (title) return title;
  const firstLine = (entry.text || "").split("\n")[0].trim();
  return firstLine || "未命名内容";
}

function previewText(entry) {
  const text = (entry.text || "").replace(/\s+/g, " ").trim();
  return text.length > 120 ? `${text.slice(0, 120)}...` : text;
}

function visibleEntries() {
  return filterContentEntries(entries, searchQuery);
}

function selectedEntry() {
  if (!selectedEntryId) return null;
  return visibleEntries().find((entry) => entry.id === selectedEntryId) || null;
}

function focusSearch() {
  searchInput.focus();
  searchInput.select();
}

function clearSelection(focus = false) {
  selectedEntryId = null;
  render();
  if (focus) focusSearch();
}

function moveSelection(direction) {
  const visible = visibleEntries();
  if (!visible.length) return;
  const currentIndex = visible.findIndex((entry) => entry.id === selectedEntryId);
  const nextIndex = nextSelectionIndex(currentIndex, visible.length, direction);
  selectedEntryId = visible[nextIndex].id;
  render();
  list
    .querySelector(`.content-item[data-id="${selectedEntryId}"]`)
    ?.scrollIntoView({ block: "nearest" });
}

function showToast(message, type = "success") {
  clearTimeout(toastTimer);
  toast.textContent = message;
  toast.classList.toggle("error", type === "error");
  toast.classList.remove("hidden");
  toastTimer = setTimeout(() => {
    toast.classList.add("hidden");
    toast.classList.remove("error");
  }, 2500);
}

function render() {
  list.innerHTML = "";
  const visible = visibleEntries();
  const hasEntries = visible.length > 0;
  empty.classList.toggle("hidden", hasEntries);
  list.classList.toggle("hidden", !hasEntries);
  empty.textContent = searchQuery ? "未找到匹配内容" : "还没有内容，点击“新增内容”添加";
  count.textContent = searchQuery
    ? `${visible.length}/${entries.length} 条`
    : `${entries.length} 条`;

  visible.forEach((entry) => {
    const row = document.createElement("div");
    row.className = "content-item";
    row.dataset.id = entry.id;
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(entry.id === selectedEntryId));
    row.classList.toggle("selected", entry.id === selectedEntryId);

    const body = document.createElement("div");
    body.className = "content-item-body";
    body.addEventListener("click", () => pasteEntry(entry));

    const title = document.createElement("div");
    title.className = "content-item-title";
    title.textContent = displayTitle(entry);

    const text = document.createElement("div");
    text.className = "content-item-text";
    text.textContent = previewText(entry);
    text.title = entry.text;

    const mode = normalizeContentMode(entry.mode);
    const meta = document.createElement("div");
    meta.className = "content-item-meta";
    meta.textContent = `${MODE_LABELS[mode]} · ${formatTime(entry.updated_at)}`;

    body.appendChild(title);
    body.appendChild(text);
    body.appendChild(meta);

    const actions = document.createElement("div");
    actions.className = "content-item-actions";

    const previewBtn = document.createElement("button");
    previewBtn.className = "btn btn-ghost btn-sm";
    previewBtn.textContent = "预览";
    previewBtn.addEventListener("click", () => openPreview(entry));

    const editBtn = document.createElement("button");
    editBtn.className = "btn btn-ghost btn-sm";
    editBtn.textContent = "编辑";
    editBtn.addEventListener("click", () => openEditor(entry));

    const deleteBtn = document.createElement("button");
    deleteBtn.className = "btn btn-ghost btn-sm danger";
    deleteBtn.textContent = "删除";
    deleteBtn.addEventListener("click", () => {
      confirmAction(`确定删除“${displayTitle(entry)}”？`, () => deleteEntry(entry.id));
    });

    actions.appendChild(previewBtn);
    actions.appendChild(editBtn);
    actions.appendChild(deleteBtn);
    row.appendChild(body);
    row.appendChild(actions);
    list.appendChild(row);
  });
}

async function load() {
  try {
    entries = await invoke("get_contents");
  } catch (err) {
    console.error("load contents failed:", err);
    entries = [];
  }
  render();
  return entries;
}

function openEditor(entry = null) {
  editingId = entry?.id || null;
  editorModalTitle.textContent = entry ? "编辑内容" : "新增内容";
  editorTitleInput.value = entry?.title || "";
  editorTextarea.value = entry?.text || "";
  editorModeSelect.value = normalizeContentMode(entry?.mode);
  editorModal.classList.remove("hidden");
  setTimeout(() => editorTitleInput.focus(), 50);
}

function closeEditor() {
  editorModal.classList.add("hidden");
  editingId = null;
  editorTitleInput.value = "";
  editorTextarea.value = "";
  editorModeSelect.value = "plain";
  setTimeout(focusSearch, 0);
}

async function saveEditor() {
  const text = editorTextarea.value;
  if (!text.trim()) {
    showToast("内容不能为空", "error");
    editorTextarea.focus();
    return;
  }

  const payload = {
    title: editorTitleInput.value.trim() || null,
    text,
    mode: normalizeContentMode(editorModeSelect.value),
  };
  saveEditorBtn.disabled = true;
  try {
    if (editingId) {
      await invoke("update_content", { id: editingId, ...payload });
    } else {
      await invoke("add_content", payload);
    }
    closeEditor();
    await load();
    showToast("已保存");
  } catch (err) {
    console.error("save content failed:", err);
    showToast(String(err), "error");
  } finally {
    saveEditorBtn.disabled = false;
  }
}

function openPreview(entry) {
  const mode = normalizeContentMode(entry.mode);
  previewModalTitle.textContent = displayTitle(entry);
  previewModalContent.className = `modal-body ${mode}`;
  if (mode === "markdown") {
    previewModalContent.innerHTML = renderMarkdown(entry.text);
  } else {
    previewModalContent.textContent = entry.text;
  }
  previewModalMeta.textContent = `${MODE_LABELS[mode]} · ${formatTime(entry.updated_at)}`;
  previewEditBtn.onclick = () => {
    closePreview();
    openEditor(entry);
  };
  previewPasteBtn.onclick = () => pasteEntry(entry);
  previewModal.classList.remove("hidden");
}

function closePreview() {
  previewModal.classList.add("hidden");
  previewModalContent.textContent = "";
  previewModalContent.innerHTML = "";
  previewModalContent.className = "modal-body";
  previewEditBtn.onclick = null;
  previewPasteBtn.onclick = null;
  setTimeout(focusSearch, 0);
}

async function pasteEntry(entry) {
  closePreview();
  try {
    await invoke("paste_content", { id: entry.id });
  } catch (err) {
    console.error("paste content failed:", err);
    showToast(String(err), "error");
  }
}

async function deleteEntry(id) {
  try {
    await invoke("delete_content", { id });
    await load();
    showToast("已删除");
  } catch (err) {
    console.error("delete content failed:", err);
    showToast(String(err), "error");
  }
}

function confirmAction(message, action) {
  pendingAction = action;
  confirmText.textContent = message;
  confirmModal.classList.remove("hidden");
}

function closeConfirm() {
  confirmModal.classList.add("hidden");
  pendingAction = null;
  setTimeout(focusSearch, 0);
}

async function applySettings() {
  try {
    const settings = await invoke("get_settings");
    if (settings?.bg_opacity != null) {
      document.documentElement.style.setProperty("--bg-opacity", String(settings.bg_opacity));
    }
  } catch (err) {
    console.error("load settings failed:", err);
  }
}

addContentBtn.addEventListener("click", () => openEditor());
closeEditorBtn.addEventListener("click", closeEditor);
cancelEditorBtn.addEventListener("click", closeEditor);
saveEditorBtn.addEventListener("click", saveEditor);
closePreviewBtn.addEventListener("click", closePreview);
closeBtn.addEventListener("click", () => getCurrentWindow().close());

searchInput.addEventListener("input", () => {
  searchQuery = searchInput.value.trim().toLowerCase();
  selectedEntryId = null;
  render();
});

confirmCancelBtn.addEventListener("click", closeConfirm);
confirmOkBtn.addEventListener("click", () => {
  if (!pendingAction) return;
  const action = pendingAction;
  closeConfirm();
  action();
});

confirmModal.addEventListener("click", (event) => {
  if (event.target === confirmModal) closeConfirm();
});
editorModal.addEventListener("click", (event) => {
  if (event.target === editorModal) closeEditor();
});
previewModal.addEventListener("click", (event) => {
  if (event.target === previewModal) closePreview();
});

document.addEventListener("keydown", (event) => {
  const overlayOpen =
    !confirmModal.classList.contains("hidden") ||
    !editorModal.classList.contains("hidden") ||
    !previewModal.classList.contains("hidden");
  const buttonFocused =
    event.target instanceof HTMLElement && event.target.closest("button") !== null;

  if (event.key === "Escape") {
    if (!confirmModal.classList.contains("hidden")) {
      closeConfirm();
    } else if (!editorModal.classList.contains("hidden")) {
      closeEditor();
    } else if (!previewModal.classList.contains("hidden")) {
      closePreview();
    } else if (selectedEntry()) {
      event.preventDefault();
      clearSelection(true);
    } else {
      getCurrentWindow().close();
    }
  } else if (!overlayOpen && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
    event.preventDefault();
    moveSelection(event.key === "ArrowDown" ? 1 : -1);
  } else if (
    !overlayOpen &&
    event.key === "Enter" &&
    !buttonFocused &&
    selectedEntry()
  ) {
    event.preventDefault();
    pasteEntry(selectedEntry());
  } else if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    if (!editorModal.classList.contains("hidden")) {
      event.preventDefault();
      saveEditor();
    }
  }
});

listen("reload-content", () => load());
listen("page-shortcuts-updated", applySettings);
listen("content-paste-error", (event) => {
  showToast(`粘贴失败：${event.payload?.message || event.payload}`, "error");
});
getCurrentWindow().onFocusChanged(({ payload: focused }) => {
  if (focused) {
    load().then(focusSearch);
  }
});

load().then(focusSearch);
applySettings();
initializePageShortcuts();

const dragRegion = document.getElementById("drag-region");
if (dragRegion) {
  dragRegion.addEventListener("mousedown", (event) => {
    if (event.target.closest("button, select, input, textarea, a, label")) return;
    getCurrentWindow().startDragging().catch(() => {});
  });
}
