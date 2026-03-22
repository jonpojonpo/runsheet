<script lang="ts">
  import { tables, tablesLoading, tablesStore, pdfProgress } from "../stores/tables.svelte";
  import { dropTable } from "../lib/tauri";
  import { settingsStore } from "../stores/settings.svelte";
  import type { TableMeta } from "../lib/types";

  let expanded: Record<string, boolean> = $state({});
  let panelWidth = $state(280);

  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = panelWidth;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (e: MouseEvent) => {
      panelWidth = Math.max(180, Math.min(600, startWidth + e.clientX - startX));
    };
    const onUp = () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function toggle(name: string) {
    expanded[name] = !expanded[name];
  }

  async function handleDrop(table: TableMeta) {
    try {
      await dropTable(table.name);
      tablesStore.removeTable(table.name);
    } catch (e) {
      console.error(e);
    }
  }

  function formatCount(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return n.toString();
  }
</script>

<aside class="data-panel" style="width: {panelWidth}px">
  <div class="panel-header">
    <span class="panel-title">DATA</span>
    {#if $tablesLoading}
      <span class="text-faint" style="font-size:11px">loading…</span>
    {/if}
  </div>

  {#if $pdfProgress}
    <div class="pdf-progress">
      <div class="pdf-filename">{$pdfProgress.file}</div>
      <div class="pdf-bar-track">
        <div class="pdf-bar-fill" style="width:{Math.round($pdfProgress.page / $pdfProgress.total * 100)}%"></div>
      </div>
      <div class="pdf-stats">
        <span class="text-faint">page {$pdfProgress.page}/{$pdfProgress.total}</span>
        {#if $pdfProgress.tables_found > 0}
          <span class="text-green">{$pdfProgress.tables_found} tables</span>
        {/if}
      </div>
    </div>
  {/if}

  <div class="panel-content">
    {#if $tables.length === 0}
      <div class="empty-hint">
        <p class="text-faint">Drop CSV, XLSX, or Parquet</p>
      </div>
    {:else}
      {#each $tables as table (table.name)}
        <div class="table-item">
          <div
            class="table-header"
            role="button"
            tabindex="0"
            onclick={() => toggle(table.name)}
            onkeydown={(e) => e.key === "Enter" && toggle(table.name)}
          >
            <span class="expand-arrow" class:open={expanded[table.name]}>▶</span>
            <span class="table-name mono">{table.name}</span>
            <span class="row-count text-green">{formatCount(table.row_count)}</span>
            <button
              class="drop-btn text-faint"
              onclick={(e) => { e.stopPropagation(); handleDrop(table); }}
              title="Remove table"
            >×</button>
          </div>

          {#if expanded[table.name]}
            <div class="column-list">
              {#each table.columns as col}
                <div class="column-item">
                  <span class="col-name mono">{col.name}</span>
                  <span class="col-type text-faint mono">{col.type}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <div
    class="resize-handle"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize panel"
    onmousedown={startResize}
  ></div>

  <div class="panel-footer">
    <span class="text-faint">{$tables.length} table{$tables.length !== 1 ? "s" : ""}</span>
    <div class="footer-actions">
      <button
        class="icon-btn"
        onclick={() => settingsStore.open()}
        title="Settings"
      >⚙</button>
    </div>
  </div>
</aside>

<style>
  .data-panel {
    min-width: 180px;
    max-width: 600px;
    height: 100%;
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
    position: relative;
  }

  .resize-handle {
    position: absolute;
    top: 0;
    right: 0;
    width: 4px;
    height: 100%;
    cursor: col-resize;
    z-index: 10;
  }

  .resize-handle::after {
    content: "";
    position: absolute;
    top: 0;
    right: 0;
    width: 1px;
    height: 100%;
    background: var(--border);
    transition: background 0.15s, width 0.15s;
  }

  .resize-handle:hover::after {
    width: 2px;
    background: var(--accent-green);
    opacity: 0.6;
  }

  .panel-header {
    padding: 12px 12px 8px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .panel-title {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: var(--accent-green);
    opacity: 0.7;
  }

  .pdf-progress {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex-shrink: 0;
  }

  .pdf-filename {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pdf-bar-track {
    height: 3px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }

  .pdf-bar-fill {
    height: 100%;
    background: var(--accent-green);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .pdf-stats {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 0;
  }

  .empty-hint {
    padding: 20px 12px;
    text-align: center;
    font-size: 12px;
  }

  .table-item {
    border-bottom: 1px solid var(--border);
  }

  .table-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    cursor: pointer;
    user-select: none;
  }

  .table-header:hover { background: var(--surface-2); }

  .expand-arrow {
    font-size: 9px;
    color: var(--text-faint);
    transition: transform 0.15s;
    flex-shrink: 0;
  }

  .expand-arrow.open { transform: rotate(90deg); }

  .table-name {
    flex: 1;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-count {
    font-size: 11px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .drop-btn {
    font-size: 16px;
    line-height: 1;
    padding: 0 2px;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.1s;
  }

  .table-header:hover .drop-btn { opacity: 1; }
  .drop-btn:hover { color: var(--accent-red) !important; }

  .column-list {
    padding: 4px 0 4px 24px;
    background: var(--bg);
  }

  .column-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 10px 3px 0;
    gap: 8px;
  }

  .col-name { font-size: 11px; color: var(--text-dim); }
  .col-type { font-size: 10px; }

  .panel-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }

  .footer-actions {
    display: flex;
    gap: 4px;
  }

  .icon-btn {
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-faint);
    transition: color 0.1s, border-color 0.1s;
  }
  .icon-btn:hover { color: var(--accent-green); border-color: var(--accent-green); }
  .icon-btn.active { color: var(--accent-green); border-color: var(--accent-green); }

</style>
