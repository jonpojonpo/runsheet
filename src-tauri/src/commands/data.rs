use crate::duckdb_engine::{QueryResult, TableMeta};
use crate::error::AppError;
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

/// Acquire the DB lock, recovering from a poisoned state rather than dying.
macro_rules! db_lock {
    ($state:expr) => {
        $state.db.lock().unwrap_or_else(|e| e.into_inner())
    };
}

fn sanitise_table_name(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let sanitised: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c.to_ascii_lowercase() } else { '_' })
        .collect();
    if sanitised.is_empty() {
        "data".to_string()
    } else if sanitised.starts_with(|c: char| c.is_numeric()) {
        format!("t_{}", sanitised)
    } else {
        sanitised
    }
}

#[derive(Clone, Serialize)]
struct PdfProgressEvent {
    file: String,
    page: u64,
    total: u64,
    tables_found: usize,
}

#[tauri::command]
pub async fn ingest_file(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<TableMeta>, AppError> {
    let p = Path::new(&path);
    let filename = p.file_name().unwrap_or_default().to_string_lossy().to_string();
    let base_name = sanitise_table_name(&filename);
    let ext = p.extension().unwrap_or_default().to_string_lossy().to_lowercase();

    match ext.as_str() {
        "csv" | "tsv" => {
            let db = db_lock!(state);
            Ok(vec![db.ingest_csv(&path, &base_name)?])
        }
        "xlsx" | "xls" => {
            let db = db_lock!(state);
            db.ingest_xlsx(&path, &base_name)
        }
        "parquet" => {
            let db = db_lock!(state);
            Ok(vec![db.ingest_parquet(&path, &base_name)?])
        }
        "pdf" => ingest_pdf(path, base_name, filename, app_handle, state).await,
        _ => Err(AppError::io(format!(
            "Unsupported file type: .{ext} (supported: CSV, XLSX, Parquet, PDF)"
        ))),
    }
}

/// Extract a short index entry from a page's markdown content.
/// Returns the first heading found, or falls back to the first non-empty line (≤120 chars).
fn page_preview(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            // Strip leading `#` chars and whitespace to get the heading text
            return trimmed.trim_start_matches('#').trim().chars().take(120).collect();
        }
    }
    // No heading — use first non-empty line, truncated
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.chars().take(120).collect();
        }
    }
    String::new()
}

/// Decode a PDF string (UTF-16 BE with BOM, or Latin-1 / PDFDocEncoding).
fn pdf_string_to_utf8(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&chars).to_string()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Resolve a PDF destination object to a 1-indexed page number.
fn dest_to_page(
    doc: &lopdf::Document,
    dest: &lopdf::Object,
    page_map: &HashMap<lopdf::ObjectId, u32>,
) -> Option<u32> {
    // Dereference if this is an indirect reference
    let resolved;
    let obj = if let Ok(id) = dest.as_reference() {
        resolved = doc.get_object(id).ok()?.clone();
        &resolved
    } else {
        dest
    };
    // Destination array: [page_ref, /XYZ|/Fit|..., ...]
    if let Ok(arr) = obj.as_array() {
        let page_ref = arr.first()?;
        let id = page_ref.as_reference().ok()?;
        return page_map.get(&id).copied();
    }
    None
}

/// Walk the PDF outline tree from `item_id`, appending (level, title, page) to `out`.
/// Uses a loop for siblings to avoid stack overflow on wide outlines.
fn collect_outline(
    doc: &lopdf::Document,
    mut item_id: lopdf::ObjectId,
    level: u32,
    page_map: &HashMap<lopdf::ObjectId, u32>,
    out: &mut Vec<(u32, String, u32)>,
) {
    loop {
        let item = match doc.get_object(item_id).ok().and_then(|o| o.as_dict().ok()) {
            Some(d) => d,
            None => return,
        };

        // Title
        let title = item
            .get(b"Title")
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(pdf_string_to_utf8)
            .unwrap_or_default();

        // Page number: prefer /Dest, fall back to /A GoTo action
        let page = item
            .get(b"Dest")
            .ok()
            .and_then(|d| dest_to_page(doc, d, page_map))
            .or_else(|| {
                item.get(b"A")
                    .ok()
                    .and_then(|o| o.as_dict().ok())
                    .and_then(|action| {
                        let s = action.get(b"S").ok()?.as_name().ok()?;
                        if s == b"GoTo" {
                            let d = action.get(b"D").ok()?;
                            dest_to_page(doc, d, page_map)
                        } else {
                            None
                        }
                    })
            })
            .unwrap_or(0);

        if !title.is_empty() {
            out.push((level, title, page));
        }

        // Recurse into children
        if let Ok(first_id) = item.get(b"First").and_then(|o| o.as_reference()) {
            collect_outline(doc, first_id, level + 1, page_map, out);
        }

        // Advance to next sibling (loop instead of recursion)
        match item.get(b"Next").and_then(|o| o.as_reference()) {
            Ok(next_id) => item_id = next_id,
            Err(_) => return,
        }
    }
}

