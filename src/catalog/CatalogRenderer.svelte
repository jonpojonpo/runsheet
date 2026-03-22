<script lang="ts">
  import type { ComponentSpec, ArtifactSpec } from "../lib/catalog";
  import MetricCard from "./MetricCard.svelte";
  import DataTable from "./DataTable.svelte";
  import ChartView from "./ChartView.svelte";
  import TextView from "./TextView.svelte";
  import DividerView from "./DividerView.svelte";
  import SectionView from "./SectionView.svelte";

  let { spec, components }: { spec?: ArtifactSpec; components?: ComponentSpec[] } = $props();

  // Accept either a full spec or a components array directly
  const items: ComponentSpec[] = $derived(components ?? spec?.components ?? []);

  // For metric_group — render metrics in a row
  const isMetricGroup = $derived(spec?.type === "metric_group");
</script>

{#if isMetricGroup}
  <div class="metric-row">
    {#each items as item}
      {#if item.component === "metric"}
        <MetricCard spec={item} />
      {/if}
    {/each}
  </div>
{:else}
  <div class="catalog-stack">
    {#each items as item}
      {#if item.component === "metric"}
        <MetricCard spec={item} />
      {:else if item.component === "data_table"}
        <DataTable spec={item} />
      {:else if item.component === "chart"}
        <ChartView spec={item} />
      {:else if item.component === "text"}
        <TextView spec={item} />
      {:else if item.component === "divider"}
        <DividerView />
      {:else if item.component === "section"}
        <SectionView spec={item} />
      {:else}
        <div class="unknown-component">unknown component: "{(item as Record<string,unknown>).component ?? JSON.stringify(item).slice(0,120)}"</div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .catalog-stack { display: flex; flex-direction: column; gap: 20px; }
  .metric-row { display: flex; gap: 16px; flex-wrap: wrap; }
  .unknown-component {
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-amber);
    background: var(--surface-2);
    border: 1px solid var(--accent-amber);
    border-radius: var(--radius);
    word-break: break-all;
  }
</style>
