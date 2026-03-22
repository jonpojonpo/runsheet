<script lang="ts">
  import type { ArtifactSpec } from "../lib/catalog";
  import CatalogRenderer from "./CatalogRenderer.svelte";
  let { spec }: { spec: ArtifactSpec } = $props();
</script>

<div class="dashboard" class:grid-2={spec.layout === "grid_2col"} class:grid-3={spec.layout === "grid_3col"}>
  <CatalogRenderer components={spec.components} />
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 20px; }
  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .grid-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 16px; }
</style>