/// Extract the PDF bookmark outline as (level, title, page) tuples.
/// Returns an empty vec if the PDF has no outline or on any error.
fn load_pdf_outline(path: &str) -> Vec<(u32, String, u32)> {
    let doc = match lopdf::Document::load(path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    // Build page-object-id → page-number lookup
    let page_map: HashMap<lopdf::ObjectId, u32> =
        doc.get_pages().into_iter().map(|(n, id)| (id, n)).collect();

    // Navigate to the first outline item
    let first_id = (|| -> Option<lopdf::ObjectId> {
        let catalog = doc.catalog().ok()?;
        let outlines_ref = catalog.get(b"Outlines").ok()?.as_reference().ok()?;
        let outlines_dict = doc.get_object(outlines_ref).ok()?.as_dict().ok()?;
        outlines_dict.get(b"First").ok()?.as_reference().ok()
    })();

    let Some(first_id) = first_id else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    collect_outline(&doc, first_id, 1, &page_map, &mut entries);
    entries
}

async fn ingest_pdf(
    path: String,
    base_name: String,
    filename: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<TableMeta>, AppError> {
    // Get page count + outline in one lopdf pass before starting kreuzberg
    let meta_path = path.clone();
    let (page_count, outline) = tokio::task::spawn_blocking(move || {
        let outline = load_pdf_outline(&meta_path);
        let count = lopdf::Document::load(&meta_path)
            .map(|d| d.get_pages().len() as u64)
            .unwrap_or(1)
            .max(1);
        (count, outline)
    })
    .await
    .unwrap_or((1, Vec::new()));

    let _ = app_handle.emit("pdf:progress", PdfProgressEvent {
        file: filename.clone(),
        page: 0,
        total: page_count,
        tables_found: 0,
    });

    let config = kreuzberg::ExtractionConfig {
        output_format: kreuzberg::OutputFormat::Markdown,
        pages: Some(kreuzberg::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = kreuzberg::extract_file(&path, None, &config)
        .await
        .map_err(|e| AppError::io(format!("PDF extraction failed: {}", e)))?;

    // Collect all tables — try top-level first, then per-page
    let mut pending: Vec<(String, Vec<String>, Vec<Vec<String>>)> = Vec::new();

    let top_tables: Vec<_> = result.tables.iter()
        .enumerate()
        .filter(|(_, t)| !t.cells.is_empty())
        .collect();

    if !top_tables.is_empty() {
        for (idx, table) in top_tables {
            let name = format!("{}_{}", base_name, idx);
            let headers = table.cells[0].clone();
            let rows = table.cells[1..].to_vec();
            pending.push((name, headers, rows));
        }
    } else if let Some(pages) = &result.pages {
        // Per-page tables (tagged-PDF structure may appear at page level)
        for page in pages {
            for (idx, table) in page.tables.iter().enumerate() {
                if table.cells.is_empty() {
                    continue;
                }
                let name = format!("{}_p{}_{}", base_name, page.page_number, idx);
                let headers = table.cells[0].clone();
                let rows = table.cells[1..].to_vec();
                pending.push((name, headers, rows));
            }
        }
    }

    // No structured tables found — fall back to a (page, preview, content) text table
    if pending.is_empty() {
        if let Some(pages) = &result.pages {
            if !pages.is_empty() {
                let headers = vec![
                    "page".to_string(),
                    "preview".to_string(),
                    "content".to_string(),
                ];
                let rows: Vec<Vec<String>> = pages.iter()
                    .map(|p| {
                        let preview = page_preview(&p.content);
                        vec![p.page_number.to_string(), preview, p.content.clone()]
                    })
                    .collect();
                pending.push((base_name.clone(), headers, rows));
            }
        }
        if pending.is_empty() && !result.content.is_empty() {
            // Last resort: whole document as single row
            let headers = vec!["content".to_string()];
            let rows = vec![vec![result.content.clone()]];
            pending.push((base_name.clone(), headers, rows));
        }
    }

    if pending.is_empty() {
        return Err(AppError::io(
            "Could not extract any content from PDF".to_string(),
        ));
    }

    // Append the outline as a separate _index table if the PDF has one
    if !outline.is_empty() {
        let headers = vec!["level".to_string(), "title".to_string(), "page".to_string()];
        let rows: Vec<Vec<String>> = outline
            .into_iter()
            .map(|(level, title, page)| vec![level.to_string(), title, page.to_string()])
            .collect();
        pending.push((format!("{}_index", base_name), headers, rows));
    }

    // All async work done — now hold the lock briefly to ingest
    let db = db_lock!(state);
    let mut results = Vec::new();
    for (name, headers, rows) in pending {
        match db.ingest_table_from_rows(&name, &headers, &rows) {
            Ok(meta) => results.push(meta),
            Err(e) => eprintln!("Failed to ingest PDF table '{}': {}", name, e),
        }
    }

    if results.is_empty() {
        Err(AppError::io("Failed to ingest any content from PDF".to_string()))
    } else {
        let _ = app_handle.emit("pdf:progress", PdfProgressEvent {
            file: filename,
            page: page_count,
            total: page_count,
            tables_found: results.len(),
        });
        Ok(results)
    }
}

#[tauri::command]
pub async fn run_sql(query: String, state: State<'_, AppState>) -> Result<QueryResult, AppError> {
    let db = db_lock!(state);
    db.execute_query(&query)
}

#[tauri::command]
pub async fn get_schema(state: State<'_, AppState>) -> Result<Vec<TableMeta>, AppError> {
    let db = db_lock!(state);
    let tables = db.list_tables()?;
    tables.iter().map(|name| db.get_table_meta(name)).collect()
}

#[tauri::command]
pub async fn drop_table(table: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = db_lock!(state);
    db.drop_table(&table)
}
