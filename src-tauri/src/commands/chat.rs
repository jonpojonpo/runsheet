use crate::duckdb_engine::TableMeta;
use crate::error::AppError;
use crate::llm::{LlmTurn, Message, ToolCallRecord, ToolDefinition, LlmResponse};
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use chrono;

#[derive(Debug, Clone, Serialize)]
struct ArtifactRecord {
    id: String,
    title: String,
    description: Option<String>,
    kind: String,
    spec: Option<Value>,
    html: Option<String>,
    created_at: String,
    source_queries: Vec<String>,
    revision: u32,
}

const MAX_TOOL_ITERATIONS: usize = 10;

fn run_sql_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "run_sql".to_string(),
        description: "Execute a DuckDB SQL query against the loaded data tables. Returns results as JSON rows. Use SELECT queries to explore data. DuckDB supports window functions, CTEs, PIVOT, UNPIVOT, SUMMARIZE, etc.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "DuckDB-compatible SQL query"
                }
            },
            "required": ["query"]
        }),
    }
}

fn generate_artifact_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "generate_artifact".to_string(),
        description: "Generate a structured artifact (table, chart, dashboard, document) using the Runsheet component catalog. The artifact renders natively in the app with full interactivity. PREFERRED over generate_html for all standard data presentations.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Human-readable title for this artifact" },
                "description": { "type": "string", "description": "Brief description of what this artifact shows" },
                "spec": {
                    "type": "object",
                    "description": "Component spec following the Runsheet catalog schema",
                    "properties": {
                        "type": { "type": "string", "enum": ["dashboard", "data_table", "chart", "document", "metric_group"] },
                        "components": { "type": "array", "items": { "type": "object" } }
                    },
                    "required": ["type", "components"]
                }
            },
            "required": ["title", "spec"]
        }),
    }
}

fn generate_html_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "generate_html".to_string(),
        description: "Render a rich HTML artifact in a sandboxed iframe. Use for D3 visualisations, custom layouts, or styled reports that go beyond the catalog components. The HTML must be fully self-contained (inline CSS and JS only, no external network fetches).".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "title":       { "type": "string", "description": "Short display title" },
                "description": { "type": "string", "description": "One-line summary shown in the drawer" },
                "html":        { "type": "string", "description": "Full self-contained HTML document" }
            },
            "required": ["title", "html"]
        }),
    }
}

