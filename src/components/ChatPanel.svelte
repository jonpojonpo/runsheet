<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { messages, isLoading, activeToolCall, streamingText, chatStore } from "../stores/chat.svelte";
  import { artifactsStore } from "../stores/artifacts.svelte";
  import { drawerStore } from "../stores/drawer.svelte";
  import { chat, type Message } from "../lib/tauri";
  import ChatMessage from "./ChatMessage.svelte";
  import { renderMarkdown } from "../lib/markdown";

  let inputValue = $state("");
  let messagesEl: HTMLDivElement | undefined;

  onMount(() => {
    let unlisteners: Array<() => void> = [];

    Promise.all([
      listen<{ tool_name: string; input: Record<string, unknown> }>("chat:tool_call", (event) => {
        const query = (event.payload.input?.query as string) ?? "";
        chatStore.setActiveToolCall(query);
      }),
      listen("chat:done", () => {
        chatStore.setActiveToolCall(null);
      }),
      listen<{ text: string }>("chat:token", (event) => {
        streamingText.update((s) => s + event.payload.text);
        scrollToBottom();
      }),
    ]).then((fns) => { unlisteners = fns; });

    return () => { unlisteners.forEach((fn) => fn()); };
  });

  async function scrollToBottom() {
    await tick();
    if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  async function sendMessage() {
    const content = inputValue.trim();
    if (!content || $isLoading) return;

    inputValue = "";
    chatStore.addUserMessage(content);
    chatStore.setLoading(true);
    await scrollToBottom();

    // Build message history for the API
    const apiMessages: Message[] = chatStore.getMessages()
      .filter((m) => m.role === "user" || m.role === "assistant")
      .map((m) => ({ role: m.role, content: m.content }));

    try {
      const response = await chat(apiMessages);
      chatStore.addAssistantMessage(response.text, response.tool_calls_made);
    } catch (e: unknown) {
      const msg = (e as { message?: string })?.message ?? String(e);
      chatStore.addAssistantMessage(`Error: ${msg}`);
    } finally {
      chatStore.setLoading(false);
      chatStore.setActiveToolCall(null);
      streamingText.set("");
      await scrollToBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  // Auto-scroll on new messages
  $effect(() => {
    if ($messages.length) scrollToBottom();
  });
</script>

<section class="chat-panel">
  <div class="chat-header">
    <span class="header-title">RUNSHEET</span>
    {#if $messages.length > 0}
      <button
        class="clear-btn text-faint"
        onclick={() => { chatStore.clear(); artifactsStore.clear(); drawerStore.close(); }}
        title="Clear conversation"
      >clear</button>
    {/if}
  </div>

  <div class="messages" bind:this={messagesEl}>
    {#if $messages.length === 0}
      <div class="welcome">
        <div class="welcome-logo">RUNSHEET</div>
        <p>Drop a CSV file onto the app, then ask a question.</p>
      </div>
    {:else}
      {#each $messages as message (message.id)}
        <ChatMessage {message} />
      {/each}
    {/if}

    {#if $isLoading && $streamingText}
      <div class="message assistant streaming">
        <div class="message-meta">
          <span class="role-label">RUNSHEET</span>
        </div>
        <div class="message-content">
          {@html renderMarkdown($streamingText)}<span class="stream-cursor">▌</span>
        </div>
      </div>
    {/if}

    {#if $isLoading}
      <div class="loading-indicator">
        {#if $activeToolCall}
          <span class="tool-running">
            <span class="spinner">⟳</span>
            <span class="mono text-amber">Running:</span>
            <code class="query-preview">{$activeToolCall.slice(0, 80)}{$activeToolCall.length > 80 ? "…" : ""}</code>
          </span>
        {:else}
          <span class="thinking">
            <span class="spinner">⟳</span>
            <span class="text-dim">Thinking…</span>
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <div class="input-area">
    <textarea
      bind:value={inputValue}
      onkeydown={handleKeydown}
      placeholder="Ask Runsheet…"
      rows={1}
      disabled={$isLoading}
    ></textarea>
    <button
      class="send-btn"
      onclick={sendMessage}
      disabled={!inputValue.trim() || $isLoading}
    >⏎</button>
  </div>
</section>

<style>
  .chat-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .chat-header {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    background: var(--surface);
  }

  .header-title {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: var(--accent-green);
  }

  .clear-btn {
    margin-left: auto;
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .clear-btn:hover { color: var(--accent-red); border-color: var(--accent-red); }

  .badge {
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 1px 6px;
    background: var(--surface-2);
    border: 1px solid var(--border-active);
    border-radius: var(--radius);
    color: var(--text-dim);
  }

  .messages {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: var(--text-dim);
    text-align: center;
  }

  .welcome-logo {
    font-family: var(--font-mono);
    font-size: 28px;
    font-weight: 700;
    color: var(--accent-green);
    letter-spacing: 0.15em;
    opacity: 0.6;
  }

  .loading-indicator {
    padding: 8px 0;
    display: flex;
    align-items: center;
  }

  .tool-running, .thinking {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    color: var(--accent-green);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .query-preview {
    font-size: 11px;
    color: var(--text-dim);
    font-family: var(--font-mono);
  }

  .input-area {
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    display: flex;
    gap: 8px;
    align-items: flex-end;
    background: var(--surface);
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    padding: 8px 12px;
    resize: none;
    min-height: 36px;
    max-height: 120px;
    line-height: 1.5;
    border-radius: var(--radius);
    field-sizing: content;
  }

  .send-btn {
    padding: 8px 14px;
    background: var(--accent);
    color: var(--accent-fg);
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 700;
    border-radius: var(--radius);
    line-height: 1;
    height: 36px;
    transition: opacity 0.15s;
  }

  .send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .send-btn:not(:disabled):hover { opacity: 0.85; }

  .message.streaming .message-content {
    white-space: normal;
    word-break: break-word;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text);
  }

  .message.streaming .message-content :global(.md-bold) { color: var(--accent-green); font-weight: 600; }
  .message.streaming .message-content :global(.md-code) {
    font-family: var(--font-mono);
    font-size: 11px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--accent-amber, #f5a623);
  }
  .message.streaming .message-content :global(.md-code-block) {
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

  .stream-cursor {
    display: inline-block;
    color: var(--accent-green);
    animation: blink 1s step-end infinite;
    font-weight: bold;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .message.assistant .message-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .message.assistant .role-label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--accent-green);
    opacity: 0.7;
  }

  .message.assistant {
    padding: 12px 0;
  }
</style>
