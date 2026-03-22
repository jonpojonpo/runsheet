use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppError {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    DuckDb,
    Llm,
    Io,
    Config,
    NotFound,
}

impl AppError {
    pub fn duckdb(msg: impl ToString) -> Self {
        Self { message: msg.to_string(), kind: ErrorKind::DuckDb }
    }
    pub fn llm(msg: impl ToString) -> Self {
        Self { message: msg.to_string(), kind: ErrorKind::Llm }
    }
    pub fn io(msg: impl ToString) -> Self {
        Self { message: msg.to_string(), kind: ErrorKind::Io }
    }
    pub fn config(msg: impl ToString) -> Self {
        Self { message: msg.to_string(), kind: ErrorKind::Config }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<duckdb::Error> for AppError {
    fn from(e: duckdb::Error) -> Self {
        AppError::duckdb(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::llm(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::llm(format!("JSON parse error: {}", e))
    }
}
