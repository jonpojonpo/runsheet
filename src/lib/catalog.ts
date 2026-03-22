// Component catalog types — matches RUNSHEET-SPEC-v2.1.md

export interface MetricComponent {
  component: "metric";
  props: {
    label: string;
    value: string | number;
    format?: "currency" | "percent" | "number" | "compact";
    change?: number;
    change_label?: string;
    status?: "good" | "bad" | "neutral";
  };
}

export interface DataTableColumn {
  key: string;
  label: string;
  type?: "string" | "number" | "currency" | "percent" | "date";
  align?: "left" | "right" | "center";
  sortable?: boolean;
  filterable?: boolean;
  width?: string;
}

export interface HighlightRule {
  column: string;
  condition: "gt" | "lt" | "eq" | "contains";
  value: unknown;
  style: "good" | "bad" | "warning" | "emphasis";
}

export interface DataTableComponent {
  component: "data_table";
  props: {
    columns: DataTableColumn[];
    rows: Record<string, unknown>[];
    default_sort?: { key: string; direction: "asc" | "desc" };
    groupable?: boolean;
    page_size?: number;
    summary_row?: Record<string, unknown>;
    highlight_rules?: HighlightRule[];
  };
}

export interface ChartDataset {
  label: string;
  data: number[];
  color?: string;
}

export interface ChartComponent {
  component: "chart";
  props: {
    chart_type: "bar" | "line" | "pie" | "doughnut" | "scatter" | "area" | "horizontal_bar";
    title?: string;
    data: { labels: string[]; datasets: ChartDataset[] };
    x_label?: string;
    y_label?: string;
    stacked?: boolean;
    show_legend?: boolean;
    show_values?: boolean;
  };
}

export interface TextComponent {
  component: "text";
  props: {
    content: string;
    style?: "body" | "heading" | "caption" | "callout";
  };
}

export interface DividerComponent {
  component: "divider";
  props?: Record<string, never>;
}

export interface SectionComponent {
  component: "section";
  props: {
    title: string;
    collapsible?: boolean;
    components: ComponentSpec[];
  };
}

export type ComponentSpec =
  | MetricComponent
  | DataTableComponent
  | ChartComponent
  | TextComponent
  | DividerComponent
  | SectionComponent;

export interface ArtifactSpec {
  type: "dashboard" | "data_table" | "chart" | "document" | "metric_group";
  layout?: "stack" | "grid_2col" | "grid_3col";
  components: ComponentSpec[];
}
