# RUNSHEET v2.1 — Tauri App Specification

> *Your spreadsheet. Your question. Your answer. Before your coffee gets cold.*

---

## Project Overview

**Runsheet** is a desktop application that lets a user:

1. Drag-and-drop `.xlsx` / `.csv` / `.parquet` files into the app
2. Automatically load them into an embedded DuckDB database
3. Chat with an LLM about the data in a persistent conversation
4. The LLM queries data via SQL tool calls and generates **artifacts** — interactive tables, charts, dashboards, and documents
5. Artifacts render in a **slide-out drawer** alongside the chat, with full interactivity (sort, filter, pivot on tables), provenance tracking, and export

**Stack:** Tauri v2 (Rust) + Svelte 5 + TanStack Table + native DuckDB + Claude API + Agent Skills

**Output model:** Two tiers — (1) a **structured component catalog** (tables, charts, metrics, documents) where the LLM outputs a JSON/YAML spec that Svelte renders natively with full interactivity, and (2) a **raw HTML fallback** for freeform reports the catalog can't express. The structured path is preferred; HTML is the escape hatch.

**Aesthetic:** Dark command-center theme with restrained NERV/Evangelion influence. Professional default, theatrical optional.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  TAURI v2 — Rust Backend                                     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐ │
│  │  DuckDB      │  │  LLM Client  │  │  Skills Engine     │ │
│  │  (native)    │  │  (reqwest)   │  │  (fs scanner)      │ │
│  │              │  │              │  │                    │ │
│  │  - ingest    │  │  - Anthropic │  │  - discovery       │ │
│  │  - query     │  │  - Ollama    │  │  - SKILL.md parse  │ │
│  │  - schema    │  │  - tool loop │  │  - activation      │ │
│  │  - persist   │  │  - streaming │  │                    │ │
│  └──────┬───────┘  └──────┬───────┘  └────────┬───────────┘ │
│         │                 │                    │             │
│  ───────┴─────────────────┴────────────────────┴──────────── │
│                    Tauri Command API                          │
│  ──────────────────────────────────────────────────────────── │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  SVELTE 5 — Frontend (WebView)                         │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────────┐  ┌──────────────────┐ │  │
│  │  │  Data    │  │  Chat Panel  │  │  Artifact Drawer │ │  │
│  │  │  Panel   │  │  + messages  │  │  (slides out →)  │ │  │
│  │  │  + drop  │  │  + tool log  │  │  + DataTable     │ │  │
│  │  │  + schema│  │  + input     │  │  + Charts        │ │  │
│  │  │          │  │              │  │  + Documents     │ │  │
│  │  │          │  │              │  │  + HTML fallback │ │  │
│  │  └──────────┘  └──────────────┘  └──────────────────┘ │  │
│  │                                                        │  │
│  │  Component Catalog: Metric, DataTable, Chart,          │  │
│  │  KPICard, Document — rendered natively by Svelte       │  │
│  │  with TanStack Table for sort/filter/group/pivot       │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### The Key Architectural Decision

**DuckDB runs natively in Rust, not as WASM in the browser.**

Rationale:
- This is a desktop app — there's a Rust process right there, use it
- Native DuckDB has full extension support, including the `excel` extension for xlsx
- Persistence is trivial — a single `.duckdb` file per workspace
- No WASM extension loading weirdness in packaged builds
- Cleaner security boundary — the frontend never holds the raw engine
- The Svelte frontend becomes a pure view layer talking over Tauri commands
- Performance headroom is vastly better for large files

The frontend communicates with DuckDB exclusively through Tauri commands:
```
ingest_file(path) → table_name
run_sql(query) → ResultSet
get_schema() → Vec<TableMeta>
profile_table(name) → ProfileStats
drop_table(name) → ()
```

### Technology Choices

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Shell | Tauri v2 | Tiny binary, native fs/dialog, Rust backend |
| Frontend | Svelte 5 | Minimal JS, runes reactivity, no framework bloat |
| Data Engine | DuckDB (native, via `duckdb-rs`) | Full extension support, single-file persistence, analytical SQL |
| Tables | TanStack Table (via `@tanstack/svelte-table`) | Headless, sortable/filterable/groupable, Svelte adapter |
| Charts | Chart.js (bundled as app asset) | No CDN dependency, works offline |
| LLM (cloud) | Anthropic Claude API | Best at structured output, tool_use, spec generation |
| LLM (local) | Ollama (optional, v2+) | Privacy mode, local GPU |
| Styling | Hand-rolled CSS | NERV-inspired — no framework captures this |

---

## Core Features

### 1. Data Ingestion

```
User drags file → Tauri backend receives path →
DuckDB native loads file → auto-creates table → schema returned to frontend
```

**Supported formats and strategy:**

| Format | Ingestion Method | Reliability |
|--------|-----------------|-------------|
| `.csv` / `.tsv` | DuckDB `read_csv()` | Rock solid |
| `.parquet` | DuckDB `read_parquet()` | Rock solid |
| `.json` (newline-delimited) | DuckDB `read_json()` | Rock solid |
| `.xlsx` | DuckDB `excel` extension (`read_xlsx()`) | Primary path — requires extension install on first run |
| `.xlsx` (fallback) | Rust `calamine` crate → CSV temp file → DuckDB `read_csv()` | Guaranteed fallback if extension fails |

**xlsx strategy in detail:**
- On first run, attempt to `INSTALL excel; LOAD excel;` in DuckDB
- If successful, use `read_xlsx()` directly — this is the clean path
- If extension install fails (corporate firewall, air-gapped machine), fall back to:
  - Rust `calamine` reads the xlsx → extracts sheet data → writes temp CSV → DuckDB reads CSV
  - User is informed: "Using fallback xlsx reader (some formatting metadata may be lost)"
- **Test the extension in the actual packaged build on Windows, macOS, Linux before shipping**
- Never promise xlsx in marketing materials until the packaged build is proven on all three platforms

**On successful load:**
- Run `DESCRIBE {table}` → column names and types
- Run `SUMMARIZE {table}` → basic stats (min, max, mean, nulls, approx distinct)
- Run `SELECT * FROM {table} LIMIT 5` → sample rows
- Store metadata: filename, row count, column names/types, load timestamp
- Table name derived from filename, sanitised: `Q1_Sales_2025.xlsx` → `q1_sales_2025`
- Multiple files can be loaded simultaneously — each becomes a named table

**Data panel (left sidebar):**
- Lists all loaded tables with row counts
- Expandable to show columns + types
- Click column for quick profile (top 10 values, null %, min/max/mean for numerics)
- "×" to drop a table
- Drag a new file anywhere on the app to ingest

### 2. Chat Interface with Tool-Use Loop

A persistent conversation between the user and the LLM about the loaded data. The LLM queries the data via tool calls — it never executes SQL directly.

**The tool-use loop:**

```
1. User types a question
2. App builds API request:
   - System prompt: RUNSHEET instructions + schema + stats + active skill (if any)
   - Messages: conversation history + new user message
   - Tools: [run_sql, generate_report]
3. Send to Claude API
4. If response contains tool_use (run_sql):
   a. Execute the SQL against native DuckDB in Rust
   b. Return tool_result with query results (or error message)
   c. Send back to Claude for next turn
   d. Repeat if Claude calls another tool
5. If response contains tool_use (generate_report):
   a. Extract the HTML
   b. Create an Artifact record (see §3)
   c. Render in sandbox
6. If response contains text:
   a. Render in chat
7. Update conversation history with all messages
```

