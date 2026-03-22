/**
 * Lightweight markdown renderer — no dependencies.
 * Supports: fenced code blocks, inline code, h1–h6, bold, italic,
 * bullet lists, numbered lists, GFM tables, and paragraph breaks.
 */
export function renderMarkdown(raw: string): string {
  // 1. HTML-escape
  let text = raw
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Helper: apply inline transforms to a fragment (used inside table cells)
  const inlineFmt = (s: string): string => {
    s = s.replace(/`([^`\n]+)`/g, '<code class="md-code">$1</code>');
    s = s.replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>");
    s = s.replace(/\*\*(.+?)\*\*/g, '<strong class="md-bold">$1</strong>');
    s = s.replace(/\*([^*\n]+)\*/g, '<em class="md-em">$1</em>');
    s = s.replace(/_([^_\n]+)_/g, '<em class="md-em">$1</em>');
    return s;
  };

  // 2. Extract fenced code blocks → placeholders (protects content from inline transforms)
  const codeBlocks: string[] = [];
  text = text.replace(/```(\w*)\n?([\s\S]*?)```/g, (_match, lang: string, code: string) => {
    const langAttr = lang ? ` class="lang-${lang}"` : "";
    codeBlocks.push(`<pre class="md-code-block"><code${langAttr}>${code.trimEnd()}</code></pre>`);
    return `\x00CODE${codeBlocks.length - 1}\x00`;
  });

  // 3. Extract GFM tables → placeholders
  // A table is 2+ consecutive lines of |…|, where the second line is a separator (---|---)
  const tablePlaceholders: string[] = [];
  text = text.replace(/((?:\|[^\n]+\|\n?){2,})/g, (block: string) => {
    const rows = block.trim().split("\n");
    if (rows.length < 2 || !/^\|[\s:|-]+\|/.test(rows[1])) return block;

    const parseRow = (row: string): string[] =>
      row.trim().replace(/^\||\|$/g, "").split("|").map(c => c.trim());

    const headers = parseRow(rows[0]);
    const dataRows = rows.slice(2).filter(Boolean).map(parseRow);

    let html = '<table class="md-table"><thead><tr>';
    for (const h of headers) html += `<th>${inlineFmt(h)}</th>`;
    html += "</tr></thead><tbody>";
    for (const row of dataRows) {
      html += "<tr>";
      for (const cell of row) html += `<td>${inlineFmt(cell)}</td>`;
      html += "</tr>";
    }
    html += "</tbody></table>";

    tablePlaceholders.push(html);
    return `\x00TABLE${tablePlaceholders.length - 1}\x00`;
  });

  // 4. Extract inline code → placeholders
  const inlineCodes: string[] = [];
  text = text.replace(/`([^`\n]+)`/g, (_match, code: string) => {
    inlineCodes.push(`<code class="md-code">${code}</code>`);
    return `\x00INLINE${inlineCodes.length - 1}\x00`;
  });

  // 5. Line-by-line block element processing
  const lines = text.split("\n");
  const out: string[] = [];
  let inUl = false;
  let inOl = false;

  function closeList() {
    if (inUl) { out.push("</ul>"); inUl = false; }
    if (inOl) { out.push("</ol>"); inOl = false; }
  }

  for (const line of lines) {
    if (/^######\s/.test(line))      { closeList(); out.push(`<h6 class="md-h6">${line.slice(7)}</h6>`); }
    else if (/^#####\s/.test(line))  { closeList(); out.push(`<h5 class="md-h5">${line.slice(6)}</h5>`); }
    else if (/^####\s/.test(line))   { closeList(); out.push(`<h4 class="md-h4">${line.slice(5)}</h4>`); }
    else if (/^###\s/.test(line))    { closeList(); out.push(`<h3 class="md-h3">${line.slice(4)}</h3>`); }
    else if (/^##\s/.test(line))     { closeList(); out.push(`<h2 class="md-h2">${line.slice(3)}</h2>`); }
    else if (/^#\s/.test(line))      { closeList(); out.push(`<h1 class="md-h1">${line.slice(2)}</h1>`); }
    else if (/^[-*]\s/.test(line)) {
      if (!inUl) { closeList(); out.push(`<ul class="md-ul">`); inUl = true; }
      out.push(`<li>${line.slice(2)}</li>`);
    }
    else if (/^\d+\.\s/.test(line)) {
      if (!inOl) { closeList(); out.push(`<ol class="md-ol">`); inOl = true; }
      out.push(`<li>${line.replace(/^\d+\.\s/, "")}</li>`);
    }
    else {
      closeList();
      out.push(line === "" ? "<br>" : line);
    }
  }
  closeList();
  text = out.join("\n");

  // 6. Inline transforms (bold+italic before bold before italic to avoid partial matches)
  text = text.replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>");
  text = text.replace(/\*\*(.+?)\*\*/g, '<strong class="md-bold">$1</strong>');
  text = text.replace(/\*([^*\n]+)\*/g, '<em class="md-em">$1</em>');
  text = text.replace(/_([^_\n]+)_/g, '<em class="md-em">$1</em>');

  // 7. Restore inline code
  text = text.replace(/\x00INLINE(\d+)\x00/g, (_m, i: string) => inlineCodes[Number(i)]);

  // 8. Restore code blocks
  text = text.replace(/\x00CODE(\d+)\x00/g, (_m, i: string) => codeBlocks[Number(i)]);

  // 9. Restore tables
  text = text.replace(/\x00TABLE(\d+)\x00/g, (_m, i: string) => tablePlaceholders[Number(i)]);

  return text;
}
