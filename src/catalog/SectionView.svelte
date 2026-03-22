<script lang="ts">
  import type { SectionComponent } from "../lib/catalog";
  import CatalogRenderer from "./CatalogRenderer.svelte";
  let { spec }: { spec: SectionComponent } = $props();
  let collapsed = $state(false);
</script>

<div class="section">
  <div class="section-header" role="button" tabindex="0" onclick={() => collapsed = !collapsed} onkeydown={(e) => e.key === "Enter" && (collapsed = !collapsed)}>
    {#if spec.props.collapsible}<span class="arrow" class:collapsed>{collapsed ? "▶" : "▼"}</span>{/if}
    <span class="section-title">{spec.props.title}</span>
  </div>
  {#if !collapsed}
    <div class="section-body">
      <CatalogRenderer components={spec.props.components} />
    </div>
  {/if}
</div>

<style>
  .section { border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
  .section-header { display: flex; align-items: center; gap: 8px; padding: 10px 14px; cursor: pointer; background: var(--surface); user-select: none; }
  .section-header:hover { background: var(--surface-2); }
  .section-title { font-size: 12px; font-weight: 600; color: var(--text-dim); }
  .arrow { font-size: 9px; color: var(--text-faint); }
  .section-body { padding: 14px; display: flex; flex-direction: column; gap: 16px; }
</style>