**Why tool_use, not markdown parsing:**
- Gives a clean execution boundary — the app controls what runs
- Handles multi-step queries naturally (LLM can chain: query → inspect → refine → report)
- Error handling is explicit — DuckDB error messages go back as tool_result
- No ambiguity about "is this SQL or a code example?"
- Audit trail is automatic — every tool call is logged

**Tool definitions:**

The LLM has three tools. The first two are the workhorses — `run_sql` for data access, `generate_artifact` for structured output that renders natively. The third, `generate_html`, is a fallback for freeform content the catalog can't express.

```json
[
  {
    "name": "run_sql",
    "description": "Execute a DuckDB SQL query against the loaded data tables. Returns results as JSON rows. Use SELECT queries to explore data. Use DuckDB syntax — supports window functions, CTEs, PIVOT, UNPIVOT, SUMMARIZE, etc.",
    "input_schema": {
      "type": "object",
      "properties": {
        "query": {
          "type": "string",
          "description": "DuckDB-compatible SQL query"
        }
      },
      "required": ["query"]
    }
  },
  {
    "name": "generate_artifact",
    "description": "Generate a structured artifact (table, chart, dashboard, document) using the Runsheet component catalog. The artifact renders natively in the app with full interactivity — tables are sortable/filterable, charts are zoomable. PREFERRED over generate_html for all standard data presentations.",
    "input_schema": {
      "type": "object",
      "properties": {
        "title": {
          "type": "string",
          "description": "Human-readable title for this artifact"
        },
        "description": {
          "type": "string",
          "description": "Brief description of what this artifact shows"
        },
        "spec": {
          "type": "object",
          "description": "Component spec following the Runsheet catalog schema. See catalog documentation below.",
          "properties": {
            "type": {
              "type": "string",
              "enum": ["dashboard", "data_table", "chart", "document", "metric_group"],
              "description": "Top-level artifact type"
            },
            "components": {
              "type": "array",
              "description": "Array of component specs that compose the artifact",
              "items": { "type": "object" }
            }
          },
          "required": ["type", "components"]
        }
      },
      "required": ["title", "spec"]
    }
  },
  {
    "name": "generate_html",
    "description": "Generate a self-contained HTML document for freeform reports that the component catalog cannot express (e.g. custom layouts, Sankey diagrams, annotated prose documents with embedded visuals). Use generate_artifact for standard tables, charts, dashboards, and metric displays. Include inline CSS and use the bundled Chart.js (available as window.Chart). No external script tags or CDN links.",
    "input_schema": {
      "type": "object",
      "properties": {
        "title": {
          "type": "string",
          "description": "Human-readable title for this report"
        },
        "html": {
          "type": "string",
          "description": "Complete self-contained HTML document with inline styles"
        },
        "description": {
          "type": "string",
          "description": "Brief description of what this report shows"
        }
      },
      "required": ["title", "html"]
    }
  }
]
```

### Component Catalog

The structured output path. Instead of the LLM generating raw HTML, it outputs a spec that maps to Svelte components. The app renders these natively — which means interactivity (sort, filter, pivot) comes for free, styling is always consistent, and there's no script injection risk.

