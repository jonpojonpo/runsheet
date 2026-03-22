export interface ColumnMeta {
  name: string;
  type: string;
}

export interface TableMeta {
  name: string;
  row_count: number;
  columns: ColumnMeta[];
  sample: Record<string, unknown>[];
}

export interface QueryResult {
  columns: string[];
  rows: Record<string, unknown>[];
  row_count: number;
}

export interface ToolCallRecord {
  tool_name: string;
  input: Record<string, unknown>;
  result: unknown;
}

export interface LlmResponse {
  text: string;
  tool_calls_made: ToolCallRecord[];
}

export type MessageRole = "user" | "assistant";

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  tool_calls?: ToolCallRecord[];
  timestamp: Date;
  artifact_ids?: string[];
}

export interface Artifact {
  id: string;
  title: string;
  description?: string;
  kind: "structured" | "html";
  spec?: import("./catalog").ArtifactSpec;
  html?: string;
  created_at: string;
  source_queries: string[];
  revision: number;
}

export interface AppError {
  message: string;
  kind: "duck_db" | "llm" | "io" | "config" | "not_found";
}

// Must match Rust ProviderKind serde(rename_all = "lowercase") — OpenAi → "openai"
export type ProviderKind = "anthropic" | "openai" | "mistral";

export interface SettingsPayload {
  provider: ProviderKind;
  anthropic_api_key: string | null;
  anthropic_model: string;
  openai_api_key: string | null;
  openai_model: string;
  mistral_api_key: string | null;
  mistral_model: string;
}
