import { mount } from "svelte";
import "./styles/global.css";
import App from "./App.svelte";
import { Chart, registerables } from "chart.js";

Chart.register(...registerables);
// Make available globally for ChartView components
(window as unknown as Record<string, unknown>).Chart = Chart;

mount(App, { target: document.getElementById("app")! });
