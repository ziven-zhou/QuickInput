export const DEFAULT_INPUT_FONT_SIZE = 14;
export const MIN_INPUT_FONT_SIZE = 12;
export const MAX_INPUT_FONT_SIZE = 32;

export function normalizeInputFontSize(value) {
  const size = Number(value);
  if (
    Number.isInteger(size) &&
    size >= MIN_INPUT_FONT_SIZE &&
    size <= MAX_INPUT_FONT_SIZE
  ) {
    return size;
  }
  return DEFAULT_INPUT_FONT_SIZE;
}

export function applyInputPreferences(appElement, rootElement, settings = {}) {
  const pureMode = settings?.pure_mode === true;
  const fontSize = normalizeInputFontSize(settings?.input_font_size);

  appElement?.classList.toggle("pure-mode", pureMode);
  rootElement?.style.setProperty("--input-font-size", `${fontSize}px`);

  return { pureMode, fontSize };
}
