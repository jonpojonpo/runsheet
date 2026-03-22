<script lang="ts">
  import { onMount } from "svelte";
  import type { ChartComponent } from "../lib/catalog";

  let { spec }: { spec: ChartComponent } = $props();
  let wrapper: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let chartInstance: { destroy(): void } | null = null;
  let debugInfo = $state("");

  const COLORS = ["#DA291C", "#3399ff", "#ffbb33", "#9E9EA8", "#aa88ff", "#00bbff", "#ff8833"];

  function buildChart(Chart: any) {
    if (!canvas || !wrapper) return;

    // Set canvas buffer dimensions to match the wrapper's CSS dimensions.
    // This must happen BEFORE new Chart() so Chart.js draws at the right size.
    const w = wrapper.clientWidth || 400;
    const h = wrapper.clientHeight || 280;
    canvas.width = w;
    canvas.height = h;
    debugInfo = `${w}×${h}`;

    const { chart_type, title, data, x_label, y_label, stacked, show_legend } = spec.props;
    if (!data?.labels || !data?.datasets) {
      debugInfo = `bad spec: ${JSON.stringify(spec.props).slice(0, 100)}`;
      return;
    }

    const typeMap: Record<string, string> = { horizontal_bar: "bar", area: "line" };
    const chartJsType = typeMap[chart_type] ?? chart_type;

    const isPie = chartJsType === "pie" || chartJsType === "doughnut";

    const datasets = data.datasets.map((ds, i) => ({
      label: ds.label,
      data: ds.data,
      // Pie/doughnut: one colour per slice; everything else: one colour per dataset
      backgroundColor: isPie
        ? ds.data.map((_, j) => COLORS[j % COLORS.length])
        : ds.color ?? (chartJsType === "line" ? COLORS[i % COLORS.length] + "33" : COLORS[i % COLORS.length]),
      borderColor: isPie
        ? ds.data.map((_, j) => COLORS[j % COLORS.length])
        : ds.color ?? COLORS[i % COLORS.length],
      borderWidth: chartJsType === "line" ? 2 : 1,
      fill: chart_type === "area",
      tension: 0.3,
    }));

    try {
      chartInstance = new Chart(canvas, {
        type: chartJsType,
        data: { labels: data.labels, datasets },
        options: {
          indexAxis: chart_type === "horizontal_bar" ? "y" : "x",
          responsive: false,
          maintainAspectRatio: false,
          plugins: {
            legend: { display: show_legend ?? datasets.length > 1, labels: { color: "#888", font: { size: 11 } } },
            title: { display: !!title, text: title, color: "#e0e0e0", font: { size: 13 } },
          },
          scales: (chartJsType === "pie" || chartJsType === "doughnut") ? {} : {
            x: { stacked, grid: { color: "#2a2a2a" }, ticks: { color: "#888" } },
            y: { stacked, grid: { color: "#2a2a2a" }, ticks: { color: "#888" } },
          },
        },
      });
      debugInfo = `ok ${w}×${h}`;
    } catch (e) {
      debugInfo = `err: ${e}`;
    }
  }

  function exportPng() {
    if (!canvas) return;
    const a = document.createElement("a");
    a.href = canvas.toDataURL("image/png");
    a.download = (spec.props.title ?? "chart") + ".png";
    a.click();
  }

  onMount(() => {
    // @ts-ignore
    const Chart = window.Chart;
    if (!Chart) { debugInfo = "no window.Chart"; return; }

    // Two RAFs: first lets Svelte flush DOM, second lets the browser lay out CSS.
    let raf1 = requestAnimationFrame(() => {
      let raf2 = requestAnimationFrame(() => buildChart(Chart));
      return () => cancelAnimationFrame(raf2);
    });

    return () => {
      cancelAnimationFrame(raf1);
      chartInstance?.destroy();
      chartInstance = null;
    };
  });
</script>

<div class="chart-wrapper" bind:this={wrapper}>
  <canvas bind:this={canvas}></canvas>
  <button class="export-btn" onclick={exportPng} title="Save as PNG">⬇</button>
  {#if debugInfo}
    <div class="debug">{debugInfo}</div>
  {/if}
</div>

<style>
  .chart-wrapper {
    width: 100%;
    height: 280px;
    position: relative;
  }

  .export-btn {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 3px 7px;
    font-size: 12px;
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-faint);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
    z-index: 2;
  }

  .chart-wrapper:hover .export-btn { opacity: 1; }
  .export-btn:hover { color: var(--accent-green); }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
  .debug {
    position: absolute;
    bottom: 4px;
    right: 6px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--accent-green);
    background: rgba(0,0,0,0.6);
    padding: 2px 5px;
    border-radius: 3px;
    pointer-events: none;
  }
</style>
