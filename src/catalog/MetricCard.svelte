<script lang="ts">
  import type { MetricComponent } from "../lib/catalog";

  let { spec }: { spec: MetricComponent } = $props();

  function formatValue(v: string | number, fmt?: string): string {
    if (typeof v === "string") return v;
    switch (fmt) {
      case "currency": return new Intl.NumberFormat("en-GB", { style: "currency", currency: "GBP", notation: "compact" }).format(v);
      case "percent": return `${v > 0 ? "+" : ""}${v.toFixed(1)}%`;
      case "compact": return new Intl.NumberFormat("en", { notation: "compact" }).format(v);
      default: return new Intl.NumberFormat("en").format(v);
    }
  }

  const displayValue = $derived(formatValue(spec.props.value, spec.props.format));
  const trendUp = $derived(spec.props.change !== undefined && spec.props.change > 0);
  const trendDown = $derived(spec.props.change !== undefined && spec.props.change < 0);
  const trendArrow = $derived(trendUp ? "▲" : trendDown ? "▼" : "—");
  const statusClass = $derived(spec.props.status ?? "neutral");
</script>

<div class="metric-card" class:good={statusClass === "good"} class:bad={statusClass === "bad"}>
  <div class="metric-label">{spec.props.label}</div>
  <div class="metric-value">{displayValue}</div>
  {#if spec.props.change !== undefined}
    <div class="metric-change" class:up={trendUp} class:down={trendDown}>
      <span class="arrow">{trendArrow}</span>
      <span>{Math.abs(spec.props.change).toFixed(1)}%</span>
      {#if spec.props.change_label}<span class="change-label">{spec.props.change_label}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .metric-card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    min-width: 140px;
  }
  .metric-card.good { border-color: var(--accent-green); }
  .metric-card.bad { border-color: var(--accent-red); }

  .metric-label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 6px;
  }
  .metric-value {
    font-size: 24px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text);
    margin-bottom: 4px;
  }
  .metric-change {
    font-size: 12px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .metric-change.up { color: var(--accent-green); }
  .metric-change.down { color: var(--accent-red); }
  .change-label { color: var(--text-faint); font-size: 11px; }
</style>
