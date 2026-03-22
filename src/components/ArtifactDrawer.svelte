<script lang="ts">
  import { tick } from "svelte";
  import { drawerOpen, drawerStore } from "../stores/drawer.svelte";
  import { artifacts } from "../stores/artifacts.svelte";
  import CatalogRenderer from "../catalog/CatalogRenderer.svelte";
  import HtmlFrame from "../catalog/HtmlFrame.svelte";
  import ArtifactToolbar from "./ArtifactToolbar.svelte";

  let scrollEl: HTMLDivElement;

  async function scrollToBottom() {
    await tick();
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  }

  // Scroll to bottom whenever a new artifact is added
  $effect(() => {
    $artifacts.length;
    scrollToBottom();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && $drawerOpen) drawerStore.close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<aside class="artifact-drawer" class:open={$drawerOpen}>
  {#if $drawerOpen && $artifacts.length > 0}
    <div class="drawer-header">
      <span class="drawer-title">ARTIFACTS</span>
      <span class="count text-faint">{$artifacts.length}</span>
      <button class="close-btn" onclick={() => drawerStore.close()} title="Close (Esc)">×</button>
    </div>

    <div class="drawer-scroll" bind:this={scrollEl}>
      {#each $artifacts as artifact (artifact.id)}
        <div class="artifact-block">
          <div class="artifact-title-row">
            <span class="artifact-title">{artifact.title}</span>
            {#if artifact.description}
              <span class="artifact-desc text-faint">{artifact.description}</span>
            {/if}
          </div>
          <div class="artifact-body">
            {#if artifact.kind === "structured" && artifact.spec}
              {#key artifact.id}
                <CatalogRenderer spec={artifact.spec} />
              {/key}
            {:else if artifact.kind === "html" && artifact.html}
              <HtmlFrame html={artifact.html} title={artifact.title} />
            {:else}
              <p class="text-faint">No content</p>
            {/if}
          </div>
          <ArtifactToolbar {artifact} />
        </div>
      {/each}
    </div>
  {/if}
</aside>

<style>
  .artifact-drawer {
    width: 0;
    min-width: 0;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    flex-shrink: 0;
  }

  .artifact-drawer.open {
    width: 480px;
    min-width: 320px;
    border-left: 1px solid var(--border);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--surface);
  }

  .drawer-title {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    color: var(--accent-green);
    opacity: 0.7;
  }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    flex: 1;
  }

  .close-btn {
    font-size: 20px;
    color: var(--text-faint);
    line-height: 1;
    padding: 0 4px;
    cursor: pointer;
  }
  .close-btn:hover { color: var(--accent-red); }

  .drawer-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .artifact-block {
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }

  .artifact-title-row {
    padding: 12px 16px 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .artifact-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .artifact-desc {
    font-size: 11px;
    line-height: 1.4;
  }

  .artifact-body {
    padding: 0 16px 12px;
  }

</style>
