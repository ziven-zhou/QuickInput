export function normalizeContentMode(mode) {
  return typeof mode === "string" && mode.trim().toLowerCase() === "markdown"
    ? "markdown"
    : "plain";
}
