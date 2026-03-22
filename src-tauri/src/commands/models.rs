use crate::error::AppError;

#[tauri::command]
pub async fn list_models(provider: String, api_key: String) -> Result<Vec<String>, AppError> {
    if api_key.trim().is_empty() {
        return Ok(Vec::new());
    }
    match provider.as_str() {
        "anthropic" => fetch_anthropic(&api_key).await,
        "openai"    => fetch_openai(&api_key).await,
        "mistral"   => fetch_mistral(&api_key).await,
        other       => Err(AppError::io(format!("Unknown provider: {other}"))),
    }
}

async fn fetch_anthropic(key: &str) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| AppError::io(e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AppError::io(e.to_string()))?;

    let mut ids: Vec<String> = resp["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["id"].as_str())
        .filter(|id| id.starts_with("claude-"))
        .map(String::from)
        .collect();

    // Newest first (Anthropic returns ordered, but sort by name descending as fallback)
    ids.sort_by(|a, b| b.cmp(a));
    Ok(ids)
}

async fn fetch_openai(key: &str) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| AppError::io(e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AppError::io(e.to_string()))?;

    let exclude = ["whisper", "tts", "dall-e", "embed", "babbage",
                   "davinci", "curie", "ada", "gpt-3.5", "moderat"];

    let mut ids: Vec<String> = resp["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["id"].as_str())
        .filter(|id| {
            let lc = id.to_lowercase();
            (lc.starts_with("gpt-") || lc.starts_with("o1") ||
             lc.starts_with("o3") || lc.starts_with("o4") ||
             lc.starts_with("chatgpt")) &&
            !exclude.iter().any(|e| lc.contains(e))
        })
        .map(String::from)
        .collect();

    ids.sort_by(|a, b| b.cmp(a));
    Ok(ids)
}

async fn fetch_mistral(key: &str) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.mistral.ai/v1/models")
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| AppError::io(e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AppError::io(e.to_string()))?;

    let mut ids: Vec<String> = resp["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["id"].as_str())
        .filter(|id| {
            let lc = id.to_lowercase();
            !lc.contains("embed") && !lc.contains("moderat")
        })
        .map(String::from)
        .collect();

    ids.sort_by(|a, b| b.cmp(a));
    Ok(ids)
}
