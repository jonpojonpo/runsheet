<script lang="ts">
  import type { DataTableComponent } from "../lib/catalog";

  let { spec }: { spec: DataTableComponent } = $props();

  let globalFilter = $state("");
  let sortKey = $state(spec.props.default_sort?.key ?? "");
  let sortDesc = $state(spec.props.default_sort?.direction === "desc");
  let page = $state(0);

  const pageSize = spec.props.page_size ?? 25;

  function formatCell(value: unknown, type?: string): string {
    if (value === null || value === undefined) return "—";
    if (type === "currency" && typeof value === "number") {
      return new Intl.NumberFormat("en-GB", { style: "currency", currency: "GBP", notation: "compact" }).format(value);
    }
    if (type === "percent" && typeof value === "number") return `${value.toFixed(1)}%`;
    return String(value);
  }

  function colAlign(type?: string): string {
    return (type === "number" || type === "currency" || type === "percent") ? "right" : "left";
  }

  const filtered = $derived.by(() => {
    const q = globalFilter.toLowerCase();
    if (!q) return spec.props.rows;
    return spec.props.rows.filter((row) =>
      Object.values(row).some((v) => String(v ?? "").toLowerCase().includes(q))
    );
  });

  const sorted = $derived.by(() => {
    if (!sortKey) return filtered;
    return [...filtered].sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (av === bv) return 0;
      const cmp = av == null ? -1 : bv == null ? 1 : av < bv ? -1 : 1;
      return sortDesc ? -cmp : cmp;
    });
  });

  const pageCount = $derived(Math.max(1, Math.ceil(sorted.length / pageSize)));
  const pageRows = $derived(sorted.slice(page * pageSize, (page + 1) * pageSize));

  function toggleSort(key: string) {
    if (sortKey === key) {
      if (!sortDesc) { sortDesc = true; }
      else { sortKey = ""; sortDesc = false; }
    } else {
      sortKey = key;
      sortDesc = false;
    }
    page = 0;
  }

  function sortIcon(key: string): string {
    if (sortKey !== key) return "⇅";
    return sortDesc ? "▼" : "▲";
  }

  $effect(() => {
    // reset to page 0 when filter changes
    globalFilter;
    page = 0;
  });
</script>

<div class="datatable-wrap">
  <div class="table-controls">
    <input
      class="global-filter"
      placeholder="Search all columns…"
      bind:value={globalFilter}
    />
    <span class="row-count text-faint">{filtered.length} rows</span>
  </div>

  <div class="table-scroll">
    <table>
      <thead>
        <tr>
          {#each spec.props.columns as col}
            <th
              class:sortable={col.sortable !== false}
              class:sorted={sortKey === col.key}
              style="text-align: {col.align ?? colAlign(col.type)}"
              onclick={() => col.sortable !== false && toggleSort(col.key)}
            >
              <span>{col.label}</span>
              {#if col.sortable !== false}
                <span class="sort-icon mono">{sortIcon(col.key)}</span>
              {/if}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each pageRows as row}
          <tr>
            {#each spec.props.columns as col}
              <td style="text-align: {col.align ?? colAlign(col.type)}">
                {formatCell(row[col.key], col.type)}
              </td>
            {/each}
          </tr>
        {/each}
        {#if spec.props.summary_row}
          <tr class="summary-row">
            {#each spec.props.columns as col}
              <td style="text-align: {col.align ?? colAlign(col.type)}">
                <strong>{formatCell(spec.props.summary_row![col.key], col.type)}</strong>
              </td>
            {/each}
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if pageCount > 1}
    <div class="pagination">
      <button onclick={() => page--} disabled={page === 0}>←</button>
      <span class="text-dim">Page {page + 1} / {pageCount}</span>
      <button onclick={() => page++} disabled={page >= pageCount - 1}>→</button>
    </div>
  {/if}
</div>

<style>
  .datatable-wrap { display: flex; flex-direction: column; gap: 8px; }

  .table-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .global-filter {
    padding: 5px 10px;
    font-size: 12px;
    flex: 1;
    max-width: 280px;
  }

  .row-count { font-size: 11px; }

  .table-scroll { overflow-x: auto; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  th {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border-active);
    color: var(--text-dim);
    font-weight: 600;
    font-family: var(--font-mono);
    font-size: 11px;
    white-space: nowrap;
    user-select: none;
    background: var(--surface);
    position: sticky;
    top: 0;
  }

  th.sortable { cursor: pointer; }
  th.sortable:hover { color: var(--text); }
  th.sorted { color: var(--accent-green); }

  .sort-icon { margin-left: 4px; opacity: 0.5; font-size: 9px; }
  th.sorted .sort-icon { opacity: 1; }

  td {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    vertical-align: middle;
  }

  tr:hover td { background: var(--surface-2); }

  .summary-row td {
    border-top: 1px solid var(--border-active);
    background: var(--surface);
    color: var(--text);
  }

  .pagination {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: center;
    padding: 8px 0;
    font-size: 12px;
  }

  .pagination button {
    padding: 3px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-dim);
  }

  .pagination button:disabled { opacity: 0.4; cursor: not-allowed; }
  .pagination button:not(:disabled):hover { border-color: var(--accent-green); color: var(--accent-green); }
</style>
