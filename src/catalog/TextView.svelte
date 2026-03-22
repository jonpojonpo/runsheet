<script lang="ts">
  import type { TextComponent } from "../lib/catalog";
  let { spec }: { spec: TextComponent } = $props();

  function renderMarkdown(text: string): string {
    return text
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
      .replace(/\*\*(.*?)\*\*/gs, '<strong style="color:var(--accent-green);font-weight:600">$1</strong>')
      .replace(/`(.*?)`/g, '<code style="font-family:var(--font-mono);background:var(--surface-2);padding:1px 4px;border-radius:3px">$1</code>');
  }
</script>

<div class="text-view" class:heading={spec.props.style === "heading"} class:caption={spec.props.style === "caption"} class:callout={spec.props.style === "callout"}>
  {@html renderMarkdown(spec.props.content)}
</div>

<style>
  .text-view { font-size: 13px; line-height: 1.6; color: var(--text); white-space: pre-wrap; }
  .heading { font-size: 16px; font-weight: 700; color: var(--text); margin-bottom: 8px; }
  .caption { font-size: 11px; color: var(--text-dim); }
  .callout { background: var(--surface-2); border-left: 3px solid var(--accent-green); padding: 10px 14px; border-radius: var(--radius); }
</style>
