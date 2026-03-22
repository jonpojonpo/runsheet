<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import DataPanel from "./components/DataPanel.svelte";
  import ChatPanel from "./components/ChatPanel.svelte";
  import ArtifactDrawer from "./components/ArtifactDrawer.svelte";
  import { tablesStore, pdfProgress } from "./stores/tables.svelte";
  import { artifacts, artifactsStore } from "./stores/artifacts.svelte";
  import { drawerOpen, drawerStore } from "./stores/drawer.svelte";
  import { chatStore } from "./stores/chat.svelte";
  import { settingsOpen } from "./stores/settings.svelte";
  import { ingestFile } from "./lib/tauri";
  import type { TableMeta, Artifact } from "./lib/types";
  import SettingsModal from "./components/SettingsModal.svelte";

  let isDragOver = $state(false);

  onMount(() => {
    tablesStore.refresh();

    // Tauri v2 drag-drop events — set up listeners, collect unlisteners
    let unlisteners: Array<() => void> = [];

    Promise.all([
      listen<{ paths: string[] }>("tauri://drag-drop", async (event) => {
        isDragOver = false;
        for (const path of event.payload.paths) {
          try {
            const metas = await ingestFile(path);
            for (const meta of metas) tablesStore.addOrUpdateTable(meta);
          } catch (e: unknown) {
            console.error("Failed to ingest:", e);
          } finally {
            pdfProgress.set(null);
          }
        }
      }),
      listen("tauri://drag-enter", () => { isDragOver = true; }),
      listen("tauri://drag-leave", () => { isDragOver = false; }),
      listen<{ file: string; page: number; total: number; tables_found: number }>("pdf:progress", (e) => {
        pdfProgress.set(e.payload);
      }),
      listen<Artifact>("chat:artifact", (event) => {
        artifactsStore.add(event.payload);
        drawerStore.open();
        chatStore.attachArtifactToLastMessage(event.payload.id);
      }),
    ]).then((fns) => { unlisteners = fns; });

    return () => { unlisteners.forEach((fn) => fn()); };
  });
</script>

<div class="app-layout">
  <DataPanel />
  <ChatPanel />
  <ArtifactDrawer />

  <!-- Persistent drawer toggle — visible even when drawer is closed -->
  {#if $artifacts.length > 0}
    <button
      class="drawer-tab"
      class:drawer-open={$drawerOpen}
      onclick={() => $drawerOpen ? drawerStore.close() : drawerStore.open()}
      title={$drawerOpen ? "Close artifacts" : "Open artifacts"}
    >
      <span class="tab-arrow">{$drawerOpen ? "›" : "‹"}</span>
      {#if !$drawerOpen}
        <span class="tab-count">{$artifacts.length}</span>
      {/if}
    </button>
  {/if}

  {#if isDragOver}
    <div class="drop-overlay">
      <div class="drop-hint">
        <span class="drop-icon">&#x2B07;</span>
        <span>Drop file to load</span>
      </div>
    </div>
  {/if}
</div>

<!-- Modal rendered outside app-layout so it can't affect column flex layout -->
{#if $settingsOpen}
  <SettingsModal />
{/if}

<style>
  .app-layout {
    display: flex;
    flex-direction: row;
    flex: 1 1 0;
    height: 0;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  /* Drawer toggle tab — anchored to right edge of the viewport */
  .drawer-tab {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    width: 18px;
    height: 48px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-right: none;
    border-radius: var(--radius) 0 0 var(--radius);
    cursor: pointer;
    z-index: 20;
    color: var(--text-faint);
    transition: color 0.15s, border-color 0.15s;
  }

  .drawer-tab:hover {
    color: var(--accent-green);
    border-color: var(--accent-green);
  }

  /* When drawer is open, shift the tab to sit against the drawer's left edge */
  .drawer-tab.drawer-open {
    right: 480px;
  }

  .tab-arrow {
    font-size: 14px;
    line-height: 1;
  }

  .tab-count {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--accent-green);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 221, 119, 0.05);
    border: 2px dashed var(--accent-green);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .drop-hint {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: var(--accent-green);
    font-size: 18px;
  }

  .drop-icon {
    font-size: 48px;
  }
</style>
