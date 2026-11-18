export function renderMarkdown(md) {
  const escaped = md
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return renderBlocks(escaped);
}

// Renders already-escaped markdown text into HTML.
function renderBlocks(text) {
  const lines = text.split("\n");
  const blocks = parseBlocks(lines);
  return blocks.map(renderBlock).join("");
}

// --- Block parser ---

function parseBlocks(lines) {
  const blocks = [];
  let i = 0;
  let afterBlank = true;

  while (i < lines.length) {
    const line = lines[i];

    if (/^\s*$/.test(line)) {
      afterBlank = true;
      i++;
      continue;
    }

    // Fenced code block
    if (line.trim().startsWith("```")) {
      const codeLines = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      i++;
      blocks.push({ type: "code", content: codeLines.join("\n") });
      afterBlank = false;
      continue;
    }

    // ATX heading
    const atxMatch = line.match(/^(#{1,6})\s?(.*)$/);
    if (atxMatch) {
      const content = atxMatch[2].replace(/\s+#+\s*$/, "").trim();
      blocks.push({ type: "heading", level: atxMatch[1].length, content });
      i++;
      afterBlank = false;
      continue;
    }

    // Setext heading: line of = or - right after a paragraph
    if (!afterBlank && blocks.length && blocks[blocks.length - 1].type === "paragraph") {
      if (/^\s*=+\s*$/.test(line)) {
        const para = blocks.pop();
        blocks.push({ type: "heading", level: 1, content: para.content });
        i++;
        afterBlank = false;
        continue;
      }
      if (/^\s*-+\s*$/.test(line)) {
        const para = blocks.pop();
        blocks.push({ type: "heading", level: 2, content: para.content });
        i++;
        afterBlank = false;
        continue;
      }
    }

    // Thematic break (horizontal rule)
    if (isThematicBreak(line)) {
      blocks.push({ type: "hr" });
      i++;
      afterBlank = false;
      continue;
    }

    // Blockquote (&gt; after HTML escaping)
    if (/^&gt;\s?/.test(line)) {
      const quoteLines = [];
      while (i < lines.length && /^&gt;\s?/.test(lines[i])) {
        quoteLines.push(lines[i].replace(/^&gt;\s?/, ""));
        i++;
      }
      blocks.push({ type: "blockquote", content: quoteLines.join("\n") });
      afterBlank = false;
      continue;
    }

    // Table
    if (i + 1 < lines.length && isTableLine(line) && isTableSeparator(lines[i + 1])) {
      const tableLines = [line];
      i++;
      while (i < lines.length && isTableLine(lines[i])) {
        tableLines.push(lines[i]);
        i++;
      }
      blocks.push(parseTable(tableLines));
      afterBlank = false;
      continue;
    }

    // List
    if (/^[-*+]\s/.test(line) || /^\d+\.\s/.test(line)) {
      const result = parseList(lines, i);
      blocks.push(result.block);
      i = result.nextIndex;
      afterBlank = false;
      continue;
    }

    // Paragraph: collect consecutive non-blank, non-block-starting lines
    {
      const paraLines = [];
      while (i < lines.length) {
        const l = lines[i];
        if (/^\s*$/.test(l)) break;
        if (l.trim().startsWith("```")) break;
        if (/^#{1,6}\s?/.test(l)) break;
        if (isThematicBreak(l)) break;
        if (/^&gt;\s?/.test(l)) break;
        if (/^[-*+]\s/.test(l) || /^\d+\.\s/.test(l)) break;
        // Stop if next line is a setext underline
        if (i + 1 < lines.length) {
          const nl = lines[i + 1];
          if (/^\s*=+\s*$/.test(nl) || /^\s*-+\s*$/.test(nl)) {
            paraLines.push(l);
            i++;
            break;
          }
        }
        paraLines.push(l);
        i++;
      }
      if (paraLines.length) {
        blocks.push({ type: "paragraph", content: paraLines.join(" ") });
      }
      afterBlank = false;
    }
  }

  return blocks;
}

// --- Block helpers ---

function isThematicBreak(line) {
  const trimmed = line.trim();
  if (!trimmed) return false;
  const chars = trimmed.replace(/\s/g, "");
  if (chars.length < 3) return false;
  const first = chars[0];
  if (first !== "-" && first !== "*" && first !== "_") return false;
  return [...chars].every((c) => c === first);
}

function isTableLine(line) {
  return line.includes("|");
}

function isTableSeparator(line) {
  const trimmed = line.trim();
  if (!trimmed.includes("-")) return false;
  const inner = trimmed.replace(/^\s*\|/, "").replace(/\|\s*$/, "");
  const cells = inner.split("|");
  if (cells.length < 1) return false;
  return cells.every((cell) => {
    const t = cell.trim();
    return /^:?-+:?$/.test(t) && t.includes("-");
  });
}

function parseTable(tableLines) {
  const header = splitTableRow(tableLines[0]);
  const alignments = splitTableRow(tableLines[1]).map((cell) => {
    const t = cell.trim();
    if (t.startsWith(":") && t.endsWith(":")) return "center";
    if (t.endsWith(":")) return "right";
    if (t.startsWith(":")) return "left";
    return null;
  });
  const rows = tableLines.slice(2).map(splitTableRow);
  return { type: "table", header, alignments, rows };
}

function splitTableRow(line) {
  return line.trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
}

function parseList(lines, startI) {
  const items = [];
  let i = startI;
  const isUl = /^[-*+]\s/.test(lines[startI]);

  while (i < lines.length) {
    const line = lines[i];
    if (isUl ? /^[-*+]\s/.test(line) : /^\d+\.\s/.test(line)) {
      let content = isUl
        ? line.replace(/^[-*+]\s/, "")
        : line.replace(/^\d+\.\s/, "");
      const task = content.match(/^\[([ xX])\]\s(.*)$/);
      if (task) {
        items.push({ content: task[2], task: task[1].toLowerCase() === "x" });
      } else {
        items.push({ content, task: null });
      }
      i++;
    } else if (/^\s+/.test(line) && items.length > 0) {
      items[items.length - 1].content += " " + line.trim();
      i++;
    } else {
      break;
    }
  }

  return { block: { type: "list", ordered: !isUl, items }, nextIndex: i };
}

// --- Block renderer ---

function renderBlock(block) {
  switch (block.type) {
    case "code":
      return `<pre><code>${block.content}</code></pre>`;
    case "heading":
      return `<h${block.level}>${formatInline(block.content)}</h${block.level}>`;
    case "hr":
      return "<hr>";
    case "blockquote":
      return `<blockquote>${renderBlocks(block.content)}</blockquote>`;
    case "list":
      return renderList(block);
    case "table":
      return renderTable(block);
    case "paragraph":
      return `<p>${formatInline(block.content)}</p>`;
    default:
      return "";
  }
}

function renderList(block) {
  const tag = block.ordered ? "ol" : "ul";
  const items = block.items.map((item) => {
    let content = formatInline(item.content);
    if (item.task !== null) {
      const checked = item.task ? " checked" : "";
      content = `<input type="checkbox"${checked} disabled> ${content}`;
    }
    return `<li>${content}</li>`;
  });
  return `<${tag}>${items.join("")}</${tag}>`;
}

function renderTable(block) {
  const head = block.header.map((cell, j) => {
    const a = block.alignments[j];
    const s = a ? ` style="text-align:${a}"` : "";
    return `<th${s}>${formatInline(cell)}</th>`;
  }).join("");
  const body = block.rows.map((row) => {
    const cells = row.map((cell, j) => {
      const a = block.alignments[j];
      const s = a ? ` style="text-align:${a}"` : "";
      return `<td${s}>${formatInline(cell)}</td>`;
    }).join("");
    return `<tr>${cells}</tr>`;
  }).join("");
  return `<table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

// --- Inline formatter ---

function formatInline(text) {
  // 1. Extract code spans to protect their content
  const codes = [];
  text = text.replace(/`([^`]+)`/g, (_, code) => {
    codes.push(code);
    return `\u0000${codes.length - 1}\u0000`;
  });

  // 2. Backslash escapes
  text = text.replace(/\\([*_`~\[\]()!\\])/g, (_, ch) => {
    const m = {
      "*": "&#42;", "_": "&#95;", "`": "&#96;", "~": "&#126;",
      "[": "&#91;", "]": "&#93;", "(": "&#40;", ")": "&#41;",
      "!": "&#33;", "\\": "&#92;",
    };
    return m[ch] || ch;
  });

  // 3. Images (before links, since ![]() contains []())
  text = text.replace(/!\[([^\]]*)\]\(([^)\s]+)(?:\s+([^)]*))?\)/g, (_, alt, url, title) => {
    const safe = sanitizeUrl(url);
    if (!safe) return alt;
    let img = `<img src="${escapeAttribute(safe)}" alt="${alt}"`;
    if (title) img += ` title="${escapeAttribute(title.trim())}"`;
    return img + ">";
  });

  // 4. Links
  text = text.replace(/\[([^\]]+)\]\(([^)\s]+)(?:\s+([^)]*))?\)/g, (_, label, url, title) => {
    const safe = sanitizeUrl(url);
    if (!safe) return label;
    let a = `<a href="${escapeAttribute(safe)}"`;
    if (title) a += ` title="${escapeAttribute(title.trim())}"`;
    return a + `>${label}</a>`;
  });

  // 5. Bold (** before *)
  text = text.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");

  // 6. Italic (avoid * surrounded by spaces)
  text = text.replace(/\*(?!\s)(.+?)(?<!\s)\*/g, "<em>$1</em>");

  // 7. Strikethrough (GFM ~~)
  text = text.replace(/~~(?!\s)(.+?)(?<!\s)~~/g, "<del>$1</del>");

  // 8. Autolinks (bare URLs not inside HTML attributes)
  text = text.replace(/(?<!["'>])https?:\/\/[^\s<]+/g, (url) => {
    const m = url.match(/^(.*?)([.,;]+)$/);
    if (m) return `<a href="${m[1]}">${m[1]}</a>${m[2]}`;
    return `<a href="${url}">${url}</a>`;
  });

  // 9. Restore code spans
  text = text.replace(/\u0000(\d+)\u0000/g, (_, idx) => `<code>${codes[parseInt(idx)]}</code>`);

  return text;
}

// --- URL safety ---

function sanitizeUrl(url) {
  const trimmed = url.trim().replace(/&quot;/g, "");
  if (/^(https?:|mailto:)/i.test(trimmed) && !/[\u0000-\u001F\u007F<>"'`]/.test(trimmed)) {
    return trimmed;
  }
  return "";
}

function escapeAttribute(value) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;");
}
