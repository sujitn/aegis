//! Prompt extraction from LLM API request bodies.
//!
//! Extracts the user's prompt text from various LLM API formats.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domains::service_name;

/// Extracted prompt information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// The extracted prompt text.
    pub text: String,
    /// The source service (e.g., "ChatGPT", "Claude").
    pub service: String,
    /// The API endpoint path.
    pub endpoint: String,
}

impl PromptInfo {
    /// Creates a new prompt info.
    pub fn new(
        text: impl Into<String>,
        service: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            service: service.into(),
            endpoint: endpoint.into(),
        }
    }
}

/// Extracts prompt text from a request body.
///
/// Supports various LLM API formats:
/// - OpenAI Chat Completions API
/// - Anthropic Messages API
/// - Google Generative Language API
///
/// Falls back to extracting all text content from the JSON body if specific
/// extraction fails. This ensures we scan all messages even for unknown formats.
///
/// Returns `None` if no prompt could be extracted.
pub fn extract_prompt(host: &str, path: &str, body: &[u8]) -> Option<PromptInfo> {
    // Try to parse as JSON
    let json: Value = serde_json::from_slice(body).ok()?;

    let service = service_name(host);

    // Try different extraction strategies based on host and structure
    let text = if host.contains("openai.com") || host.contains("chatgpt.com") {
        extract_openai(&json)
    } else if host.contains("anthropic.com") || host.contains("claude.ai") {
        extract_anthropic(&json)
    } else if host.contains("googleapis.com") || host.contains("gemini.google.com") {
        extract_google(&json)
    } else {
        // Generic extraction - try common patterns
        extract_generic(&json)
    };

    // If specific extraction worked, use it
    if let Some(t) = text {
        return Some(PromptInfo::new(t, service, path));
    }

    // Fall back: extract ALL text content from JSON and scan it
    // This catches any format we don't specifically handle
    let all_text = extract_all_text(&json);
    if !all_text.is_empty() {
        return Some(PromptInfo::new(all_text, service, path));
    }

    None
}

/// Recursively extracts all string values from a JSON structure.
/// This is used as a fallback to scan the entire payload.
fn extract_all_text(value: &Value) -> String {
    let mut texts = Vec::new();
    collect_text_recursive(value, &mut texts);
    texts.join(" ")
}

/// Recursively collects all string values from JSON.
fn collect_text_recursive(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            // Only include strings that look like actual content (not IDs, tokens, etc.)
            if s.len() > 10 && !looks_like_id(s) {
                texts.push(s.clone());
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_text_recursive(item, texts);
            }
        }
        Value::Object(obj) => {
            for (key, val) in obj {
                // Skip keys that are likely metadata
                if !is_metadata_key(key) {
                    collect_text_recursive(val, texts);
                }
            }
        }
        _ => {}
    }
}

/// Checks if a string looks like an ID/token rather than content.
fn looks_like_id(s: &str) -> bool {
    // UUIDs, base64 tokens, etc.
    s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        || s.starts_with("eyJ") // JWT/base64
        || s.chars().filter(|c| c.is_ascii_alphanumeric()).count() == s.len() && s.len() == 32
}

/// Checks if a JSON key is likely metadata rather than content.
fn is_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "id"
            | "uuid"
            | "token"
            | "access_token"
            | "model"
            | "timestamp"
            | "created"
            | "updated"
            | "parent_message_id"
            | "conversation_id"
            | "message_id"
            | "author_id"
            | "client_id"
    )
}

