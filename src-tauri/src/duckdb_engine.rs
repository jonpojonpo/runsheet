use crate::error::AppError;
use calamine::{open_workbook_auto, Data, Reader};
use duckdb::{params, Connection};
use std::io::Write;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnMeta {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableMeta {
    pub name: String,
    pub row_count: i64,
    pub columns: Vec<ColumnMeta>,
    pub sample: Vec<Map<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Map<String, Value>>,
    pub row_count: usize,
}

pub struct DuckDbEngine {
    conn: Connection,
}

impl DuckDbEngine {
    pub fn new() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| AppError::duckdb(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn ingest_csv(&self, path: &str, table_name: &str) -> Result<TableMeta, AppError> {
        let sql = format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_csv_auto('{}', header=true)",
            table_name,
            path.replace('\'', "\\'")
        );
        self.conn.execute(&sql, params![])
            .map_err(|e| AppError::duckdb(format!("Failed to ingest CSV: {}", e)))?;

        self.get_table_meta(table_name)
    }

    pub fn get_table_meta(&self, table_name: &str) -> Result<TableMeta, AppError> {
        // Row count
        let count_sql = format!("SELECT COUNT(*) FROM {}", table_name);
        let mut stmt = self.conn.prepare(&count_sql)?;
        let row_count: i64 = stmt.query_row(params![], |row| row.get(0))
            .map_err(|e| AppError::duckdb(e.to_string()))?;

        // Columns
        let describe_sql = format!("DESCRIBE {}", table_name);
        let mut stmt = self.conn.prepare(&describe_sql)?;
        let columns: Vec<ColumnMeta> = stmt.query_map(params![], |row| {
            Ok(ColumnMeta {
                name: row.get::<_, String>(0)?,
                col_type: row.get::<_, String>(1)?,
            })
        })
        .map_err(|e| AppError::duckdb(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        // Sample rows
        let sample_sql = format!("SELECT * FROM {} LIMIT 5", table_name);
        let result = self.execute_query(&sample_sql)?;

        Ok(TableMeta {
            name: table_name.to_string(),
            row_count,
            columns,
            sample: result.rows,
        })
    }

    pub fn execute_query(&self, sql: &str) -> Result<QueryResult, AppError> {
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::duckdb(format!("SQL error: {}", e)))?;

        // Execute first — column_name() panics if called before execution
        let mut duckrows = stmt.query(params![])
            .map_err(|e| AppError::duckdb(format!("Query error: {}", e)))?;

        // Now safe to get column names (schema is populated after execution)
        let column_count = duckrows.as_ref().map(|s| s.column_count()).unwrap_or(0);
        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                duckrows.as_ref()
                    .and_then(|s| s.column_name(i).ok())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("col{i}"))
            })
            .collect();

        // Collect rows
        let mut rows: Vec<Map<String, Value>> = Vec::new();
        loop {
            match duckrows.next() {
                Ok(Some(row)) => {
                    let mut map = Map::new();
                    for i in 0..column_count {
                        let val: Value = if let Ok(v) = row.get::<_, i64>(i) {
                            Value::Number(v.into())
                        } else if let Ok(v) = row.get::<_, f64>(i) {
                            serde_json::Number::from_f64(v)
                                .map(Value::Number)
                                .unwrap_or(Value::Null)
                        } else if let Ok(v) = row.get::<_, String>(i) {
                            Value::String(v)
                        } else if let Ok(v) = row.get::<_, bool>(i) {
                            Value::Bool(v)
                        } else {
                            Value::Null
                        };
                        map.insert(column_names[i].clone(), val);
                    }
                    rows.push(map);
                }
                Ok(None) => break,
                Err(e) => return Err(AppError::duckdb(format!("Row error: {}", e))),
            }
        }

        let row_count = rows.len();
        Ok(QueryResult { columns: column_names, rows, row_count })
    }

    pub fn list_tables(&self) -> Result<Vec<String>, AppError> {
        // SHOW TABLES is simpler and more reliable than information_schema
        let mut stmt = self.conn.prepare("SHOW TABLES")?;
        let tables: Vec<String> = stmt
            .query_map(params![], |row| row.get(0))
            .map_err(|e| AppError::duckdb(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tables)
    }

    pub fn ingest_parquet(&self, path: &str, table_name: &str) -> Result<TableMeta, AppError> {
        let sql = format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_parquet('{}')",
            table_name,
            path.replace('\'', "\\'")
        );
        self.conn.execute(&sql, params![])
            .map_err(|e| AppError::duckdb(format!("Failed to ingest Parquet: {}", e)))?;
        self.get_table_meta(table_name)
    }

    pub fn ingest_xlsx(&self, path: &str, base_name: &str) -> Result<Vec<TableMeta>, AppError> {
        fn cell_to_csv(cell: &Data) -> String {
            match cell {
                Data::String(s) => format!("\"{}\"", s.replace('"', "\"\"")),
                Data::Float(f)  => f.to_string(),
                Data::Int(i)    => i.to_string(),
                Data::Bool(b)   => b.to_string(),
                _               => String::new(),
            }
        }

        fn sanitise_sheet(name: &str) -> String {
            let s: String = name.chars()
                .map(|c| if c.is_alphanumeric() || c == '_' { c.to_ascii_lowercase() } else { '_' })
                .collect();
            if s.is_empty() { "sheet".to_string() } else { s }
        }

        let mut workbook = open_workbook_auto(path)
            .map_err(|e| AppError::io(format!("Failed to open workbook: {}", e)))?;
        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err(AppError::io("Workbook has no sheets".to_string()));
        }

        let multi = sheet_names.len() > 1;
        let mut results = Vec::new();

        for sheet_name in &sheet_names {
            let range = match workbook.worksheet_range(sheet_name) {
                Ok(r)  => r,
                Err(_) => continue,
            };

            let table_name = if multi {
                format!("{}_{}", base_name, sanitise_sheet(sheet_name))
            } else {
                base_name.to_string()
            };

            let mut rows = range.rows();

            let headers: Vec<String> = match rows.next() {
                Some(row) => row.iter().map(|cell| match cell {
                    Data::String(s) => s.clone(),
                    Data::Float(f)  => f.to_string(),
                    Data::Int(i)    => i.to_string(),
                    Data::Bool(b)   => b.to_string(),
                    _               => String::new(),
                }).collect(),
                None => continue, // empty sheet — skip
            };

            let data_rows: Vec<_> = rows.collect();
            if data_rows.is_empty() { continue; } // header-only sheet — skip

            let mut tmp = tempfile::NamedTempFile::new()
                .map_err(|e| AppError::io(format!("Failed to create temp file: {}", e)))?;

            let header_line = headers.iter()
                .map(|h| format!("\"{}\"", h.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(tmp, "{}", header_line)
                .map_err(|e| AppError::io(format!("Failed to write temp CSV: {}", e)))?;

            for row in &data_rows {
                let line = row.iter().map(cell_to_csv).collect::<Vec<_>>().join(",");
                writeln!(tmp, "{}", line)
                    .map_err(|e| AppError::io(format!("Failed to write temp CSV: {}", e)))?;
            }

            tmp.flush().map_err(|e| AppError::io(format!("Failed to flush temp CSV: {}", e)))?;
            let tmp_path = tmp.path().to_string_lossy().to_string();

            match self.ingest_csv(&tmp_path, &table_name) {
                Ok(meta) => results.push(meta),
                Err(e)   => eprintln!("Skipped sheet '{}': {}", sheet_name, e),
            }
        }

        if results.is_empty() {
            Err(AppError::io("No readable sheets with data found in workbook".to_string()))
        } else {
            Ok(results)
        }
    }

    pub fn ingest_table_from_rows(&self, table_name: &str, headers: &[String], rows: &[Vec<String>]) -> Result<TableMeta, AppError> {
        let mut tmp = tempfile::NamedTempFile::new()
            .map_err(|e| AppError::io(format!("Failed to create temp file: {}", e)))?;

        let header_line = headers.iter()
            .map(|h| format!("\"{}\"", h.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(tmp, "{}", header_line)
            .map_err(|e| AppError::io(format!("Failed to write temp CSV: {}", e)))?;

        for row in rows {
            let line = row.iter()
                .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(tmp, "{}", line)
                .map_err(|e| AppError::io(format!("Failed to write temp CSV: {}", e)))?;
        }

        tmp.flush().map_err(|e| AppError::io(format!("Failed to flush temp CSV: {}", e)))?;
        let tmp_path = tmp.path().to_string_lossy().to_string();
        self.ingest_csv(&tmp_path, table_name)
    }

    pub fn drop_table(&self, table_name: &str) -> Result<(), AppError> {
        let sql = format!("DROP TABLE IF EXISTS {}", table_name);
        self.conn.execute(&sql, params![])
            .map_err(|e| AppError::duckdb(e.to_string()))?;
        Ok(())
    }
}
