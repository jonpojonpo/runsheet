import { writable } from "svelte/store";
import type { TableMeta } from "../lib/types";
import { getSchema } from "../lib/tauri";

export const tables = writable<TableMeta[]>([]);
export const tablesLoading = writable(false);
export const tablesError = writable<string | null>(null);

export interface PdfProgress {
  file: string;
  page: number;
  total: number;
  tables_found: number;
}
export const pdfProgress = writable<PdfProgress | null>(null);

export const tablesStore = {
  async refresh() {
    tablesLoading.set(true);
    tablesError.set(null);
    try {
      const result = await getSchema();
      tables.set(result);
    } catch (e: unknown) {
      tablesError.set((e as { message?: string })?.message ?? String(e));
    } finally {
      tablesLoading.set(false);
    }
  },

  removeTable(name: string) {
    tables.update((ts) => ts.filter((t) => t.name !== name));
  },

  addOrUpdateTable(table: TableMeta) {
    tables.update((ts) => {
      const idx = ts.findIndex((t) => t.name === table.name);
      if (idx >= 0) {
        const next = [...ts];
        next[idx] = table;
        return next;
      }
      return [...ts, table];
    });
  },
};
