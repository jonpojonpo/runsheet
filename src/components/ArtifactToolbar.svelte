<script lang="ts">
  import type { Artifact } from "../lib/types";

  let { artifact }: { artifact: Artifact } = $props();

  const hasTable = $derived(
    !!artifact.spec?.components.find((c) => c.component === "data_table")
  );

  function downloadCSV() {
    if (!artifact.spec) return;
    const tableComp = artifact.spec.components.find((c) => c.component === "data_table");
    if (!tableComp || tableComp.component !== "data_table") return;
    const { columns, rows } = tableComp.props;
    const header = columns.map((c: { label: string }) => c.label).join(",");
    const body = rows
      .map((r: Record<string, unknown>) =>
        columns.map((c: { key: string }) => JSON.stringify(r[c.key] ?? "")).join(",")
      )
      .join("\n");
    const csv = `${header}\n${body}`;
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${artifact.title.replace(/[^a-z0-9]/gi, "_")}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function saveHtml() {
    if (!artifact.html) return;
    const blob = new Blob([artifact.html], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = artifact.title.replace(/[^a-z0-9]/gi, "_") + ".html";
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="toolbar">
  {#if hasTable}
    <button class="tool-btn" onclick={downloadCSV} title="Download table as CSV">⬇ CSV</button>
  {/if}
  {#if artifact.kind === "html"}
    <button class="tool-btn" onclick={saveHtml} title="Save HTML file">⬇ HTML</button>
  {/if}
  <span class="spacer"></span>
  <span class="artifact-kind text-faint mono">{artifact.kind}</span>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
  }
  .tool-btn {
    padding: 4px 10px;
    font-size: 11px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-dim);
    transition: border-color 0.1s, color 0.1s;
  }
  .tool-btn:hover { border-color: var(--accent-green); color: var(--accent-green); }
  .spacer { flex: 1; }
  .artifact-kind { font-size: 10px; }
</style>