/// Extracts prompt from OpenAI Chat Completions API format.
///
/// Format: `{"messages": [{"role": "user", "content": "..."}]}`
/// Also handles ChatGPT web format: `{"messages": [{"author": {"role": "user"}, "content": {"parts": [...]}}]}`
fn extract_openai(json: &Value) -> Option<String> {
    // Get messages array
    let messages = json.get("messages")?.as_array()?;

    // Find the last user message
    let user_messages: Vec<String> = messages
        .iter()
        .filter_map(|msg| {
            // Try standard OpenAI format: messages[].role
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                // Fall back to ChatGPT web format: messages[].author.role
                .or_else(|| msg.get("author")?.get("role")?.as_str())?;

            if role == "user" {
                // Try standard format: content as string
                if let Some(content) = msg.get("content") {
                    if let Some(text) = content.as_str() {
                        return Some(text.to_string());
                    }
                    // Handle OpenAI multimodal format: content as array of {type, text}
                    if let Some(parts) = content.as_array() {
                        let text_parts: Vec<&str> = parts
                            .iter()
                            .filter_map(|part| {
                                if part.get("type")?.as_str()? == "text" {
                                    part.get("text")?.as_str()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !text_parts.is_empty() {
                            return Some(text_parts.join(" "));
                        }
                    }
                    // Handle ChatGPT web format: content.parts as array of strings
                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                        let text_parts: Vec<&str> = parts
                            .iter()
                            .filter_map(|part| part.as_str())
                            .collect();
                        if !text_parts.is_empty() {
                            return Some(text_parts.join(" "));
                        }
                    }
                }
            }
            None
        })
        .collect();

    // Return all user messages concatenated
    if user_messages.is_empty() {
        None
    } else {
        Some(user_messages.join("\n"))
    }
}

/// Extracts prompt from Anthropic Messages API format.
///
/// Format: `{"messages": [{"role": "user", "content": "..."}]}`
fn extract_anthropic(json: &Value) -> Option<String> {
    // Get messages array
    let messages = json.get("messages")?.as_array()?;

    // Find all user messages
    let user_messages: Vec<String> = messages
        .iter()
        .filter_map(|msg| {
            let role = msg.get("role")?.as_str()?;
            if role == "user" {
                // Content can be string or array of content blocks
                if let Some(content) = msg.get("content") {
                    if let Some(text) = content.as_str() {
                        return Some(text.to_string());
                    }
                    // Handle array format
                    if let Some(parts) = content.as_array() {
                        let text_parts: Vec<&str> = parts
                            .iter()
                            .filter_map(|part| {
                                if part.get("type")?.as_str()? == "text" {
                                    part.get("text")?.as_str()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !text_parts.is_empty() {
                            return Some(text_parts.join(" "));
                        }
                    }
                }
            }
            None
        })
        .collect();

    if user_messages.is_empty() {
        None
    } else {
        Some(user_messages.join("\n"))
    }
}

/// Extracts prompt from Google Generative Language API format.
///
/// Format: `{"contents": [{"parts": [{"text": "..."}]}]}`
fn extract_google(json: &Value) -> Option<String> {
    // Get contents array
    let contents = json.get("contents")?.as_array()?;

    // Extract all text parts
    let texts: Vec<&str> = contents
        .iter()
        .filter_map(|content| {
            // Filter to user role if present
            if let Some(role) = content.get("role") {
                if role.as_str()? != "user" {
                    return None;
                }
            }

            let parts = content.get("parts")?.as_array()?;
            let text_parts: Vec<&str> = parts
                .iter()
                .filter_map(|part| part.get("text")?.as_str())
                .collect();

            if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join(" ").leak() as &str)
            }
        })
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

/// Generic extraction for unknown API formats.
///
/// Tries common field names like "prompt", "text", "query", "input".
fn extract_generic(json: &Value) -> Option<String> {
    // Try common field names
    for field in &["prompt", "text", "query", "input", "message", "content"] {
        if let Some(value) = json.get(*field) {
            if let Some(text) = value.as_str() {
                return Some(text.to_string());
            }
        }
    }

    // Try messages array (common pattern)
    if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
        let texts: Vec<&str> = messages
            .iter()
            .filter_map(|msg| msg.get("content")?.as_str())
            .collect();
        if !texts.is_empty() {
            return Some(texts.join("\n"));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== OpenAI Format Tests ====================

    #[test]
    fn extract_openai_simple() {
        let body = r#"{"messages": [{"role": "user", "content": "Hello, world!"}]}"#;
        let result = extract_prompt("api.openai.com", "/v1/chat/completions", body.as_bytes());

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.text, "Hello, world!");
        assert_eq!(info.service, "ChatGPT");
    }

    #[test]
    fn extract_openai_multiple_messages() {
        let body = r#"{
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "First question"},
                {"role": "assistant", "content": "First answer"},
                {"role": "user", "content": "Second question"}
            ]
        }"#;
        let result = extract_prompt("api.openai.com", "/v1/chat/completions", body.as_bytes());

        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.text.contains("First question"));
        assert!(info.text.contains("Second question"));
        assert!(!info.text.contains("You are helpful")); // System message excluded
    }

    #[test]
    fn extract_openai_multimodal() {
        let body = r#"{
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/img.jpg"}}
                ]
            }]
        }"#;
        let result = extract_prompt("api.openai.com", "/v1/chat/completions", body.as_bytes());

        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "What is in this image?");
    }

    // ==================== Anthropic Format Tests ====================

    #[test]
    fn extract_anthropic_simple() {
        let body = r#"{"messages": [{"role": "user", "content": "Hello Claude!"}]}"#;
        let result = extract_prompt("api.anthropic.com", "/v1/messages", body.as_bytes());

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.text, "Hello Claude!");
        assert_eq!(info.service, "Claude");
    }

    #[test]
    fn extract_anthropic_content_blocks() {
        let body = r#"{
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Analyze this code:"},
                    {"type": "text", "text": "function hello() {}"}
                ]
            }]
        }"#;
        let result = extract_prompt("api.anthropic.com", "/v1/messages", body.as_bytes());

        assert!(result.is_some());
        let text = result.unwrap().text;
        assert!(text.contains("Analyze this code:"));
        assert!(text.contains("function hello()"));
    }

    // ==================== Google Format Tests ====================

    #[test]
    fn extract_google_simple() {
        let body = r#"{
            "contents": [{
                "parts": [{"text": "Hello Gemini!"}]
            }]
        }"#;
        let result = extract_prompt(
            "generativelanguage.googleapis.com",
            "/v1/models/gemini-pro:generateContent",
            body.as_bytes(),
        );

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.text, "Hello Gemini!");
        assert_eq!(info.service, "Gemini");
    }

    #[test]
    fn extract_google_with_role() {
        let body = r#"{
            "contents": [
                {"role": "user", "parts": [{"text": "What is 2+2?"}]},
                {"role": "model", "parts": [{"text": "4"}]}
            ]
        }"#;
        let result = extract_prompt("gemini.google.com", "/api/generate", body.as_bytes());

        assert!(result.is_some());
        let text = result.unwrap().text;
        assert!(text.contains("What is 2+2?"));
        assert!(!text.contains("4")); // Model response excluded
    }

    // ==================== Generic Format Tests ====================

    #[test]
    fn extract_generic_prompt_field() {
        let body = r#"{"prompt": "Generate a story"}"#;
        let result = extract_prompt("unknown.com", "/api", body.as_bytes());

        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "Generate a story");
    }

    #[test]
    fn extract_generic_text_field() {
        let body = r#"{"text": "Translate this"}"#;
        let result = extract_prompt("unknown.com", "/api", body.as_bytes());

        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "Translate this");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn extract_empty_body() {
        let result = extract_prompt("api.openai.com", "/api", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn extract_invalid_json() {
        let result = extract_prompt("api.openai.com", "/api", b"not json");
        assert!(result.is_none());
    }

    #[test]
    fn extract_no_messages() {
        let body = r#"{"model": "gpt-4"}"#;
        let result = extract_prompt("api.openai.com", "/api", body.as_bytes());
        assert!(result.is_none());
    }

    // ==================== PromptInfo Tests ====================

    #[test]
    fn prompt_info_new() {
        let info = PromptInfo::new("test prompt", "ChatGPT", "/v1/chat");
        assert_eq!(info.text, "test prompt");
        assert_eq!(info.service, "ChatGPT");
        assert_eq!(info.endpoint, "/v1/chat");
    }
}
