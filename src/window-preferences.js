export const MIN_WINDOW_WIDTH = 400;
export const MAX_WINDOW_WIDTH = 1200;
export const DEFAULT_WINDOW_WIDTH = 720;
export const MIN_WINDOW_HEIGHT = 240;
export const MAX_WINDOW_HEIGHT = 800;
export const DEFAULT_WINDOW_HEIGHT = 240;

function normalizeInteger(value, min, max, fallback) {
  if (value == null || value === "") return fallback;
  const number = Number(value);
  if (!Number.isFinite(number)) return fallback;
  return Math.min(max, Math.max(min, Math.round(number)));
}

export function normalizeWindowWidth(value) {
  return normalizeInteger(value, MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH, DEFAULT_WINDOW_WIDTH);
}

export function normalizeWindowHeight(value) {
  return normalizeInteger(value, MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT, DEFAULT_WINDOW_HEIGHT);
}
