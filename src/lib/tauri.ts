import { invoke } from "@tauri-apps/api/core";
import type { TableMeta, QueryResult, LlmResponse, SettingsPayload } from "./types";

export interface Message {
  role: string;
  content: string;
}

export async function ingestFile(path: string): Promise<TableMeta[]> {
  return invoke("ingest_file", { path });
}

export async function runSql(query: string): Promise<QueryResult> {
  return invoke("run_sql", { query });
}

export async function getSchema(): Promise<TableMeta[]> {
  return invoke("get_schema");
}

export async function dropTable(table: string): Promise<void> {
  return invoke("drop_table", { table });
}

export async function chat(messages: Message[]): Promise<LlmResponse> {
  return invoke("chat", { messages });
}

export async function getSettings(): Promise<SettingsPayload> {
  return invoke("get_settings");
}

export async function saveSettings(settings: SettingsPayload): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function listModels(provider: string, apiKey: string): Promise<string[]> {
  return invoke("list_models", { provider, apiKey });
}
