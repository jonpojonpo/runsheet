<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  let { html, title }: { html: string; title: string } = $props();

  let blobUrl = $state("");

  onMount(() => {
    const blob = new Blob([html], { type: "text/html" });
    blobUrl = URL.createObjectURL(blob);
  });

  onDestroy(() => {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
  });
</script>

{#if blobUrl}
  <iframe
    src={blobUrl}
    sandbox="allow-scripts"
    class="html-frame"
    {title}
  ></iframe>
{/if}

<style>
  .html-frame {
    width: 100%;
    height: 300px;
    border: none;
    background: white;
    border-radius: var(--radius);
  }
</style>
