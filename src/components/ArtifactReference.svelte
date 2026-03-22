<script lang="ts">
  import { drawerStore } from "../stores/drawer.svelte";
  import { artifacts } from "../stores/artifacts.svelte";
  import type { Artifact } from "../lib/types";

  let { artifactId }: { artifactId: string } = $props();

  const artifact: Artifact | undefined = $derived($artifacts.find((a) => a.id === artifactId));

  const componentCount = $derived(artifact?.spec?.components?.length ?? 0);

  function formatAge(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 60000) return "just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
  }
</script>

{#if artifact}
<button class="artifact-ref" onclick={() => drawerStore.open()}>
  <span class="icon">📊</span>
  <span class="title">{artifact.title}</span>
  <span class="arrow-open">↗</span>
  <span class="meta text-faint">
    {componentCount} component{componentCount !== 1 ? "s" : ""} · {formatAge(artifact.created_at)}
  </span>
</button>
{/if}

<style>
  .artifact-ref {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    width: 100%;
    max-width: 400px;
    margin-top: 6px;
    transition: border-color 0.15s;
  }
  .artifact-ref:hover { border-color: var(--accent-green); }
  .icon { font-size: 14px; }
  .title { font-size: 12px; font-weight: 600; color: var(--text); flex: 1; }
  .arrow-open { color: var(--accent-green); font-size: 12px; }
  .meta { font-size: 11px; }
</style>
