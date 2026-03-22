<script lang="ts">
  import { onMount } from "svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { getSettings, saveSettings, listModels } from "../lib/tauri";
  import type { SettingsPayload, ProviderKind } from "../lib/types";

  let settings: SettingsPayload = $state({
    provider: "anthropic",
    anthropic_api_key: null,
    anthropic_model: "claude-haiku-4-5-20251001",
    openai_api_key: null,
    openai_model: "gpt-4o-mini",
    mistral_api_key: null,
    mistral_model: "mistral-small-latest",
  });

  let saving = $state(false);
  let saved = $state(false);
  let error = $state<string | null>(null);

  let anthropicKey = $state("");
  let openaiKey = $state("");
  let mistralKey = $state("");

  // Dynamic model lists per provider (empty = not yet fetched)
  let modelLists: Record<ProviderKind, string[]> = $state({
    anthropic: [],
    openai: [],
    mistral: [],
  });
  let modelsLoading = $state(false);
  let modelsError = $state<string | null>(null);

  const providerMeta: Record<ProviderKind, { label: string; color: string; keyPlaceholder: string }> = {
    anthropic: { label: "Anthropic", color: "#DA291C", keyPlaceholder: "sk-ant-…" },
    openai:    { label: "OpenAI",    color: "#10A37F", keyPlaceholder: "sk-…"     },
    mistral:   { label: "Mistral",   color: "#F7731C", keyPlaceholder: "…"        },
  };

  const providers: ProviderKind[] = ["anthropic", "openai", "mistral"];

  /** Shorten a raw model ID into something readable */
  function displayName(id: string): string {
    return id
      .replace(/^claude-/, "")
      .replace(/-\d{8,}$/, "")          // strip long date suffixes
      .replace(/-latest$/, "")
      .replace(/-(\d+)-(\d+)$/, " $1.$2") // -4-6 → 4.6
      .replace(/-/g, " ");
  }

  const activeModel = $derived(
    settings.provider === "anthropic" ? settings.anthropic_model :
    settings.provider === "openai"    ? settings.openai_model :
                                        settings.mistral_model
  );

  function setModel(id: string) {
    if (settings.provider === "anthropic") settings.anthropic_model = id;
    else if (settings.provider === "openai") settings.openai_model = id;
    else settings.mistral_model = id;
  }

  const activeKey = $derived(
    settings.provider === "anthropic" ? anthropicKey :
    settings.provider === "openai"    ? openaiKey :
                                        mistralKey
  );

  function setKey(v: string) {
    if (settings.provider === "anthropic") anthropicKey = v;
    else if (settings.provider === "openai") openaiKey = v;
    else mistralKey = v;
  }

  async function fetchModels(provider: ProviderKind = settings.provider) {
    const key = provider === "anthropic" ? anthropicKey
              : provider === "openai"    ? openaiKey
              :                            mistralKey;
    if (!key.trim()) { modelsError = "Enter an API key first"; return; }
    modelsLoading = true;
    modelsError = null;
    try {
      const ids = await listModels(provider, key);
      modelLists[provider] = ids;
      // If current model not in list, default to first
      const current = provider === "anthropic" ? settings.anthropic_model
                    : provider === "openai"    ? settings.openai_model
                    :                            settings.mistral_model;
      if (ids.length > 0 && !ids.includes(current)) setModel(ids[0]);
    } catch (e: unknown) {
      modelsError = (e as { message?: string })?.message ?? String(e);
    } finally {
      modelsLoading = false;
    }
  }

  // Re-fetch when switching to a provider whose list hasn't loaded yet
  $effect(() => {
    const p = settings.provider;
    if (modelLists[p].length === 0) fetchModels(p);
  });

  onMount(async () => {
    try {
      const s = await getSettings();
      settings = s;
      anthropicKey = s.anthropic_api_key ?? "";
      openaiKey = s.openai_api_key ?? "";
      mistralKey = s.mistral_api_key ?? "";
      fetchModels(s.provider);
    } catch {
      // Non-fatal
    }
  });

  async function handleSave() {
    saving = true;
    error = null;
    try {
      await saveSettings({
        ...settings,
        anthropic_api_key: anthropicKey || null,
        openai_api_key:    openaiKey    || null,
        mistral_api_key:   mistralKey   || null,
      });
      saved = true;
      setTimeout(() => { saved = false; settingsStore.close(); }, 1200);
    } catch (e: unknown) {
      error = (e as { message?: string })?.message ?? String(e);
    } finally {
      saving = false;
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) settingsStore.close();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && settingsStore.close()} />