**Design philosophy (inspired by Vercel's json-render):**
- The LLM can only compose from the catalog — no arbitrary markup
- Every spec is validated against a schema before rendering
- Components render natively in Svelte, not in an iframe
- Interactive features (sort, filter, group) are properties of the components, not the LLM's problem
- Specs are compact (lower token cost than full HTML) and diffable (revision comparison works)

**Catalog components:**

```typescript
// === Top-level artifact types ===

interface DashboardSpec {
  type: "dashboard";
  components: ComponentSpec[];        // laid out vertically, or in a grid
  layout?: "stack" | "grid_2col" | "grid_3col";
}

interface DataTableSpec {
  type: "data_table";
  components: [DataTableComponent];   // single table artifact
}

interface ChartSpec {
  type: "chart";
  components: [ChartComponent];       // single chart artifact
}

interface DocumentSpec {
  type: "document";
  components: ComponentSpec[];        // rich text + embedded tables/charts
}

interface MetricGroupSpec {
  type: "metric_group";
  components: MetricComponent[];      // row of KPI cards
}

// === Individual components ===

interface MetricComponent {
  component: "metric";
  props: {
    label: string;                    // "Total Revenue"
    value: string | number;           // "£4.2M" or 4200000
    format?: "currency" | "percent" | "number" | "compact";
    change?: number;                  // +12.3 (percentage change, shows trend arrow)
    change_label?: string;            // "vs last quarter"
    status?: "good" | "bad" | "neutral";
  };
}

interface DataTableComponent {
  component: "data_table";
  props: {
    columns: Array<{
      key: string;                    // field name in row data
      label: string;                  // display header
      type?: "string" | "number" | "currency" | "percent" | "date";
      align?: "left" | "right" | "center";
      sortable?: boolean;             // default true
      filterable?: boolean;           // default true
      width?: string;                 // "200px" or "20%"
    }>;
    rows: Array<Record<string, any>>; // the data
    default_sort?: { key: string; direction: "asc" | "desc" };
    default_filter?: { key: string; value: string };
    groupable?: boolean;              // enable group-by UI
    page_size?: number;               // default 25
    summary_row?: Record<string, any>; // totals row at bottom
    highlight_rules?: Array<{
      column: string;
      condition: "gt" | "lt" | "eq" | "contains";
      value: any;
      style: "good" | "bad" | "warning" | "emphasis";
    }>;
  };
}

interface ChartComponent {
  component: "chart";
  props: {
    chart_type: "bar" | "line" | "pie" | "doughnut" | "scatter" | "area" | "horizontal_bar";
    title?: string;
    data: {
      labels: string[];
      datasets: Array<{
        label: string;
        data: number[];
        color?: string;               // override theme color
      }>;
    };
    x_label?: string;
    y_label?: string;
    stacked?: boolean;
    show_legend?: boolean;
    show_values?: boolean;            // data labels on bars/points
  };
}

interface TextComponent {
  component: "text";
  props: {
    content: string;                  // Markdown-formatted text
    style?: "body" | "heading" | "caption" | "callout";
  };
}

interface DividerComponent {
  component: "divider";
  props?: {};
}

interface SectionComponent {
  component: "section";
  props: {
    title: string;
    collapsible?: boolean;
    components: ComponentSpec[];       // nested components
  };
}

type ComponentSpec =
  | MetricComponent
  | DataTableComponent
  | ChartComponent
  | TextComponent
  | DividerComponent
  | SectionComponent;
```

**Example: LLM generates a dashboard artifact**

When the user asks "show me a sales dashboard by region":

```json
{
  "title": "Sales Dashboard by Region",
  "description": "Q1 2025 sales performance across all regions",
  "spec": {
    "type": "dashboard",
    "layout": "stack",
    "components": [
      {
        "component": "metric",
        "props": {
          "label": "Total Revenue", "value": 10600000,
          "format": "currency", "change": 12.3, "change_label": "vs Q4 2024",
          "status": "good"
        }
      },
      {
        "component": "metric",
        "props": {
          "label": "Avg Deal Size", "value": 47200,
          "format": "currency", "change": -3.1, "change_label": "vs Q4 2024",
          "status": "bad"
        }
      },
      {
        "component": "chart",
        "props": {
          "chart_type": "bar",
          "title": "Revenue by Region",
          "data": {
            "labels": ["EMEA", "APAC", "Americas"],
            "datasets": [{ "label": "Revenue", "data": [4200000, 3600000, 2800000] }]
          },
          "y_label": "Revenue (£)",
          "show_values": true
        }
      },
      {
        "component": "data_table",
        "props": {
          "columns": [
            { "key": "region", "label": "Region", "type": "string" },
            { "key": "revenue", "label": "Revenue", "type": "currency", "align": "right" },
            { "key": "deals", "label": "Deals", "type": "number", "align": "right" },
            { "key": "growth", "label": "YoY Growth", "type": "percent", "align": "right" }
          ],
          "rows": [
            { "region": "EMEA", "revenue": 4200000, "deals": 89, "growth": 12.3 },
            { "region": "APAC", "revenue": 3600000, "deals": 76, "growth": 18.1 },
            { "region": "Americas", "revenue": 2800000, "deals": 62, "growth": 5.7 }
          ],
          "default_sort": { "key": "revenue", "direction": "desc" },
          "groupable": true,
          "summary_row": { "region": "Total", "revenue": 10600000, "deals": 227, "growth": 12.0 }
        }
      }
    ]
  }
}
```

The app receives this, validates it against the catalog schema, and renders:
- MetricComponent → styled KPI cards with trend arrows
- ChartComponent → Chart.js bar chart with Runsheet theme
- DataTableComponent → **TanStack Table** with clickable column headers for sort, filter inputs, pagination — all instant, client-side, zero LLM calls

**The user can now click "Revenue" to sort descending, type "EMEA" in the filter, group by another column — without asking the LLM anything.** This is the critical UX advantage over raw HTML artifacts.

**Why the LLM still decides what to show:**
The LLM is great at: picking the right chart type, choosing which columns matter, computing derived metrics, writing narrative summaries, deciding layout. These are analytical decisions.

The LLM is terrible at: re-sorting a column (2s latency for a 5ms operation), paginating through rows, toggling a filter. These are interaction decisions. TanStack Table handles them.

### Artifact Drawer — The Fold-Out Model

Artifacts don't render inline in the chat. They render in a **slide-out drawer** that opens from the right side of the screen, alongside the chat. This is the same pattern used in Claude's artifact panel, VS Code's side panels, and most modern productivity tools.

**How the drawer works:**

```
┌──────────┬────────────────────────┬──────────────────────────────┐
│          │                        │                              │
│  DATA    │  CHAT                  │  ARTIFACT DRAWER             │
│  PANEL   │                        │  (slides in from right)      │
│          │  you: show me a        │                              │
│ ▸ q1_sales│  sales dashboard      │  ┌────────────────────────┐  │
│ ▸ returns │                       │  │ Sales Dashboard        │  │
│          │  runsheet:             │  │ by Region              │  │
│          │  ▶ ran: SELECT ...     │  │ ────────────────────── │  │
│          │                        │  │                        │  │
│          │  Here's the Q1 sales   │  │  £10.6M    £47.2K     │  │
│          │  overview. EMEA leads  │  │  ▲ 12.3%   ▼ 3.1%    │  │
│          │  at £4.2M...           │  │                        │  │
│          │                        │  │  ┌──────────────────┐ │  │
│          │  📊 Sales Dashboard    │  │  │  ████ EMEA £4.2M │ │  │
│          │     ↗ (click to view)  │  │  │  ███ APAC £3.6M  │ │  │
│          │                        │  │  │  ██ Amer £2.8M   │ │  │
│          │                        │  │  └──────────────────┘ │  │
│          │  you: break down EMEA  │  │                        │  │
│          │  by product            │  │  Region  Revenue  ▼   │  │
│          │                        │  │  ─────────────────── │  │
│          │                        │  │  EMEA    £4.2M  ↕   │  │
│          │                        │  │  APAC    £3.6M  ↕   │  │
│          │                        │  │  Amer    £2.8M  ↕   │  │
│          │                        │  │  [filter] [group]    │  │
│          │                        │  │                        │  │
│          │                        │  ├────────────────────────┤  │
│          │                        │  │ [Export] [Copy] [Pop]  │  │
│          │                        │  └────────────────────────┘  │
│          │                        │                              │
├──────────┴────────────────────────┴──────────────────────────────┤
│  Ask Runsheet...                                       [⏎] [/]  │
└──────────────────────────────────────────────────────────────────┘
```

**Drawer behaviour:**
- **Closed by default** — chat takes full width (minus data panel)
- **Opens when an artifact is generated** — slides in from right, chat panel narrows
- **Chat shows a compact reference** — "📊 Sales Dashboard ↗ (click to view)" — clicking opens the drawer if closed, or scrolls to that artifact
- **Drawer stays open** across conversation turns — the user can keep chatting while viewing the artifact
- **Multiple artifacts** — drawer has tabs or a stack. New artifacts push onto the stack. User can switch between them.
- **Close** — click X or press Esc, chat expands back
- **Pop out** — open artifact in a separate Tauri window for side-by-side viewing
- **Resize** — draggable divider between chat and drawer

**Drawer content varies by artifact type:**

| Artifact Type | Drawer Renders | Interactive Features |
|--------------|---------------|---------------------|
| `data_table` | TanStack Table | Sort, filter, group, pivot, paginate, resize columns |
| `chart` | Chart.js canvas | Hover tooltips, zoom (stretch), click-through |
| `dashboard` | Stack of metrics + charts + tables | All of the above, scrollable |
| `document` | Rendered markdown + embedded components | Collapsible sections, embedded tables are interactive |
| `metric_group` | Row of KPI cards | Click metric to drill down (sends follow-up to LLM) |
| HTML (fallback) | Sandboxed iframe | View only, export only |

**Artifact toolbar (bottom of drawer):**
- **[Export HTML]** — renders the structured spec to a self-contained HTML file (for email, SharePoint, etc.)
- **[Copy PNG]** — screenshot the drawer content
- **[Copy Data]** — for tables: copy as CSV/TSV to clipboard
- **[Pop Out]** — open in separate window
- **[Revisions]** — show revision chain for this artifact

**System prompt:**

```
You are Runsheet, a data analysis assistant embedded in a desktop application.

## Available Data
{for each loaded table:}
### Table: {name}
- Rows: {row_count}
- Columns:
  {col_name} ({col_type}) — sample: {top_3_values}
  ...
- Summary:
  {output of SUMMARIZE, formatted}

## Tools
You have three tools:

1. `run_sql` — Execute DuckDB SQL against the loaded tables. Use this to explore data, answer questions, compute aggregates, join tables. DuckDB supports: CTEs, window functions, PIVOT/UNPIVOT, SUMMARIZE, regexp, list/struct types, and more.

2. `generate_artifact` — **Preferred for all standard outputs.** Create an interactive artifact using the Runsheet component catalog. The artifact opens in a drawer alongside the chat. Available components:
   - `metric` — KPI card with value, trend, and status
   - `data_table` — Interactive table (users can sort, filter, group, pivot after generation)
   - `chart` — Bar, line, pie, doughnut, scatter, area, horizontal bar
   - `text` — Markdown-formatted narrative text
   - `section` — Collapsible container for grouping components
   - `divider` — Visual separator
   Top-level types: `dashboard` (multi-component), `data_table`, `chart`, `document`, `metric_group`

3. `generate_html` — **Fallback only.** Create a self-contained HTML document for custom layouts or visualisations the catalog cannot express (e.g. Sankey diagrams, geographic maps, complex annotated documents). Rules:
   - ALL styles must be inline or in a <style> block
   - Chart.js is pre-loaded — use `new Chart(ctx, config)` directly
   - No external script tags or CDN links
   - Use the Runsheet colour palette

## Behaviour
- When asked a question about data: use run_sql to get the answer, then explain it in plain language.
- When asked for a report/chart/dashboard/table: use run_sql to gather data, then use generate_artifact to build the visual. Tables you generate with generate_artifact are fully interactive — users can sort, filter, and group columns without asking you again.
- Only use generate_html if the component catalog genuinely cannot express what's needed.
- If a query errors, read the error, fix the SQL, and retry.
- Be concise in explanations. Lead with the answer, not the method.

{if active_skill:}
## Active Skill: {skill_name}
{skill_content}
{/if}
```

### 3. Artifacts — First-Class Output Objects

Artifacts are the primary output of Runsheet. They are not message blobs. They are first-class objects with provenance, lifecycle, and interactivity.

**Artifact record:**

```typescript
interface Artifact {
  id: string;                    // uuid
  title: string;                 // from tool call
  description?: string;          // from tool call
  kind: "structured" | "html";   // which rendering path
  spec?: ArtifactSpec;           // structured spec (for kind=structured)
  html?: string;                 // raw HTML (for kind=html)
  created_at: string;            // ISO timestamp
  conversation_id: string;       // which conversation produced this
  message_index: number;         // position in conversation
  source_queries: string[];      // SQL queries that fed into this artifact
  source_tables: string[];       // table names referenced
  revision: number;              // increments on "update this chart"
  parent_id?: string;            // previous revision's artifact id
  active_skill?: string;         // skill that was active when generated
}
```

**Structured artifacts (kind=structured):**
- Rendered natively by Svelte components — no iframe, no sandbox needed
- DataTable components use TanStack Table → sort/filter/group/pivot are instant
- Chart components use Chart.js → hover tooltips, consistent theming
- Metric components are styled KPI cards → click to drill down
- Security is inherent — the LLM outputs data, not code. No script injection possible.
- Specs are validated against the catalog schema before rendering

**HTML artifacts (kind=html, fallback):**
- Rendered in a sandboxed iframe (same security model as v2 spec)
- Chart.js injected as app asset, no CDN
- Sanitised with `ammonia` crate before render
- `sandbox="allow-scripts"`, no external network, no same-origin access
- Used only when the component catalog can't express the output

**Export:** Both artifact kinds support export:
- **Structured → HTML:** The app renders the spec to a self-contained HTML file (Svelte SSR or template-based). Tables become styled `<table>` elements, charts become Chart.js canvases, metrics become styled cards. This is the "email to stakeholders" output.
- **Structured → CSV:** For DataTable artifacts, export the underlying data as CSV
- **Structured → PNG:** Screenshot the drawer content
- **HTML → file:** Save the raw HTML directly

**What provenance enables:**
- "Show me the dashboard from before I filtered out EMEA" → look up by parent_id chain
- "What SQL produced this chart?" → source_queries
- "Regenerate this with latest data" → replay source_queries, re-run generate_artifact
- Artifact gallery: a view showing all generated artifacts, searchable, sortable by type

### 5. Conversation & Workspace Persistence

Persistence is split into three distinct layers:

**Layer 1 — Workspace (the .runsheet file)**

A workspace is a directory or a single file that captures the full state:

```
my-analysis.runsheet/
├── data.duckdb              # DuckDB database with all loaded tables
├── workspace.json           # Metadata: file paths, table mappings, settings
├── conversations/
│   ├── conv_001.json        # Full message history including tool calls
│   └── conv_002.json
└── artifacts/
    ├── art_001.html         # Generated reports (also in DB, this is for quick access)
    └── art_002.html
```

Or, simpler for v1: everything in the single `.duckdb` file using DuckDB tables:
- `_workspace_meta` — key/value config
- `_conversations` — conversation records
- `_messages` — message records with conversation_id
- `_artifacts` — artifact records with HTML content

**Layer 2 — Conversation Log**

Each conversation stores:
- All user messages
- All assistant messages (text portions)
- All tool_use calls (run_sql queries, generate_report calls)
- All tool_result responses (query results, errors)
- Artifact references

On each turn, the conversation history is sent to the LLM (with truncation for long conversations — summarise older turns, keep recent ones verbatim).

**Layer 3 — App Configuration**

Stored at `~/.runsheet/config.toml`, not in the workspace:

```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
# api_key stored in OS keychain via `keyring` crate, not in this file
ollama_url = "http://localhost:11434"
ollama_model = "qwen3:8b"

[ui]
theme = "dark"                   # dark | tactical | light
reduced_effects = false          # disables CRT scanlines, heavy glow
font_size = 13

[skills]
directories = ["~/.runsheet/skills", ".runsheet/skills"]

[export]
default_format = "html"
```

**Session resume:**
- On app launch: "Resume previous workspace?" → opens the last `.runsheet` workspace
- Or: "Open workspace" / "New workspace" / "Recent workspaces" list
- Resume restores: loaded tables, conversation history, artifact gallery, active skill

### 6. Agent Skills (agentskills.io)

Skills follow the open Agent Skills standard. For v1, keep it simple and predictable.

**Skills directory:**
```
~/.runsheet/skills/
  financial-analysis/
    SKILL.md
    templates/
      quarterly-report.html
  data-quality/
    SKILL.md
  rns-analysis/
    SKILL.md
    watchlist.json
```

**SKILL.md format:**
```yaml
---
name: financial-analysis
description: >
  Analyse financial data with focus on revenue trends, margin analysis,
  peer comparison, and regulatory context. Use when the user loads
  financial statements, earnings data, or asks about financial metrics.
---

# Financial Analysis Skill

When analysing financial data:
1. Always compute YoY and QoQ growth rates
2. Flag metrics that deviate >2σ from historical mean
3. Generate HTML reports with:
   - KPI summary cards at the top
   - Trend charts (line) for time series
   - Comparison tables with conditional formatting
4. Include a "Key Findings" narrative section
...
```

**v1 skill activation — manual with auto-suggest:**

1. **Discovery:** On startup, scan `~/.runsheet/skills/` and project-local `.runsheet/skills/`
2. **Display:** Skills listed in left sidebar with name and one-line description
3. **Manual activation:** User types `/financial-analysis` in chat input → skill content injected into system prompt for that turn
4. **Auto-suggest (lightweight):** When user types `/`, show a filterable list of available skills. When user sends a message without invoking a skill, show a subtle hint if a loaded skill's keywords match: *"Tip: /financial-analysis may help with this query"*
5. **Deactivation:** Skill stays active until user starts a new conversation or types `/clear-skill`
6. **Progressive disclosure:** Only `name` and `description` loaded at startup. Full SKILL.md body loaded on activation.

**What v1 does NOT do:**
- No LLM-based auto-triggering (adds latency, removes predictability)
- No multi-skill composition (one active skill at a time)
- No skill-authored tools (skills are prompt injections only)

These are all v2+ features once the core loop is proven.

**Skill authoring:**
- User types `/create-skill` → LLM helps draft a SKILL.md based on conversation context
- Saved to `~/.runsheet/skills/{name}/SKILL.md`
- User can edit the file directly — it's just markdown

**Built-in skills shipped with the app:**

1. **data-profiling** — Generate a comprehensive data quality report (nulls, distributions, outliers, type issues, correlations)
2. **executive-summary** — One-page HTML briefing with KPI cards and key findings narrative
3. **time-comparison** — Compare two time periods with delta highlighting and trend arrows

---

## UI Layout — The Drawer Model

The app has three panels: Data (left), Chat (centre), Artifact Drawer (right, slides in on demand).

```
STATE 1: Drawer closed (default, or no artifacts yet)
┌──────────┬───────────────────────────────────────────────────────┐
│          │                                                       │
│  DATA    │  CHAT                                                 │
│  PANEL   │                                                       │
│          │  you: What were total sales by region?                 │
│ ▸ q1_sales│                                                      │
│   12,847  │  runsheet:                                           │
│   8 cols  │  ▶ ran: SELECT region, SUM(amount) FROM q1_sales...  │
│          │                                                       │
│ ▸ returns │  EMEA leads at £4.2M, up 12% YoY. APAC grew fastest │
│   891     │  at 18%. Full breakdown:                              │
│   5 cols  │                                                       │
│          │  📊 Sales Dashboard by Region  ↗                      │
│ ─────── │                                                       │
│ SKILLS   │  you: break down EMEA by product                      │
│ ◆ finance│                                                       │
│ ◇ profile│                                                       │
│          │                                                       │
├──────────┴───────────────────────────────────────────────────────┤
│  Ask Runsheet...                                       [⏎] [/]  │
└──────────────────────────────────────────────────────────────────┘

STATE 2: Drawer open (user clicked artifact reference, or new artifact generated)
┌──────────┬──────────────────────┬────────────────────────────────┐
│          │                      │                                │
│  DATA    │  CHAT                │  ARTIFACT DRAWER               │
│  PANEL   │                      │                                │
│          │  you: show me a      │  Sales Dashboard by Region     │
│ ▸ q1_sales│  sales dashboard    │  ─────────────────────────     │
│ ▸ returns │                     │                                │
│          │  runsheet:           │  £10.6M       £47.2K           │
│          │  ▶ ran: SELECT ...   │  Total Rev    Avg Deal         │
│ ─────── │                      │  ▲ 12.3%      ▼ 3.1%          │
│ SKILLS   │  EMEA leads...      │                                │
│ ◆ finance│                      │  ┌──────────────────────┐     │
│ ◇ profile│  📊 Sales Dashboard │  │  ████ EMEA    £4.2M  │     │
│          │     ↗ (viewing)      │  │  ███ APAC    £3.6M   │     │
│          │                      │  │  ██ Americas £2.8M   │     │
│          │  you: break it down  │  └──────────────────────┘     │
│          │  by product          │                                │
│          │                      │  Region ▼  Revenue  Deals     │
│          │  📊 EMEA Products   │  ──────────────────────────    │
│          │     ↗                │  EMEA      £4.2M    89        │
│          │                      │  APAC      £3.6M    76        │
│          │                      │  Americas  £2.8M    62        │
│          │                      │  [🔍 filter] [↕ sort] [📋]   │
│          │                      │                                │
│          │                      ├────────────────────────────────┤
│          │                      │ [Export HTML] [CSV] [PNG] [⇗]  │
├──────────┴──────────────────────┴────────────────────────────────┤
│  Ask Runsheet...                                       [⏎] [/]  │
└──────────────────────────────────────────────────────────────────┘
```

**Panel behaviour:**
- **Data Panel (left):** Always visible. Loaded tables, column browser, skills list. ~200px wide.
- **Chat (centre):** Always visible. Conversation with compact artifact references. Narrows when drawer opens.
- **Artifact Drawer (right):** Slides in when artifact is generated or clicked. ~50% of remaining width. Resizable via drag handle. Close with × or Esc.

**Artifact references in chat:**
When the LLM generates an artifact, the chat shows a compact card:
```
📊 Sales Dashboard by Region  ↗
   3 components · just now
```
Clicking opens the drawer to that artifact. Multiple artifacts stack — the drawer has tabs along the top.

**Three-panel layout:**
- **Left — Data Panel:** loaded tables, column browser, skills list
- **Centre — Chat:** conversation with compact artifact references and tool-call indicators
- **Right — Artifact Drawer:** slides out to show the active artifact, with full interactivity and export toolbar

### Visual Design

**Default theme ("dark"):**
- Background: `#0a0a0a` (near-black, easier on eyes than pure black)
- Surface: `#141414` (panels, cards)
- Border: `#2a2a2a` (subtle, not glowing)
- Primary text: `#e0e0e0`
- Accent green: `#00dd77` (data, success, emphasis)
- Accent orange: `#ff8833` (highlights, user messages)
- Accent amber: `#ffbb33` (secondary emphasis, labels)
- Accent red: `#ff3333` (errors, warnings)
- Data font: `'Fira Code', 'Courier New', monospace`
- Body font: `system-ui, -apple-system, sans-serif`
- Label font: `'Fira Code'` at reduced weight

**Tactical theme (optional, unlockable):**
- Full NERV aesthetic: pure black, phosphor green glow, CRT scanlines, VT323/Orbitron fonts, circuit-trace backgrounds, pulsing alert borders
- Toggled in settings: *"Enable tactical mode"*
- Does not affect generated reports — those use the standard dark palette for portability

**Design principles:**
- Monospace for data, proportional for narrative
- Green for data emphasis, not for everything
- Glow effects only on active/hover states, not permanent
- No scanlines or CRT effects in default theme
- Dense information display, but with clear visual hierarchy
- Loading states: subtle spinner or progress bar, not dramatic pulsing text
- Errors: red accent border + clear message, not a "CONDITION RED" alert box

---

## Rust Backend — Tauri Commands

### Data Layer

```rust
/// Load a file into DuckDB. Returns table metadata.
/// Handles csv/tsv/parquet/json directly.
/// For xlsx: tries excel extension first, falls back to calamine.
#[tauri::command]
async fn ingest_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<TableMeta, AppError>

/// Execute a SQL query. Returns rows as JSON.
/// Used by the tool-use loop and by the frontend for profiling.
#[tauri::command]
async fn run_sql(
    query: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError>

/// Get schema and stats for all loaded tables.
/// Called on startup and after each ingest.
#[tauri::command]
async fn get_schema(
    state: State<'_, AppState>,
) -> Result<Vec<TableMeta>, AppError>

/// Profile a single column (distribution, nulls, top values).
#[tauri::command]
async fn profile_column(
    table: String,
    column: String,
    state: State<'_, AppState>,
) -> Result<ColumnProfile, AppError>

/// Drop a table from the database.
#[tauri::command]
async fn drop_table(
    table: String,
    state: State<'_, AppState>,
) -> Result<(), AppError>
```

### LLM Layer

```rust
/// Send a chat completion request with tool definitions.
/// Handles the full tool-use loop internally:
///   1. Send messages + tools to API
///   2. If tool_use response: execute tool, append result, re-send
///   3. Repeat until text response or max iterations (10)
///   4. Return complete response with all intermediate tool calls logged
///
/// Streams text responses back to frontend via Tauri events.
#[tauri::command]
async fn chat(
    messages: Vec<Message>,
    system_prompt: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ChatResponse, AppError>
```

**Important:** The tool-use loop runs in Rust, not in the frontend. When the LLM calls `run_sql`, the Rust backend executes it against the DuckDB connection immediately — no round-trip to the frontend. This keeps the loop fast and the security boundary clean. The frontend only sees the final result: text + artifacts.

Alternatively, for streaming UX, the backend emits Tauri events as each step completes:
```rust
app_handle.emit("chat:tool_call", &tool_call)?;    // "Running query..."
app_handle.emit("chat:tool_result", &result)?;      // Show result
app_handle.emit("chat:text_chunk", &chunk)?;         // Stream text
app_handle.emit("chat:artifact", &artifact)?;        // New report generated
app_handle.emit("chat:done", &final_response)?;      // Complete
```

### Skills Layer

```rust
/// Scan skill directories, return name + description for each.
#[tauri::command]
async fn list_skills(
    state: State<'_, AppState>,
) -> Result<Vec<SkillMeta>, AppError>

/// Load full SKILL.md content for a specific skill.
#[tauri::command]
async fn load_skill(
    skill_name: String,
    state: State<'_, AppState>,
) -> Result<SkillContent, AppError>

/// Save a new skill (from /create-skill flow).
#[tauri::command]
async fn save_skill(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), AppError>
```

### Workspace Layer

```rust
/// Create a new workspace (new .duckdb file).
#[tauri::command]
async fn new_workspace(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError>

/// Open an existing workspace.
#[tauri::command]
async fn open_workspace(
    path: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceMeta, AppError>

/// Save current conversation to workspace.
#[tauri::command]
async fn save_conversation(
    conversation: Conversation,
    state: State<'_, AppState>,
) -> Result<(), AppError>

/// Save an artifact to workspace.
#[tauri::command]
async fn save_artifact(
    artifact: Artifact,
    state: State<'_, AppState>,
) -> Result<(), AppError>

/// Export an artifact as an HTML file.
#[tauri::command]
async fn export_artifact(
    artifact_id: String,
    destination: String,
    state: State<'_, AppState>,
) -> Result<(), AppError>
```

### Export Layer

```rust
/// Export artifact as .html file (via file dialog).
#[tauri::command]
async fn export_html(artifact_id: String, state: State<'_, AppState>) -> Result<String, AppError>

/// Copy artifact HTML to clipboard.
#[tauri::command]
async fn copy_html(artifact_id: String, state: State<'_, AppState>) -> Result<(), AppError>

/// Capture artifact as PNG screenshot (via webview screenshot).
#[tauri::command]
async fn export_png(artifact_id: String, state: State<'_, AppState>) -> Result<String, AppError>

/// Open artifact in system default browser.
#[tauri::command]
async fn open_in_browser(artifact_id: String, state: State<'_, AppState>) -> Result<(), AppError>
```

---

## Data Flow: End-to-End Query

```
1. User types: "What were top 10 products by revenue last quarter?"

2. Frontend sends to Rust backend via `chat` command:
   - System prompt (with schema, active skill if any)
   - Conversation history
   - New user message

3. Rust backend sends to Claude API:
   - System: Runsheet prompt + schema + catalog docs + skill
   - Messages: history + user message
   - Tools: [run_sql, generate_artifact, generate_html]

4. Claude responds: tool_use → run_sql({ query: "SELECT ..." })

5. Rust backend executes SQL against native DuckDB
   → emits event: chat:tool_call (frontend shows "Running query...")

6. Rust backend sends tool_result back to Claude with query results
   → emits event: chat:tool_result (frontend can show intermediate results)

7. Claude responds: tool_use → generate_artifact({ title: "...", spec: {...} })

8. Rust backend:
   a. Validates the spec against the catalog schema
   b. Creates Artifact record with provenance (kind=structured)
   c. Saves to workspace
   → emits event: chat:artifact (frontend opens drawer, renders components)

9. Claude responds: text → "EMEA leads with £4.2M, up 12% YoY..."
   → emits event: chat:text_chunk (streamed)

10. Frontend renders:
    - Tool call indicators in chat (collapsible: "▶ ran: SELECT ...")
    - Text response in chat
    - Compact artifact reference in chat ("📊 Top 10 Products ↗")
    - Artifact drawer slides open with:
      - TanStack Table (sortable, filterable, paginated)
      - Chart.js chart (with hover tooltips)
      - Metric cards (with trend arrows)
    - Export toolbar at bottom of drawer

11. User clicks column header in the DataTable → instant client-side sort
    User types in filter box → instant client-side filter
    NO LLM call needed for any of this.

12. Conversation history updated with all messages + tool calls
```

---

## Key Dependencies

### Rust (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["full"] }
duckdb = { version = "1", features = ["bundled"] }   # Native DuckDB with bundled libduckdb
calamine = "0.26"                                     # xlsx fallback reader
toml = "0.8"
keyring = "3"                                         # OS keychain for API keys
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
ammonia = "4"                                         # HTML sanitisation
```

### JavaScript (package.json)
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-fs": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "@tanstack/svelte-table": "^8",
    "svelte": "^5"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4",
    "vite": "^6",
    "vitest": "^2",
    "typescript": "^5"
  }
}
```

Note: **no DuckDB-WASM dependency**. No `gray-matter` (SKILL.md parsing done in Rust). Chart.js bundled as a static asset, not an npm dependency. TanStack Table is the only significant frontend dependency beyond Svelte itself.

---

## File Structure

```
runsheet/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/                # Tauri v2 capability permissions
│   │   └── default.json
│   ├── src/
│   │   ├── main.rs                  # Tauri app setup + state init
│   │   ├── lib.rs                   # Command registration
│   │   ├── state.rs                 # AppState: DuckDB conn, config, workspace
│   │   ├── error.rs                 # AppError type
│   │   ├── commands/
│   │   │   ├── data.rs              # ingest_file, run_sql, get_schema, profile_column
│   │   │   ├── chat.rs              # chat (LLM + tool loop)
│   │   │   ├── skills.rs            # list_skills, load_skill, save_skill
│   │   │   ├── workspace.rs         # new/open/save workspace
│   │   │   └── export.rs            # export_html, copy_csv, export_png
│   │   ├── duckdb_engine.rs         # DuckDB connection management, query execution
│   │   ├── llm/
│   │   │   ├── mod.rs
│   │   │   ├── anthropic.rs         # Claude API client
│   │   │   ├── ollama.rs            # Ollama client (v2+)
│   │   │   └── tool_loop.rs         # Tool-use execution loop
│   │   ├── ingest/
│   │   │   ├── mod.rs
│   │   │   ├── csv.rs               # CSV/TSV ingestion
│   │   │   ├── parquet.rs           # Parquet ingestion
│   │   │   ├── xlsx.rs              # xlsx: extension → calamine fallback
│   │   │   └── json.rs              # NDJSON ingestion
│   │   ├── catalog.rs               # Artifact spec validation against catalog schema
│   │   ├── skills.rs                # Skill discovery, SKILL.md parsing
│   │   ├── sanitise.rs              # HTML sanitisation for HTML-fallback artifacts
│   │   ├── export_html.rs           # Render structured specs to self-contained HTML
│   │   └── workspace.rs             # Workspace persistence logic
│   └── icons/
├── src/
│   ├── App.svelte                   # Root layout: data panel + chat + drawer
│   ├── main.ts                      # Svelte mount
│   ├── lib/
│   │   ├── tauri.ts                 # Typed wrappers for Tauri commands
│   │   ├── events.ts                # Tauri event listeners (streaming)
│   │   ├── types.ts                 # Shared TypeScript types
│   │   └── catalog.ts               # Catalog schema types + validation
│   ├── components/
│   │   ├── DataPanel.svelte         # Left: tables + skills
│   │   ├── TableItem.svelte         # Single table in data panel
│   │   ├── ColumnProfile.svelte     # Column detail popover
│   │   ├── ChatPanel.svelte         # Centre: conversation
│   │   ├── ChatMessage.svelte       # Single message bubble
│   │   ├── ToolCallIndicator.svelte # Collapsible "▶ ran: SELECT..."
│   │   ├── ArtifactReference.svelte # Compact artifact card in chat
│   │   ├── ArtifactDrawer.svelte    # Right: slide-out drawer container
│   │   ├── ArtifactTabs.svelte      # Tab bar for multiple artifacts
│   │   ├── ArtifactToolbar.svelte   # Export/copy/pop-out toolbar
│   │   ├── HtmlFallback.svelte      # Sandboxed iframe for HTML artifacts
│   │   ├── DropZone.svelte          # File drag-and-drop overlay
│   │   ├── ChatInput.svelte         # Input bar with / command support
│   │   ├── SkillsList.svelte        # Skills in data panel
│   │   ├── StatusBar.svelte         # Bottom bar: workspace, connection, table count
│   │   └── Settings.svelte          # Config modal
│   ├── catalog/                     # Svelte components for the artifact catalog
│   │   ├── CatalogRenderer.svelte   # Takes a spec, dispatches to correct component
│   │   ├── MetricCard.svelte        # KPI card with value + trend
│   │   ├── DataTable.svelte         # TanStack Table wrapper (sort/filter/group/pivot)
│   │   ├── ChartView.svelte         # Chart.js wrapper (bar/line/pie/etc.)
│   │   ├── TextView.svelte          # Markdown renderer
│   │   ├── SectionView.svelte       # Collapsible section container
│   │   ├── DividerView.svelte       # Visual separator
│   │   └── Dashboard.svelte         # Layout container for multi-component dashboards
│   ├── stores/
│   │   ├── tables.svelte.ts         # Loaded table state (Svelte 5 runes)
│   │   ├── chat.svelte.ts           # Conversation history
│   │   ├── artifacts.svelte.ts      # Artifact gallery + active artifact
│   │   ├── drawer.svelte.ts         # Drawer open/closed state, active tab
│   │   ├── skills.svelte.ts         # Available skills
│   │   └── config.svelte.ts         # App configuration
│   └── styles/
│       ├── theme-dark.css           # Default dark theme
│       ├── theme-tactical.css       # NERV tactical theme (optional)
│       └── global.css               # Reset, layout, shared utilities
├── static/
│   ├── vendor/
│   │   └── chart.min.js            # Bundled Chart.js — no CDN
│   └── fonts/
│       └── FiraCode-*.woff2        # Bundled fonts — offline capable
├── .runsheet/
│   └── skills/                      # Project-local skills
├── package.json
├── vite.config.ts
├── svelte.config.js
├── tsconfig.json
└── README.md
```

---

## Development Phases

### Phase 1: The Core Loop (MVP — build this first, nothing else)

**Goal:** Drop a CSV, ask a question, see an answer in the chat. No artifacts. No skills. No persistence. No theming beyond basic dark mode.

- [ ] Tauri v2 + Svelte 5 scaffold (`cargo tauri init`)
- [ ] Rust: DuckDB connection management (`duckdb` crate with `bundled` feature)
- [ ] Rust: `ingest_file` command — CSV only for now
- [ ] Rust: `run_sql` command — execute query, return JSON rows
- [ ] Rust: `get_schema` command — DESCRIBE + SUMMARIZE
- [ ] Frontend: minimal data panel showing loaded tables + columns
- [ ] Frontend: basic chat input and message display
- [ ] Rust: `chat` command — Anthropic API with tool_use
- [ ] Rust: tool-use loop — `run_sql` tool executed in backend
- [ ] Frontend: display text responses and tool call indicators
- [ ] Frontend: file drop zone (drag CSV onto app → ingest)
- [ ] Test: ingest a real CSV, ask 5 different questions, verify correct SQL + answers

**Exit criteria:** A person can drop a CSV on the app, ask "what were total sales by region?", and see a correct text answer with the SQL that produced it. Nothing more.

### Phase 2: Artifact Drawer + Component Catalog

**Goal:** The LLM generates structured artifacts that render in a slide-out drawer with full interactivity.

- [ ] Define catalog TypeScript types (`catalog.ts`)
- [ ] Build `CatalogRenderer.svelte` — takes a spec, dispatches to component
- [ ] Build `DataTable.svelte` — TanStack Table with sort, filter, pagination
- [ ] Build `ChartView.svelte` — Chart.js wrapper with Runsheet theme
- [ ] Build `MetricCard.svelte` — KPI card with value, trend arrow, status colour
- [ ] Build `TextView.svelte` — simple markdown rendering
- [ ] Build `Dashboard.svelte` — vertical stack / grid layout for multi-component specs
- [ ] Build `ArtifactDrawer.svelte` — slide-out panel with tabs, resize handle, toolbar
- [ ] Build `ArtifactReference.svelte` — compact card in chat ("📊 Title ↗")
- [ ] Rust: `generate_artifact` tool added to tool definitions
- [ ] Rust: spec validation against catalog schema
- [ ] Rust: Artifact record creation with provenance
- [ ] Frontend: drawer opens on artifact generation, chat narrows
- [ ] Frontend: artifact toolbar (Export HTML, Copy CSV, Copy PNG, Pop Out)
- [ ] Test: ask for a dashboard, verify metrics + chart + interactive table render
- [ ] Test: sort and filter a DataTable artifact without triggering an LLM call

**Exit criteria:** Ask "show me a sales dashboard by region" and get an interactive artifact in the drawer — metrics with trend arrows, a chart, and a table you can sort and filter by clicking.

### Phase 3: HTML Fallback + Export

**Goal:** Support freeform HTML artifacts and export structured artifacts to HTML files.

- [ ] Rust: `generate_html` tool added (fallback)
- [ ] Frontend: `HtmlFallback.svelte` — sandboxed iframe for HTML artifacts
- [ ] Rust: HTML sanitisation (`ammonia` crate)
- [ ] Rust: Chart.js injection into HTML artifacts
- [ ] Rust: `export_html.rs` — render structured specs to self-contained HTML files
- [ ] Frontend: "Export HTML" button renders spec → HTML file via file dialog
- [ ] Frontend: "Copy CSV" for DataTable artifacts
- [ ] Test: verify iframe sandbox blocks external network requests
- [ ] Test: export a structured dashboard artifact as HTML, open in browser, verify it looks right

**Exit criteria:** Both artifact types render correctly. Structured artifacts export to polished HTML files suitable for emailing to stakeholders.

### Phase 4: xlsx + Parquet + Multi-Format Ingestion

**Goal:** Support all target file formats.

- [ ] Rust: xlsx ingestion via DuckDB `excel` extension
- [ ] Rust: calamine fallback for xlsx if extension unavailable
- [ ] Rust: Parquet ingestion
- [ ] Rust: NDJSON ingestion
- [ ] Frontend: file type detection and appropriate feedback
- [ ] Test: ingest 40MB xlsx (the 30-row special), verify it works
- [ ] **Critical test:** package the app (`cargo tauri build`) and verify xlsx works on Windows + macOS

**Exit criteria:** All five file types ingest correctly in both dev mode and the packaged binary.

### Phase 5: Workspace Persistence

**Goal:** Save and resume sessions.

- [ ] Rust: workspace creation (`.runsheet/` directory or single `.duckdb`)
- [ ] Rust: conversation save/load
- [ ] Rust: artifact persistence (both structured specs and HTML)
- [ ] Frontend: "New Workspace" / "Open Workspace" / "Recent" flow
- [ ] Frontend: session resume on app launch
- [ ] Rust: conversation truncation strategy for long sessions

**Exit criteria:** Close the app, reopen it, resume exactly where you left off — same data, same conversation, same artifacts in the drawer.

### Phase 6: Skills

**Goal:** Load and use Agent Skills.

- [ ] Rust: skill directory scanning
- [ ] Rust: SKILL.md parsing (YAML frontmatter + markdown body)
- [ ] Frontend: skills list in data panel
- [ ] Frontend: `/skill-name` command invocation in chat input
- [ ] Rust: skill content injection into system prompt
- [ ] Frontend: auto-suggest hint ("Tip: /financial-analysis may help")
- [ ] Rust: `/create-skill` flow — LLM drafts, user confirms, saved to disk
- [ ] Bundle 3 default skills: data-profiling, executive-summary, time-comparison

**Exit criteria:** Type `/data-profiling`, ask "profile this dataset", get a comprehensive data quality artifact in the drawer.

### Phase 7: Polish & Distribution

**Goal:** Ship it.

- [ ] Frontend: complete dark theme polish
- [ ] Frontend: tactical theme (optional NERV mode)
- [ ] Frontend: keyboard shortcuts (Ctrl+Enter to send, / for skills, Ctrl+N new conversation, Esc close drawer)
- [ ] Frontend: artifact revision chain UI in drawer
- [ ] Frontend: metric drill-down (click KPI → sends follow-up to LLM)
- [ ] Rust: OS keychain integration for API key storage
- [ ] Packaging: `cargo tauri build` for Windows (.msi), macOS (.dmg), Linux (AppImage)
- [ ] Binary size audit (target: < 30MB)
- [ ] Auto-updater configuration
- [ ] README + user guide

### Phase 8: Stretch Goals

- [ ] Ollama integration for local/private mode
- [ ] Pop-out artifacts into separate Tauri windows
- [ ] Artifact diffing: compare two revisions side-by-side in drawer
- [ ] Multiple concurrent conversations per workspace
- [ ] Streaming spec rendering (progressive artifact display as tokens arrive — json-render YAML-style)
- [ ] DataTable group-by / pivot UI
- [ ] Export to .xlsx (write back to Excel format via calamine or DuckDB)

---

## Agent Skills SKILL.md (for this project's codebase)

```yaml
---
name: runsheet
description: >
  Build and maintain the Runsheet desktop app — a Tauri v2 + Svelte 5 + TanStack Table + native DuckDB
  application for loading tabular data, querying via LLM-generated SQL (tool_use), and producing
  interactive artifacts (tables, charts, dashboards) in a slide-out drawer with provenance tracking.
  Use this skill when working on any part of the Runsheet codebase.
---

# Runsheet Development Skill

## Architecture
- **Backend:** Tauri v2 (Rust) — owns DuckDB, LLM calls, skills, persistence, spec validation
- **Frontend:** Svelte 5 (runes syntax) — pure view layer, talks to backend via Tauri commands
- **Data:** Native DuckDB via `duckdb-rs` (bundled) — NOT DuckDB-WASM
- **Tables:** TanStack Table (`@tanstack/svelte-table`) for interactive sort/filter/group/pivot
- **Charts:** Chart.js bundled as static asset — no CDN, no external scripts
- **LLM:** Anthropic Claude API with tool_use for SQL execution + artifact generation
- **Styling:** Custom dark theme, no CSS frameworks

## Key Principles
- DuckDB runs in Rust, never in the frontend
- The tool-use loop runs in Rust — frontend only sees final results + streaming events
- LLM outputs structured specs (component catalog), not raw HTML — HTML is a fallback
- Structured artifacts render natively in Svelte with full interactivity
- DataTable components use TanStack Table — sort/filter/group are client-side, zero LLM calls
- Artifacts render in a slide-out drawer, not inline in chat
- Skills are manual-invoke with auto-suggest, not auto-triggered
- Artifacts are first-class objects with provenance, not message blobs

## Component Catalog
Artifact types: dashboard, data_table, chart, document, metric_group
Components: metric, data_table, chart, text, section, divider
- `metric` → MetricCard.svelte (KPI with trend arrow)
- `data_table` → DataTable.svelte (TanStack Table, sortable/filterable)
- `chart` → ChartView.svelte (Chart.js, bar/line/pie/etc.)
- `text` → TextView.svelte (markdown)
- `section` → SectionView.svelte (collapsible container)
- CatalogRenderer.svelte dispatches spec → correct component

## Code Conventions
- Rust: clippy clean, async/await with tokio, proper error types (no unwrap in commands)
- Svelte 5: use runes ($state, $derived, $effect), not legacy reactive statements
- TypeScript: strict mode, no `any`, typed Tauri command wrappers
- CSS: custom properties in theme files, no Tailwind, no CSS-in-JS
- File naming: snake_case for Rust, PascalCase.svelte for components, camelCase.ts for lib

## Testing
- Rust: `cargo test` — unit tests for ingestion, sanitisation, skill parsing, spec validation
- Frontend: Vitest for store logic, catalog rendering, and Tauri command mocks
- E2E: manual test script for the core loop (ingest → query → artifact → interact → export)
```

---

## Summary

**Runsheet — run your sheet.**

Drop a spreadsheet, ask a question in English, get an interactive report. No Excel. No BI tool. No license. Just a desktop app.

**Stack:** Tauri v2 + Svelte 5 + TanStack Table + native DuckDB (Rust) + Claude API + Agent Skills

**Core loop:** File → DuckDB → LLM (tool_use) → SQL → structured artifact spec → interactive drawer

**Output model:** LLM generates structured specs (metrics, tables, charts, dashboards) that render as native Svelte components in a slide-out drawer. Tables are powered by TanStack Table — sort, filter, group, pivot are instant and free. HTML is a fallback for freeform content.

**What changed in v2.1:**
- Artifacts render in a **slide-out drawer** alongside the chat, not inline
- **Component catalog** — LLM outputs structured specs, not raw HTML (HTML is a fallback)
- **TanStack Table** for interactive sort/filter/group/pivot on data tables — no LLM round-trip
- Dual tool model: `generate_artifact` (preferred, structured) + `generate_html` (fallback, freeform)
- Catalog components: Metric, DataTable, Chart, Text, Section, Divider, Dashboard
- Structured artifacts export to self-contained HTML for email/SharePoint distribution
- Security is inherent for structured artifacts — LLM outputs data, not code
- Phase 2 now builds the catalog and drawer before HTML fallback (Phase 3)

**What carried forward from v2:**
- DuckDB native in Rust (not WASM)
- Chart.js bundled as app asset (no CDN)
- Tool-use loop runs in Rust backend
- Artifacts are first-class objects with provenance and revision tracking
- Skills are manual-invoke with auto-suggest
- xlsx strategy: excel extension primary, calamine fallback
- Workspace persistence in three layers (data, conversation, config)
- Professional dark theme default, NERV tactical as optional toggle