fn build_system_prompt(schema: &[TableMeta]) -> String {
    let mut prompt = String::from(
        "You are Runsheet, a data analysis assistant embedded in a desktop application.\n\n"
    );

    if schema.is_empty() {
        prompt.push_str("## Available Data\nNo data loaded yet. Ask the user to drop a CSV file onto the app.\n\n");
    } else {
        prompt.push_str("## Available Data\n");
        for table in schema {
            prompt.push_str(&format!("### Table: {}\n", table.name));
            prompt.push_str(&format!("- Rows: {}\n", table.row_count));
            prompt.push_str("- Columns:\n");
            for col in &table.columns {
                prompt.push_str(&format!("  - {} ({})\n", col.name, col.col_type));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("## Tools\nYou have two tools:\n\n");
    prompt.push_str("1. `run_sql` — Execute DuckDB SQL against the loaded tables.\n\n");
    prompt.push_str("2. `generate_artifact` — Render a visual artifact. The `spec` must follow this EXACT schema:\n\n");
    prompt.push_str("```json\n");
    prompt.push_str("// Bar / line / area chart:\n");
    prompt.push_str("{\n  \"type\": \"dashboard\",\n  \"components\": [{\n    \"component\": \"chart\",\n    \"props\": {\n      \"chart_type\": \"bar\",\n      \"title\": \"Sales by Month\",\n      \"data\": {\n        \"labels\": [\"Jan\", \"Feb\", \"Mar\"],\n        \"datasets\": [{\"label\": \"Revenue\", \"data\": [100, 200, 150]}]\n      },\n      \"x_label\": \"Month\",\n      \"y_label\": \"Revenue\"\n    }\n  }]\n}\n");
    prompt.push_str("\n// Data table:\n");
    prompt.push_str("{\n  \"type\": \"dashboard\",\n  \"components\": [{\n    \"component\": \"data_table\",\n    \"props\": {\n      \"columns\": [{\"key\": \"name\", \"label\": \"Name\"}, {\"key\": \"value\", \"label\": \"Value\", \"type\": \"number\"}],\n      \"rows\": [{\"name\": \"A\", \"value\": 42}]\n    }\n  }]\n}\n");
    prompt.push_str("\n// Metric cards:\n");
    prompt.push_str("{\n  \"type\": \"metric_group\",\n  \"components\": [{\n    \"component\": \"metric\",\n    \"props\": {\"label\": \"Total Revenue\", \"value\": 12500, \"format\": \"currency\"}\n  }]\n}\n");
    prompt.push_str("```\n\n");
    prompt.push_str("Rules: ALWAYS use `\"component\"` (not `\"type\"`) for each item. ALWAYS nest props under `\"props\"`. chart_type values: bar, line, area, pie, doughnut, horizontal_bar.\n\n");
    prompt.push_str("3. `generate_html` — Render a fully self-contained HTML document in a sandboxed iframe. Use for D3 visualisations, network graphs, or styled reports that need custom JS beyond what `generate_artifact` supports. No external URLs allowed.\n\n");
    prompt.push_str("## Behaviour\n");
    prompt.push_str("- When asked a question: use run_sql, answer in plain language.\n");
    prompt.push_str("- When asked for a chart/table/dashboard: run_sql first, then generate_artifact.\n");
    prompt.push_str("- When asked for a D3 visualisation or complex custom layout: run_sql first, then generate_html.\n");
    prompt.push_str("- Fix SQL errors by reading the message and retrying.\n");
    prompt.push_str("- Be concise. Lead with the answer.\n");

    prompt
}

#[derive(Clone, Serialize)]
struct ToolCallEvent {
    tool_name: String,
    input: Value,
}

#[derive(Clone, Serialize)]
struct ToolResultEvent {
    tool_name: String,
    row_count: usize,
    preview: Value,
}

#[derive(Clone, Serialize)]
struct TextChunkEvent {
    text: String,
}

#[tauri::command]
pub async fn chat(
    messages: Vec<Message>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<LlmResponse, AppError> {
    // Build the provider
    let provider = {
        let config = state.llm_config.lock().unwrap_or_else(|e| e.into_inner());
        config.build_provider()?
    };

    // Get current schema for system prompt
    let schema: Vec<TableMeta> = {
        let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let tables = db.list_tables()?;
        tables.iter().map(|name| db.get_table_meta(name)).collect::<Result<Vec<_>, _>>()?
    };

    let system = build_system_prompt(&schema);
    let tools = vec![run_sql_tool_definition(), generate_artifact_tool_definition(), generate_html_tool_definition()];

    // Build initial conversation as Vec<Value> using provider-agnostic format.
    // Text messages: {"role": "user"|"assistant", "content": "string"}
    let mut conversation: Vec<Value> = messages
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();

    let mut tool_call_records: Vec<ToolCallRecord> = Vec::new();
    let mut final_text = String::new();

    let on_token: crate::llm::TokenCallback = {
        let app = app_handle.clone();
        Some(Arc::new(move |token: String| {
            let _ = app.emit("chat:token", json!({ "text": token }));
        }))
    };

    for _iteration in 0..MAX_TOOL_ITERATIONS {
        let turn = provider.chat(&system, conversation.clone(), tools.clone(), on_token.clone()).await?;

        match turn {
            LlmTurn::Text(text) => {
                final_text = text.clone();
                let _ = app_handle.emit("chat:text", TextChunkEvent { text });
                break;
            }
            LlmTurn::ToolUse(tool_calls) | LlmTurn::TextAndToolUse { tool_calls, text: _ } => {
                // Append assistant tool_use message in provider-agnostic format:
                // {"role": "assistant", "tool_use": [{"id":..,"name":..,"input":..}]}
                let tool_use_entries: Vec<Value> = tool_calls.iter().map(|tc| json!({
                    "id": tc.id,
                    "name": tc.name,
                    "input": tc.input,
                })).collect();

                conversation.push(json!({
                    "role": "assistant",
                    "tool_use": tool_use_entries,
                }));

                // Execute each tool call, collect results
                let mut tool_results: Vec<Value> = Vec::new();

                for tool_call in &tool_calls {
                    match tool_call.name.as_str() {
                        "run_sql" => {
                            let query = tool_call.input["query"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();

                            let _ = app_handle.emit("chat:tool_call", ToolCallEvent {
                                tool_name: "run_sql".to_string(),
                                input: tool_call.input.clone(),
                            });

                            let result = {
                                let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                                db.execute_query(&query)
                            };

                            let (result_entry, record_result) = match result {
                                Ok(qr) => {
                                    let preview: Value = Value::Array(
                                        qr.rows.iter().take(3).map(|r| Value::Object(r.clone())).collect()
                                    );
                                    let _ = app_handle.emit("chat:tool_result", ToolResultEvent {
                                        tool_name: "run_sql".to_string(),
                                        row_count: qr.row_count,
                                        preview: preview.clone(),
                                    });
                                    let full_json = serde_json::to_value(&qr.rows).unwrap_or(Value::Null);
                                    let content_str = serde_json::to_string(&full_json).unwrap_or_default();
                                    (
                                        json!({
                                            "tool_use_id": tool_call.id,
                                            "content": content_str,
                                            "is_error": false,
                                        }),
                                        full_json,
                                    )
                                }
                                Err(e) => {
                                    let err_msg = e.message.clone();
                                    (
                                        json!({
                                            "tool_use_id": tool_call.id,
                                            "content": format!("ERROR: {}", err_msg),
                                            "is_error": true,
                                        }),
                                        Value::String(format!("ERROR: {}", err_msg)),
                                    )
                                }
                            };

                            tool_call_records.push(ToolCallRecord {
                                tool_name: "run_sql".to_string(),
                                input: tool_call.input.clone(),
                                result: record_result,
                            });

                            tool_results.push(result_entry);
                        }
                        "generate_artifact" => {
                            let title = tool_call.input["title"].as_str().unwrap_or("Artifact").to_string();
                            let description = tool_call.input["description"].as_str().map(|s| s.to_string());
                            let spec = tool_call.input.get("spec").cloned();

                            let artifact_id = uuid::Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();

                            let artifact = ArtifactRecord {
                                id: artifact_id.clone(),
                                title,
                                description,
                                kind: "structured".to_string(),
                                spec,
                                html: None,
                                created_at: now,
                                source_queries: tool_call_records.iter()
                                    .filter(|r| r.tool_name == "run_sql")
                                    .filter_map(|r| r.input.get("query").and_then(|q| q.as_str()).map(|s| s.to_string()))
                                    .collect(),
                                revision: 1,
                            };

                            let _ = app_handle.emit("chat:artifact", &artifact);

                            tool_results.push(json!({
                                "type": "tool_result",
                                "tool_use_id": tool_call.id,
                                "content": json!({ "artifact_id": artifact_id, "status": "rendered" }).to_string()
                            }));

                            tool_call_records.push(ToolCallRecord {
                                tool_name: "generate_artifact".to_string(),
                                input: tool_call.input.clone(),
                                result: json!({ "artifact_id": artifact_id }),
                            });
                        }
                        "generate_html" => {
                            let title = tool_call.input["title"].as_str().unwrap_or("HTML").to_string();
                            let description = tool_call.input["description"].as_str().map(|s| s.to_string());
                            let html = tool_call.input["html"].as_str().unwrap_or("").to_string();

                            let artifact_id = uuid::Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();

                            let artifact = ArtifactRecord {
                                id: artifact_id.clone(),
                                title,
                                description,
                                kind: "html".to_string(),
                                spec: None,
                                html: Some(html),
                                created_at: now,
                                source_queries: tool_call_records.iter()
                                    .filter(|r| r.tool_name == "run_sql")
                                    .filter_map(|r| r.input.get("query").and_then(|q| q.as_str()).map(|s| s.to_string()))
                                    .collect(),
                                revision: 1,
                            };

                            let _ = app_handle.emit("chat:artifact", &artifact);

                            tool_results.push(json!({
                                "tool_use_id": tool_call.id,
                                "content": json!({ "artifact_id": artifact_id, "status": "rendered" }).to_string()
                            }));

                            tool_call_records.push(ToolCallRecord {
                                tool_name: "generate_html".to_string(),
                                input: tool_call.input.clone(),
                                result: json!({ "artifact_id": artifact_id }),
                            });
                        }
                        unknown => {
                            tool_results.push(json!({
                                "tool_use_id": tool_call.id,
                                "content": format!("Unknown tool: {}", unknown),
                                "is_error": true,
                            }));
                        }
                    }
                }

                // Append user tool_results message in provider-agnostic format:
                // {"role": "user", "tool_results": [...]}
                conversation.push(json!({
                    "role": "user",
                    "tool_results": tool_results,
                }));
            }
        }
    }

    let _ = app_handle.emit("chat:done", json!({}));

    Ok(LlmResponse {
        text: final_text,
        tool_calls_made: tool_call_records,
    })
}
