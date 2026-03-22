<script lang="ts">
  import type { ChatMessage } from "../lib/types";
  import ArtifactReference from "./ArtifactReference.svelte";
  import { renderMarkdown } from "../lib/markdown";

  let { message }: { message: ChatMessage } = $props();

  let showTools = $state(false);

  function formatTime(d: Date): string {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
</script>

<div class="message" class:user={message.role === "user"} class:assistant={message.role === "assistant"}>
  <div class="message-meta">
    <span class="role-label">
      {message.role === "user" ? "YOU" : "RUNSHEET"}
    </span>
    <span class="timestamp text-faint">{formatTime(message.timestamp)}</span>
  </div>

  {#if message.tool_calls && message.tool_calls.length > 0}
    <div class="tool-calls">
      <button class="tool-toggle" onclick={() => showTools = !showTools}>
        <span class="arrow" class:open={showTools}>&#x25BA;</span>
        <span class="mono text-amber">{message.tool_calls.length} {message.tool_calls.length !== 1 ? "queries" : "query"} run</span>
      </button>
      {#if showTools}
        <div class="tool-list">
          {#each message.tool_calls as tc, i}
            <div class="tool-item">
              <div class="query-header mono text-faint">
                SQL {i + 1}
              </div>
              <pre class="query-sql">{String(tc.input?.query ?? "")}</pre>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="message-content">
    {@html renderMarkdown(message.content)}
  </div>

  {#if message.artifact_ids && message.artifact_ids.length > 0}
    <div class="artifact-refs">
      {#each message.artifact_ids as id}
        <ArtifactReference artifactId={id} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .message {
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .message:last-child { border-bottom: none; }

  .message-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .role-label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .user .role-label { color: var(--accent-orange); }
  .assistant .role-label { color: var(--accent-green); }

  .timestamp { font-size: 11px; }

  .message-content {
    font-size: 13px;
    line-height: 1.6;
    white-space: normal;
    word-break: break-word;
  }

  .message-content :global(.md-h1) { font-size: 16px; font-weight: 700; color: var(--text); margin: 8px 0 4px; }
  .message-content :global(.md-h2) { font-size: 14px; font-weight: 700; color: var(--text); margin: 8px 0 4px; }
  .message-content :global(.md-h3) { font-size: 13px; font-weight: 600; color: var(--text-dim); margin: 6px 0 4px; }
  .message-content :global(.md-h4),
  .message-content :global(.md-h5),
  .message-content :global(.md-h6) { font-size: 12px; font-weight: 600; color: var(--text-dim); margin: 4px 0; }
  .message-content :global(.md-bold) { color: var(--accent-green); font-weight: 600; }
  .message-content :global(.md-em) { font-style: italic; color: var(--text-dim); }
  .message-content :global(.md-code) {
    font-family: var(--font-mono);
    font-size: 11px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--accent-amber, #f5a623);
  }
  .message-content :global(.md-code-block) {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    margin: 6px 0;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
    white-space: pre;
  }
  .message-content :global(.md-ul),
  .message-content :global(.md-ol) { padding-left: 20px; margin: 4px 0; }
  .message-content :global(.md-ul li),
  .message-content :global(.md-ol li) { margin: 2px 0; line-height: 1.5; }

  .message-content :global(.md-table) {
    border-collapse: collapse;
    font-size: 12px;
    margin: 8px 0;
    width: max-content;
    max-width: 100%;
  }
  .message-content :global(.md-table th) {
    padding: 5px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border-active);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    text-align: left;
    white-space: nowrap;
  }
  .message-content :global(.md-table td) {
    padding: 4px 10px;
    border: 1px solid var(--border);
    color: var(--text);
    vertical-align: top;
  }
  .message-content :global(.md-table tr:hover td) {
    background: var(--surface-2);
  }

  .tool-calls {
    margin-bottom: 8px;
  }

  .tool-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 0;
    font-size: 12px;
    cursor: pointer;
    color: var(--text-dim);
  }

  .tool-toggle:hover { color: var(--text); }

  .arrow {
    font-size: 9px;
    transition: transform 0.15s;
  }

  .arrow.open { transform: rotate(90deg); }

  .tool-list {
    margin-top: 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .tool-item {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .tool-item:last-child { border-bottom: none; }

  .query-header {
    font-size: 10px;
    margin-bottom: 4px;
  }

  .query-sql {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    line-height: 1.5;
  }

  .artifact-refs {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