<div class="overlay" role="dialog" aria-modal="true" onclick={handleOverlayClick}>
  <div class="modal">

    <div class="modal-header">
      <div class="modal-title-row">
        <img src="/icon.png" alt="Runsheet" class="app-icon" />
        <span class="modal-title mono">SETTINGS</span>
      </div>
      <button class="close-btn" onclick={() => settingsStore.close()}>×</button>
    </div>

    <div class="modal-body">

      <!-- Provider -->
      <div class="field">
        <label class="field-label">Provider</label>
        <div class="provider-row">
          {#each providers as p}
            {@const cfg = providerMeta[p]}
            <button
              class="provider-btn"
              class:active={settings.provider === p}
              style="--p-color: {cfg.color}"
              onclick={() => settings.provider = p}
            >{cfg.label}</button>
          {/each}
        </div>
      </div>

      <!-- API Key -->
      <div class="field">
        <label class="field-label">{providerMeta[settings.provider].label} API Key</label>
        <input
          class="field-input mono"
          type="password"
          placeholder={providerMeta[settings.provider].keyPlaceholder}
          value={activeKey}
          oninput={(e) => setKey((e.target as HTMLInputElement).value)}
          autocomplete="off"
        />
      </div>

      <!-- Model -->
      <div class="field">
        <div class="model-header">
          <label class="field-label">Model</label>
          <button
            class="refresh-btn mono"
            onclick={() => fetchModels()}
            disabled={modelsLoading}
            title="Fetch latest models from API"
          >{modelsLoading ? "…" : "⟳ refresh"}</button>
        </div>

        {#if modelsError}
          <div class="models-error">{modelsError}</div>
        {/if}

        {#if modelLists[settings.provider].length > 0}
          <div class="model-grid">
            {#each modelLists[settings.provider] as id}
              <button
                class="model-btn"
                class:active={activeModel === id}
                onclick={() => setModel(id)}
                title={id}
              >{displayName(id)}</button>
            {/each}
          </div>
        {:else if !modelsLoading}
          <input
            class="field-input mono"
            type="text"
            placeholder="model-id"
            value={activeModel}
            oninput={(e) => setModel((e.target as HTMLInputElement).value)}
          />
        {/if}
      </div>

      {#if error}
        <div class="save-error mono">{error}</div>
      {/if}

    </div>

    <div class="modal-footer">
      <button class="cancel-btn" onclick={() => settingsStore.close()}>Cancel</button>
      <button class="save-btn" onclick={handleSave} disabled={saving}>
        {#if saved}✓ Saved{:else if saving}Saving…{:else}Save{/if}
      </button>
    </div>

  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    width: 420px;
    background: var(--surface);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 32px 64px rgba(0, 0, 0, 0.7);
  }

  .modal-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .app-icon {
    width: 18px;
    height: 18px;
    image-rendering: crisp-edges;
  }

  .modal-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--accent);
  }

  .close-btn {
    font-size: 18px;
    line-height: 1;
    padding: 0 4px;
    color: var(--text-faint);
  }
  .close-btn:hover { color: var(--text); }

  .modal-body {
    padding: 20px 16px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .field { display: flex; flex-direction: column; gap: 8px; }

  .field-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-faint);
    text-transform: uppercase;
  }

  /* Provider */
  .provider-row { display: flex; gap: 6px; }

  .provider-btn {
    flex: 1;
    padding: 8px 4px;
    border: 1px solid var(--border-active);
    border-radius: var(--radius);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    background: var(--surface-2);
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }
  .provider-btn:hover { border-color: var(--p-color); color: var(--text); }
  .provider-btn.active {
    border-color: var(--p-color);
    color: var(--p-color);
    background: color-mix(in srgb, var(--p-color) 10%, var(--surface-2));
  }

  /* Model */
  .model-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .refresh-btn {
    font-size: 10px;
    color: var(--text-faint);
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: color 0.15s, border-color 0.15s;
  }
  .refresh-btn:hover:not(:disabled) { color: var(--accent); border-color: var(--accent); }
  .refresh-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .model-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    max-height: 160px;
    overflow-y: auto;
  }

  .model-btn {
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-dim);
    white-space: nowrap;
    transition: border-color 0.1s, color 0.1s, background 0.1s;
  }
  .model-btn:hover { border-color: var(--border-active); color: var(--text); }
  .model-btn.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-dim);
  }

  .models-error {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--accent-muted);
  }

  /* API key input */
  .field-input {
    padding: 8px 10px;
    border-radius: var(--radius);
    font-size: 12px;
    width: 100%;
  }
  .field-input:focus { border-color: var(--accent); }

  .save-error {
    font-size: 11px;
    color: var(--color-error);
    padding: 6px 10px;
    background: rgba(255, 68, 68, 0.08);
    border-radius: var(--radius);
    border: 1px solid rgba(255, 68, 68, 0.3);
  }

  .modal-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .cancel-btn {
    padding: 7px 16px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-faint);
  }
  .cancel-btn:hover { border-color: var(--border-active); color: var(--text); }

  .save-btn {
    padding: 7px 20px;
    font-size: 12px;
    font-weight: 600;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: var(--radius);
    min-width: 80px;
    transition: opacity 0.15s;
  }
  .save-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .save-btn:not(:disabled):hover { opacity: 0.85; }
</style>
