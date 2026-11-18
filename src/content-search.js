export function filterContentEntries(entries, query) {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return entries;
  return entries.filter((entry) => {
    const title = (entry.title || "").toLowerCase();
    const text = (entry.text || "").toLowerCase();
    return title.includes(normalized) || text.includes(normalized);
  });
}

export function nextSelectionIndex(currentIndex, length, direction) {
  if (length <= 0) return -1;
  if (currentIndex < 0) return direction > 0 ? 0 : length - 1;
  return Math.min(length - 1, Math.max(0, currentIndex + direction));
}
