//! Smart Content Parsing (F026).
//!
//! Robust prompt extraction from diverse LLM payload formats with an extensible
//! parser registry. Supports JSON, form data, multipart, NDJSON, SSE, and raw text.
//!
//! ## Features
//!
//! - Trait-based parser interface with priority registration
//! - Chat history vs current prompt differentiation
//! - Streaming request support (SSE, chunked)
//! - Fallback strategies for unknown formats
//! - Zero-copy parsing where possible
//! - Confidence scoring for extraction quality

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domains::service_name;

// =============================================================================
// Core Types
// =============================================================================

/// Maximum payload size to parse (1MB default).
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Minimum string length to extract in fallback mode.
pub const MIN_EXTRACTED_STRING_LENGTH: usize = 10;

/// Context for parsing a request payload.
#[derive(Debug, Clone)]
pub struct ParseContext {
    /// The request host/domain.
    pub host: String,
    /// The request path.
    pub path: String,
    /// The Content-Type header value.
    pub content_type: Option<String>,
    /// The Content-Length header value.
    pub content_length: Option<usize>,
    /// The HTTP method.
    pub method: String,
    /// Whether to scan full history or current prompt only.
    pub scan_full_history: bool,
    /// Maximum payload size to process.
    pub max_payload_size: usize,
}

impl ParseContext {
    /// Creates a new parse context.
    pub fn new(host: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            path: path.into(),
            content_type: None,
            content_length: None,
            method: "POST".to_string(),
            scan_full_history: true,
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
        }
    }

    /// Sets the content type.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Sets the content length.
    pub fn with_content_length(mut self, len: usize) -> Self {
        self.content_length = Some(len);
        self
    }

    /// Sets the HTTP method.
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into();
        self
    }

    /// Sets whether to scan full history.
    pub fn with_scan_full_history(mut self, scan: bool) -> Self {
        self.scan_full_history = scan;
        self
    }

    /// Sets the maximum payload size.
    pub fn with_max_payload_size(mut self, size: usize) -> Self {
        self.max_payload_size = size;
        self
    }

    /// Returns the MIME type without parameters (e.g., "application/json" from "application/json; charset=utf-8").
    pub fn mime_type(&self) -> Option<&str> {
        self.content_type
            .as_ref()
            .map(|ct| ct.split(';').next().unwrap_or(ct).trim())
    }

    /// Returns the charset from content-type, if present.
    pub fn charset(&self) -> Option<&str> {
        self.content_type.as_ref().and_then(|ct| {
            ct.split(';')
                .find(|part| part.trim().to_lowercase().starts_with("charset="))
                .map(|part| part.trim().strip_prefix("charset=").unwrap_or(part).trim())
        })
    }
}

/// An extracted prompt with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPrompt {
    /// The prompt text.
    pub text: String,
    /// Whether this is the current/latest message (vs history).
    pub is_current: bool,
    /// The role of the message sender (user, assistant, system).
    pub role: Option<String>,
    /// Position in conversation (0 = oldest).
    pub position: usize,
}

impl ExtractedPrompt {
    /// Creates a new extracted prompt.
    pub fn new(text: impl Into<String>, is_current: bool) -> Self {
        Self {
            text: text.into(),
            is_current,
            role: None,
            position: 0,
        }
    }

    /// Sets the role.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Sets the position.
    pub fn with_position(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    /// Checks if this is a user message.
    pub fn is_user_message(&self) -> bool {
        self.role.as_deref() == Some("user")
    }
}

/// Warnings generated during parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParseWarning {
    /// Payload was truncated due to size limit.
    Truncated { original_size: usize, limit: usize },
    /// Malformed content was encountered but parsing continued.
    MalformedContent { details: String },
    /// BOM was detected and stripped.
    BomStripped,
    /// Charset conversion was performed.
    CharsetConverted { from: String, to: String },
    /// JSON5 quirks were handled (trailing commas, comments).
    Json5Quirks,
    /// Partial extraction - some content couldn't be parsed.
    PartialExtraction { reason: String },
    /// Binary content was detected and skipped.
    BinarySkipped,
    /// Unknown format, using fallback extraction.
    FallbackUsed,
}

/// Result of parsing a payload.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Extracted prompts.
    pub prompts: Vec<ExtractedPrompt>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Name of the parser that produced this result.
    pub parser_name: String,
    /// Warnings generated during parsing.
    pub warnings: Vec<ParseWarning>,
    /// The detected service name.
    pub service: String,
}

impl ParseResult {
    /// Creates a new empty parse result.
    pub fn empty(parser_name: impl Into<String>, service: impl Into<String>) -> Self {
        Self {
            prompts: Vec::new(),
            confidence: 0.0,
            parser_name: parser_name.into(),
            warnings: Vec::new(),
            service: service.into(),
        }
    }

    /// Creates a parse result with prompts.
    pub fn with_prompts(
        prompts: Vec<ExtractedPrompt>,
        confidence: f32,
        parser_name: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        Self {
            prompts,
            confidence,
            parser_name: parser_name.into(),
            warnings: Vec::new(),
            service: service.into(),
        }
    }

    /// Adds a warning.
    pub fn add_warning(&mut self, warning: ParseWarning) {
        self.warnings.push(warning);
    }

    /// Checks if any prompts were extracted.
    pub fn has_prompts(&self) -> bool {
        !self.prompts.is_empty()
    }

    /// Gets the current prompt (highest weight).
    pub fn current_prompt(&self) -> Option<&ExtractedPrompt> {
        self.prompts.iter().find(|p| p.is_current)
    }

    /// Gets all user prompts.
    pub fn user_prompts(&self) -> Vec<&ExtractedPrompt> {
        self.prompts
            .iter()
            .filter(|p| p.is_user_message())
            .collect()
    }

    /// Combines all prompt text into a single string.
    pub fn combined_text(&self) -> String {
        self.prompts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Gets weighted text for classification (current prompts weighted higher).
    pub fn weighted_text(&self) -> String {
        // Current prompts are repeated to give them more weight
        let mut parts = Vec::new();
        for prompt in &self.prompts {
            if prompt.is_current {
                // Current prompt gets more weight (repeated 3x)
                parts.push(prompt.text.clone());
                parts.push(prompt.text.clone());
                parts.push(prompt.text.clone());
            } else {
                parts.push(prompt.text.clone());
            }
        }
        parts.join("\n")
    }
}

// =============================================================================
// Parser Trait
// =============================================================================

/// Trait for payload parsers.
pub trait PayloadParser: Send + Sync {
    /// Returns the parser name.
    fn name(&self) -> &str;

    /// Checks if this parser can handle the given content type and host.
    fn can_parse(&self, content_type: &str, host: &str) -> bool;

    /// Parses the payload and extracts prompts.
    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult;

    /// Returns the priority (higher = checked first).
    fn priority(&self) -> i32 {
        0
    }
}

// =============================================================================
// Built-in Parsers
// =============================================================================

/// JSON parser with enhanced error handling.
#[derive(Debug, Clone, Default)]
pub struct JsonParser;

impl JsonParser {
    /// Strips BOM from UTF-8 content.
    fn strip_bom(body: &[u8]) -> (&[u8], bool) {
        if body.starts_with(&[0xEF, 0xBB, 0xBF]) {
            (&body[3..], true)
        } else {
            (body, false)
        }
    }

    /// Attempts to fix JSON5 quirks (trailing commas, comments).
    fn fix_json5_quirks(s: &str) -> (String, bool) {
        let mut fixed = String::with_capacity(s.len());
        let mut had_quirks = false;
        let mut in_string = false;
        let mut escape_next = false;
        let chars: Vec<char> = s.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];

            if escape_next {
                fixed.push(c);
                escape_next = false;
                i += 1;
                continue;
            }

            if c == '\\' && in_string {
                escape_next = true;
                fixed.push(c);
                i += 1;
                continue;
            }

            if c == '"' {
                in_string = !in_string;
                fixed.push(c);
                i += 1;
                continue;
            }

            if in_string {
                fixed.push(c);
                i += 1;
                continue;
            }

            // Handle single-line comments
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                had_quirks = true;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            // Handle multi-line comments
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                had_quirks = true;
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            // Handle trailing commas before ] or }
            if c == ',' {
                // Look ahead for closing bracket
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == ']' || chars[j] == '}') {
                    had_quirks = true;
                    i += 1;
                    continue; // Skip the trailing comma
                }
            }

            fixed.push(c);
            i += 1;
        }

        (fixed, had_quirks)
    }

    /// Extracts prompts from OpenAI format.
    fn extract_openai(&self, json: &Value, context: &ParseContext) -> Vec<ExtractedPrompt> {
        let messages = match json.get("messages").and_then(|m| m.as_array()) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let total = messages.len();
        messages
            .iter()
            .enumerate()
            .filter_map(|(idx, msg)| {
                // Get role - try standard format and ChatGPT web format
                let role = msg
                    .get("role")
                    .and_then(|r| r.as_str())
                    .or_else(|| msg.get("author")?.get("role")?.as_str())?;

                // Only extract user messages
                if role != "user" {
                    return None;
                }

                // Extract content
                let text = self.extract_content(msg.get("content")?)?;
                let is_current = if context.scan_full_history {
                    idx == total - 1 || self.is_last_user_message(messages, idx)
                } else {
                    self.is_last_user_message(messages, idx)
                };

                Some(
                    ExtractedPrompt::new(text, is_current)
                        .with_role("user")
                        .with_position(idx),
                )
            })
            .collect()
    }

    /// Checks if this is the last user message in the array.
    fn is_last_user_message(&self, messages: &[Value], current_idx: usize) -> bool {
        for msg in messages.iter().skip(current_idx + 1) {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .or_else(|| msg.get("author")?.get("role")?.as_str());

            if role == Some("user") {
                return false;
            }
        }
        true
    }

    /// Extracts text from content (string or array of content blocks).
    fn extract_content(&self, content: &Value) -> Option<String> {
        // Simple string content
        if let Some(text) = content.as_str() {
            return Some(text.to_string());
        }

        // Array of content blocks (multimodal)
        if let Some(parts) = content.as_array() {
            let texts: Vec<&str> = parts
                .iter()
                .filter_map(|part| {
                    if part.get("type")?.as_str()? == "text" {
                        part.get("text")?.as_str()
                    } else {
                        None
                    }
                })
                .collect();

            if !texts.is_empty() {
                return Some(texts.join(" "));
            }
        }

        // ChatGPT web format: content.parts
        if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
            let texts: Vec<&str> = parts.iter().filter_map(|p| p.as_str()).collect();
            if !texts.is_empty() {
                return Some(texts.join(" "));
            }
        }

        None
    }

    /// Extracts prompts from Anthropic format.
    fn extract_anthropic(&self, json: &Value, context: &ParseContext) -> Vec<ExtractedPrompt> {
        let messages = match json.get("messages").and_then(|m| m.as_array()) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let total = messages.len();
        messages
            .iter()
            .enumerate()
            .filter_map(|(idx, msg)| {
                let role = msg.get("role")?.as_str()?;
                if role != "user" {
                    return None;
                }

                let text = self.extract_content(msg.get("content")?)?;
                let is_current = if context.scan_full_history {
                    idx == total - 1 || self.is_last_user_message(messages, idx)
                } else {
                    self.is_last_user_message(messages, idx)
                };

                Some(
                    ExtractedPrompt::new(text, is_current)
                        .with_role("user")
                        .with_position(idx),
                )
            })
            .collect()
    }

    /// Extracts prompts from Google format.
    fn extract_google(&self, json: &Value, context: &ParseContext) -> Vec<ExtractedPrompt> {
        let contents = match json.get("contents").and_then(|c| c.as_array()) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let total = contents.len();
        contents
            .iter()
            .enumerate()
            .filter_map(|(idx, content)| {
                // Filter to user role if present
                if let Some(role) = content.get("role").and_then(|r| r.as_str()) {
                    if role != "user" {
                        return None;
                    }
                }

                let parts = content.get("parts")?.as_array()?;
                let texts: Vec<&str> = parts
                    .iter()
                    .filter_map(|part| part.get("text")?.as_str())
                    .collect();

                if texts.is_empty() {
                    return None;
                }

                let is_current = !context.scan_full_history || idx == total - 1;

                Some(
                    ExtractedPrompt::new(texts.join(" "), is_current)
                        .with_role("user")
                        .with_position(idx),
                )
            })
            .collect()
    }

    /// Extracts prompts from generic JSON format.
    fn extract_generic(&self, json: &Value, _context: &ParseContext) -> Vec<ExtractedPrompt> {
        // Try common field names
        for field in &["prompt", "text", "query", "input", "message", "content"] {
            if let Some(value) = json.get(*field) {
                if let Some(text) = value.as_str() {
                    return vec![ExtractedPrompt::new(text, true)];
                }
            }
        }

        // Try messages array
        if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
            let total = messages.len();
            let prompts: Vec<ExtractedPrompt> = messages
                .iter()
                .enumerate()
                .filter_map(|(idx, msg)| {
                    let text = msg.get("content")?.as_str()?;
                    Some(ExtractedPrompt::new(text, idx == total - 1).with_position(idx))
                })
                .collect();
            if !prompts.is_empty() {
                return prompts;
            }
        }

        Vec::new()
    }
}

impl PayloadParser for JsonParser {
    fn name(&self) -> &str {
        "json"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        let ct = content_type.to_lowercase();
        ct.contains("application/json") || ct.contains("text/json")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        // Check size limit
        let body = if body.len() > context.max_payload_size {
            result.add_warning(ParseWarning::Truncated {
                original_size: body.len(),
                limit: context.max_payload_size,
            });
            &body[..context.max_payload_size]
        } else {
            body
        };

        // Strip BOM
        let (body, had_bom) = Self::strip_bom(body);
        if had_bom {
            result.add_warning(ParseWarning::BomStripped);
        }

        // Convert to string
        let text = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        // Try parsing as standard JSON first
        let json: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => {
                // Try fixing JSON5 quirks
                let (fixed, had_quirks) = Self::fix_json5_quirks(text);
                if had_quirks {
                    result.add_warning(ParseWarning::Json5Quirks);
                }
                match serde_json::from_str(&fixed) {
                    Ok(v) => v,
                    Err(e) => {
                        result.add_warning(ParseWarning::MalformedContent {
                            details: format!("JSON parse error: {}", e),
                        });
                        return result;
                    }
                }
            }
        };

        // Extract based on host
        let host = &context.host;
        let prompts = if host.contains("openai.com") || host.contains("chatgpt.com") {
            self.extract_openai(&json, context)
        } else if host.contains("anthropic.com") || host.contains("claude.ai") {
            self.extract_anthropic(&json, context)
        } else if host.contains("googleapis.com") || host.contains("gemini.google.com") {
            self.extract_google(&json, context)
        } else {
            self.extract_generic(&json, context)
        };

        if !prompts.is_empty() {
            result.prompts = prompts;
            result.confidence = 0.95;
        } else {
            // Fallback: extract all text
            let all_text = extract_all_text_from_json(&json);
            if !all_text.is_empty() {
                result.prompts = vec![ExtractedPrompt::new(all_text, true)];
                result.confidence = 0.5;
                result.add_warning(ParseWarning::FallbackUsed);
            }
        }

        result
    }

    fn priority(&self) -> i32 {
        100
    }
}

/// Form data parser (application/x-www-form-urlencoded).
#[derive(Debug, Clone, Default)]
pub struct FormParser;

impl PayloadParser for FormParser {
    fn name(&self) -> &str {
        "form"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        content_type
            .to_lowercase()
            .contains("application/x-www-form-urlencoded")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        // Check size limit
        let body = if body.len() > context.max_payload_size {
            result.add_warning(ParseWarning::Truncated {
                original_size: body.len(),
                limit: context.max_payload_size,
            });
            &body[..context.max_payload_size]
        } else {
            body
        };

        let text = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        // Parse form data
        let mut prompts = Vec::new();
        let prompt_fields = [
            "prompt", "text", "query", "input", "message", "content", "q",
        ];

        for pair in text.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let key = urlencoding_decode(key);
                let value = urlencoding_decode(value);

                if prompt_fields.contains(&key.as_str())
                    && value.len() >= MIN_EXTRACTED_STRING_LENGTH
                {
                    prompts.push(ExtractedPrompt::new(value, true));
                }
            }
        }

        if !prompts.is_empty() {
            result.prompts = prompts;
            result.confidence = 0.8;
        }

        result
    }

    fn priority(&self) -> i32 {
        80
    }
}

/// Simple URL decoding.
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Multipart form data parser.
#[derive(Debug, Clone, Default)]
pub struct MultipartParser;

impl PayloadParser for MultipartParser {
    fn name(&self) -> &str {
        "multipart"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        content_type.to_lowercase().contains("multipart/form-data")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        // Extract boundary from content-type
        let boundary = context.content_type.as_ref().and_then(|ct| {
            ct.split(';')
                .find(|part| part.trim().to_lowercase().starts_with("boundary="))
                .map(|part| {
                    part.trim()
                        .strip_prefix("boundary=")
                        .or_else(|| part.trim().strip_prefix("BOUNDARY="))
                        .unwrap_or("")
                        .trim_matches('"')
                        .to_string()
                })
        });

        let boundary = match boundary {
            Some(b) if !b.is_empty() => b,
            _ => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Missing boundary in multipart".to_string(),
                });
                return result;
            }
        };

        let body_str = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8 in multipart".to_string(),
                });
                return result;
            }
        };

        // Parse multipart parts
        let delimiter = format!("--{}", boundary);
        let parts: Vec<&str> = body_str.split(&delimiter).collect();

        let prompt_fields = ["prompt", "text", "query", "input", "message", "content"];
        let mut prompts = Vec::new();

        for part in parts.iter().skip(1) {
            // Skip first empty part
            if part.starts_with("--") {
                break; // End marker
            }

            // Split headers from body
            if let Some((headers, body)) = part.split_once("\r\n\r\n") {
                // Check if this is a text field we care about
                let name = headers
                    .lines()
                    .find(|line| line.to_lowercase().starts_with("content-disposition"))
                    .and_then(|line| {
                        line.split(';')
                            .find(|part| part.trim().to_lowercase().starts_with("name="))
                            .map(|part| {
                                part.trim()
                                    .strip_prefix("name=")
                                    .or_else(|| part.trim().strip_prefix("NAME="))
                                    .unwrap_or("")
                                    .trim_matches('"')
                                    .to_string()
                            })
                    });

                if let Some(name) = name {
                    if prompt_fields.contains(&name.as_str()) {
                        let text = body.trim_end_matches("\r\n").trim();
                        if text.len() >= MIN_EXTRACTED_STRING_LENGTH {
                            prompts.push(ExtractedPrompt::new(text, true));
                        }
                    }
                }
            }
        }

        if !prompts.is_empty() {
            result.prompts = prompts;
            result.confidence = 0.8;
        }

        result
    }

    fn priority(&self) -> i32 {
        70
    }
}

/// Plain text parser.
#[derive(Debug, Clone, Default)]
pub struct TextParser;

impl PayloadParser for TextParser {
    fn name(&self) -> &str {
        "text"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        let ct = content_type.to_lowercase();
        ct.contains("text/plain") || ct.contains("text/html")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        // Check size limit
        let body = if body.len() > context.max_payload_size {
            result.add_warning(ParseWarning::Truncated {
                original_size: body.len(),
                limit: context.max_payload_size,
            });
            &body[..context.max_payload_size]
        } else {
            body
        };

        // Strip BOM
        let (body, had_bom) = JsonParser::strip_bom(body);
        if had_bom {
            result.add_warning(ParseWarning::BomStripped);
        }

        let text = match std::str::from_utf8(body) {
            Ok(t) => t.trim(),
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        if text.len() >= MIN_EXTRACTED_STRING_LENGTH {
            result.prompts = vec![ExtractedPrompt::new(text, true)];
            result.confidence = 0.6;
        }

        result
    }

    fn priority(&self) -> i32 {
        20
    }
}

/// NDJSON (Newline-Delimited JSON) parser.
#[derive(Debug, Clone, Default)]
pub struct NdjsonParser;

impl PayloadParser for NdjsonParser {
    fn name(&self) -> &str {
        "ndjson"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        let ct = content_type.to_lowercase();
        ct.contains("application/x-ndjson")
            || ct.contains("application/jsonl")
            || ct.contains("application/json-lines")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        let text = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        let json_parser = JsonParser;
        let mut all_prompts = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let total = lines.len();

        for (idx, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                let mut line_prompts = json_parser.extract_generic(&json, context);
                // Mark last line as current
                for prompt in &mut line_prompts {
                    prompt.is_current = idx == total - 1;
                    prompt.position = idx;
                }
                all_prompts.extend(line_prompts);
            }
        }

        if !all_prompts.is_empty() {
            result.prompts = all_prompts;
            result.confidence = 0.85;
        }

        result
    }

    fn priority(&self) -> i32 {
        90
    }
}

/// Server-Sent Events (SSE) parser.
#[derive(Debug, Clone, Default)]
pub struct SseParser;

impl PayloadParser for SseParser {
    fn name(&self) -> &str {
        "sse"
    }

    fn can_parse(&self, content_type: &str, _host: &str) -> bool {
        content_type.to_lowercase().contains("text/event-stream")
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        let text = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        // Parse SSE format: "data: {...}\n\n"
        let mut data_lines = Vec::new();
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if !data.is_empty() && data != "[DONE]" {
                    data_lines.push(data);
                }
            }
        }

        // Try to parse each data line as JSON
        let json_parser = JsonParser;
        let mut all_prompts = Vec::new();

        for data in data_lines {
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                let prompts = json_parser.extract_generic(&json, context);
                all_prompts.extend(prompts);
            }
        }

        // Mark last as current
        if let Some(last) = all_prompts.last_mut() {
            last.is_current = true;
        }

        if !all_prompts.is_empty() {
            result.prompts = all_prompts;
            result.confidence = 0.75;
        }

        result
    }

    fn priority(&self) -> i32 {
        85
    }
}

/// Fallback parser that extracts all text content.
#[derive(Debug, Clone, Default)]
pub struct FallbackParser;

impl PayloadParser for FallbackParser {
    fn name(&self) -> &str {
        "fallback"
    }

    fn can_parse(&self, _content_type: &str, _host: &str) -> bool {
        true // Always can parse
    }

    fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let service = service_name(&context.host);
        let mut result = ParseResult::empty(self.name(), service);

        // Check for binary content
        if is_binary_content(body) {
            result.add_warning(ParseWarning::BinarySkipped);
            return result;
        }

        // Check size limit
        let body = if body.len() > context.max_payload_size {
            result.add_warning(ParseWarning::Truncated {
                original_size: body.len(),
                limit: context.max_payload_size,
            });
            &body[..context.max_payload_size]
        } else {
            body
        };

        let text = match std::str::from_utf8(body) {
            Ok(t) => t,
            Err(_) => {
                result.add_warning(ParseWarning::MalformedContent {
                    details: "Invalid UTF-8".to_string(),
                });
                return result;
            }
        };

        // Try JSON first
        if let Ok(json) = serde_json::from_str::<Value>(text) {
            let extracted = extract_all_text_from_json(&json);
            if !extracted.is_empty() {
                result.prompts = vec![ExtractedPrompt::new(extracted, true)];
                result.confidence = 0.4;
                result.add_warning(ParseWarning::FallbackUsed);
                return result;
            }
        }

        // Just use the raw text
        let text = text.trim();
        if text.len() >= MIN_EXTRACTED_STRING_LENGTH {
            result.prompts = vec![ExtractedPrompt::new(text, true)];
            result.confidence = 0.3;
            result.add_warning(ParseWarning::FallbackUsed);
        }

        result
    }

    fn priority(&self) -> i32 {
        -1000 // Lowest priority
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Checks if content appears to be binary.
fn is_binary_content(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }

    // Check first 512 bytes for binary indicators
    let check_len = body.len().min(512);
    let sample = &body[..check_len];

    // Count null bytes and control characters
    let null_count = sample.iter().filter(|&&b| b == 0).count();
    let control_count = sample
        .iter()
        .filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13)
        .count();

    // If more than 10% are null or control chars, likely binary
    let threshold = check_len / 10;
    null_count > threshold || control_count > threshold
}

/// Recursively extracts all string values from JSON.
fn extract_all_text_from_json(value: &Value) -> String {
    let mut texts = Vec::new();
    collect_text_recursive(value, &mut texts);
    texts.join(" ")
}

/// Recursively collects all string values.
fn collect_text_recursive(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            if s.len() >= MIN_EXTRACTED_STRING_LENGTH && !looks_like_id(s) {
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
    s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        || s.starts_with("eyJ") // JWT/base64
        || (s.chars().filter(|c| c.is_ascii_alphanumeric()).count() == s.len() && s.len() == 32)
}

/// Checks if a JSON key is likely metadata rather than content.
fn is_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "id" | "uuid"
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

// =============================================================================
// Parser Registry
// =============================================================================

/// Registry of payload parsers with priority ordering.
#[derive(Clone)]
pub struct ParserRegistry {
    parsers: Vec<Arc<dyn PayloadParser>>,
}

impl std::fmt::Debug for ParserRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserRegistry")
            .field("parsers", &self.parsers.len())
            .finish()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    /// Creates a registry with all built-in parsers.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(JsonParser));
        registry.register(Arc::new(NdjsonParser));
        registry.register(Arc::new(SseParser));
        registry.register(Arc::new(FormParser));
        registry.register(Arc::new(MultipartParser));
        registry.register(Arc::new(TextParser));
        registry.register(Arc::new(FallbackParser));
        registry
    }

    /// Registers a parser.
    pub fn register(&mut self, parser: Arc<dyn PayloadParser>) {
        self.parsers.push(parser);
        // Sort by priority (descending)
        self.parsers
            .sort_by_key(|p| std::cmp::Reverse(p.priority()));
    }

    /// Parses a payload using the first matching parser.
    pub fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        let content_type = context.mime_type().unwrap_or("application/octet-stream");

        for parser in &self.parsers {
            if parser.can_parse(content_type, &context.host) {
                let result = parser.parse(body, context);
                if result.has_prompts() {
                    return result;
                }
            }
        }

        // No parser succeeded
        ParseResult::empty("none", service_name(&context.host))
    }

    /// Tries to sniff the content type from the body.
    pub fn sniff_content_type(body: &[u8]) -> Option<&'static str> {
        if body.is_empty() {
            return None;
        }

        // Skip whitespace
        let trimmed = body.iter().skip_while(|&&b| b.is_ascii_whitespace());
        let first = trimmed.clone().next()?;

        match first {
            b'{' | b'[' => Some("application/json"),
            b'<' => Some("text/html"),
            _ => {
                // Check for form data (key=value)
                if body.windows(1).any(|w| w == b"=") && !body.contains(&b'\0') {
                    Some("application/x-www-form-urlencoded")
                } else if is_binary_content(body) {
                    Some("application/octet-stream")
                } else {
                    Some("text/plain")
                }
            }
        }
    }
}

// =============================================================================
// Streaming Accumulator
// =============================================================================

/// Accumulator for streaming content (SSE, chunked).
#[derive(Debug, Clone)]
pub struct StreamAccumulator {
    buffer: Vec<u8>,
    max_size: usize,
    content_type: Option<String>,
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamAccumulator {
    /// Creates a new accumulator with default max size (1MB).
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_size: DEFAULT_MAX_PAYLOAD_SIZE,
            content_type: None,
        }
    }

    /// Creates an accumulator with a custom max size.
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_size,
            content_type: None,
        }
    }

    /// Sets the content type.
    pub fn set_content_type(&mut self, content_type: impl Into<String>) {
        self.content_type = Some(content_type.into());
    }

    /// Appends a chunk to the buffer.
    pub fn append(&mut self, chunk: &[u8]) -> bool {
        if self.buffer.len() + chunk.len() > self.max_size {
            // Truncate
            let remaining = self.max_size.saturating_sub(self.buffer.len());
            self.buffer.extend_from_slice(&chunk[..remaining]);
            false
        } else {
            self.buffer.extend_from_slice(chunk);
            true
        }
    }

    /// Returns the accumulated content.
    pub fn content(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the buffer size.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Checks if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Checks if the stream looks complete (for SSE/NDJSON).
    pub fn looks_complete(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        let text = match std::str::from_utf8(&self.buffer) {
            Ok(t) => t,
            Err(_) => return false,
        };

        // For SSE, check for [DONE] marker
        if text.contains("data: [DONE]") || text.contains("event: done") {
            return true;
        }

        // For JSON, check for complete object/array
        let trimmed = text.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            return true;
        }

        false
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

// =============================================================================
// Smart Parser (Main Entry Point)
// =============================================================================

/// Smart content parser that uses the registry.
#[derive(Debug, Clone)]
pub struct SmartParser {
    registry: ParserRegistry,
}

impl Default for SmartParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartParser {
    /// Creates a new smart parser with default parsers.
    pub fn new() -> Self {
        Self {
            registry: ParserRegistry::with_defaults(),
        }
    }

    /// Creates a smart parser with a custom registry.
    pub fn with_registry(registry: ParserRegistry) -> Self {
        Self { registry }
    }

    /// Parses a payload and extracts prompts.
    pub fn parse(&self, body: &[u8], context: &ParseContext) -> ParseResult {
        // If no content type, try to sniff it
        let context = if context.content_type.is_none() {
            let sniffed = ParserRegistry::sniff_content_type(body);
            let mut ctx = context.clone();
            if let Some(ct) = sniffed {
                ctx.content_type = Some(ct.to_string());
            }
            ctx
        } else {
            context.clone()
        };

        self.registry.parse(body, &context)
    }

    /// Extracts prompts using simplified API (backwards compatible).
    pub fn extract_prompt(
        &self,
        host: &str,
        path: &str,
        body: &[u8],
    ) -> Option<crate::extractor::PromptInfo> {
        let context = ParseContext::new(host, path);
        let result = self.parse(body, &context);

        if result.has_prompts() {
            Some(crate::extractor::PromptInfo::new(
                result.combined_text(),
                result.service.clone(),
                path,
            ))
        } else {
            None
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ParseContext Tests ====================

    #[test]
    fn parse_context_new() {
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions");
        assert_eq!(ctx.host, "api.openai.com");
        assert_eq!(ctx.path, "/v1/chat/completions");
        assert!(ctx.scan_full_history);
    }

    #[test]
    fn parse_context_mime_type() {
        let ctx =
            ParseContext::new("host", "/path").with_content_type("application/json; charset=utf-8");
        assert_eq!(ctx.mime_type(), Some("application/json"));
    }

    #[test]
    fn parse_context_charset() {
        let ctx =
            ParseContext::new("host", "/path").with_content_type("application/json; charset=utf-8");
        assert_eq!(ctx.charset(), Some("utf-8"));
    }

    // ==================== ExtractedPrompt Tests ====================

    #[test]
    fn extracted_prompt_new() {
        let prompt = ExtractedPrompt::new("Hello world", true);
        assert_eq!(prompt.text, "Hello world");
        assert!(prompt.is_current);
        assert!(prompt.role.is_none());
    }

    #[test]
    fn extracted_prompt_with_role() {
        let prompt = ExtractedPrompt::new("Hello", false).with_role("user");
        assert!(prompt.is_user_message());
    }

    // ==================== ParseResult Tests ====================

    #[test]
    fn parse_result_empty() {
        let result = ParseResult::empty("test", "ChatGPT");
        assert!(!result.has_prompts());
        assert_eq!(result.parser_name, "test");
    }

    #[test]
    fn parse_result_with_prompts() {
        let prompts = vec![
            ExtractedPrompt::new("First", false),
            ExtractedPrompt::new("Second", true),
        ];
        let result = ParseResult::with_prompts(prompts, 0.9, "json", "ChatGPT");
        assert!(result.has_prompts());
        assert_eq!(result.current_prompt().unwrap().text, "Second");
    }

    #[test]
    fn parse_result_combined_text() {
        let prompts = vec![
            ExtractedPrompt::new("First", false),
            ExtractedPrompt::new("Second", true),
        ];
        let result = ParseResult::with_prompts(prompts, 0.9, "json", "ChatGPT");
        assert_eq!(result.combined_text(), "First\nSecond");
    }

    // ==================== JsonParser Tests ====================

    #[test]
    fn json_parser_can_parse() {
        let parser = JsonParser;
        assert!(parser.can_parse("application/json", "api.openai.com"));
        assert!(parser.can_parse("application/json; charset=utf-8", "api.openai.com"));
        assert!(!parser.can_parse("text/plain", "api.openai.com"));
    }

    #[test]
    fn json_parser_openai_simple() {
        let parser = JsonParser;
        let body = r#"{"messages": [{"role": "user", "content": "Hello, world!"}]}"#;
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions")
            .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello, world!");
        assert!(result.prompts[0].is_current);
    }

    #[test]
    fn json_parser_openai_multimodal() {
        let parser = JsonParser;
        let body = r#"{
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/img.jpg"}}
                ]
            }]
        }"#;
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions")
            .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "What is in this image?");
    }

    #[test]
    fn json_parser_anthropic() {
        let parser = JsonParser;
        let body = r#"{"messages": [{"role": "user", "content": "Hello Claude!"}]}"#;
        let ctx = ParseContext::new("api.anthropic.com", "/v1/messages")
            .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello Claude!");
    }

    #[test]
    fn json_parser_google() {
        let parser = JsonParser;
        let body = r#"{
            "contents": [{
                "parts": [{"text": "Hello Gemini!"}]
            }]
        }"#;
        let ctx = ParseContext::new(
            "generativelanguage.googleapis.com",
            "/v1/models/gemini:generateContent",
        )
        .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello Gemini!");
    }

    #[test]
    fn json_parser_with_bom() {
        let parser = JsonParser;
        let mut body = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        body.extend_from_slice(r#"{"prompt": "Hello with BOM"}"#.as_bytes());

        let ctx = ParseContext::new("unknown.com", "/api").with_content_type("application/json");
        let result = parser.parse(&body, &ctx);

        assert!(result.has_prompts());
        assert!(result.warnings.contains(&ParseWarning::BomStripped));
    }

    #[test]
    fn json_parser_trailing_comma() {
        let parser = JsonParser;
        let body = r#"{"prompt": "Hello",}"#; // Trailing comma
        let ctx = ParseContext::new("unknown.com", "/api").with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert!(result.warnings.contains(&ParseWarning::Json5Quirks));
    }

    #[test]
    fn json_parser_current_prompt_detection() {
        let parser = JsonParser;
        let body = r#"{
            "messages": [
                {"role": "user", "content": "First question"},
                {"role": "assistant", "content": "First answer"},
                {"role": "user", "content": "Second question"}
            ]
        }"#;
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions")
            .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert_eq!(result.prompts.len(), 2);
        assert!(!result.prompts[0].is_current); // First user message
        assert!(result.prompts[1].is_current); // Last user message
    }

    // ==================== FormParser Tests ====================

    #[test]
    fn form_parser_can_parse() {
        let parser = FormParser;
        assert!(parser.can_parse("application/x-www-form-urlencoded", "example.com"));
        assert!(!parser.can_parse("application/json", "example.com"));
    }

    #[test]
    fn form_parser_simple() {
        let parser = FormParser;
        let body = "prompt=Hello%20world&model=gpt-4";
        let ctx = ParseContext::new("api.example.com", "/generate")
            .with_content_type("application/x-www-form-urlencoded");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello world");
    }

    #[test]
    fn form_parser_plus_as_space() {
        let parser = FormParser;
        let body = "text=Hello+world+test";
        let ctx = ParseContext::new("api.example.com", "/")
            .with_content_type("application/x-www-form-urlencoded");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello world test");
    }

    // ==================== MultipartParser Tests ====================

    #[test]
    fn multipart_parser_can_parse() {
        let parser = MultipartParser;
        assert!(parser.can_parse(
            "multipart/form-data; boundary=----WebKitFormBoundary",
            "example.com"
        ));
        assert!(!parser.can_parse("application/json", "example.com"));
    }

    #[test]
    fn multipart_parser_simple() {
        let parser = MultipartParser;
        let body = "------boundary\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\nHello multipart world\r\n------boundary--";
        let ctx = ParseContext::new("api.example.com", "/upload")
            .with_content_type("multipart/form-data; boundary=----boundary");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello multipart world");
    }

    // ==================== TextParser Tests ====================

    #[test]
    fn text_parser_can_parse() {
        let parser = TextParser;
        assert!(parser.can_parse("text/plain", "example.com"));
        assert!(!parser.can_parse("application/json", "example.com"));
    }

    #[test]
    fn text_parser_simple() {
        let parser = TextParser;
        let body = "This is a plain text prompt for the AI.";
        let ctx = ParseContext::new("api.example.com", "/").with_content_type("text/plain");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(
            result.prompts[0].text,
            "This is a plain text prompt for the AI."
        );
    }

    // ==================== NdjsonParser Tests ====================

    #[test]
    fn ndjson_parser_can_parse() {
        let parser = NdjsonParser;
        assert!(parser.can_parse("application/x-ndjson", "example.com"));
        assert!(parser.can_parse("application/jsonl", "example.com"));
        assert!(!parser.can_parse("application/json", "example.com"));
    }

    #[test]
    fn ndjson_parser_simple() {
        let parser = NdjsonParser;
        let body = r#"{"prompt": "Line one"}
{"prompt": "Line two"}"#;
        let ctx = ParseContext::new("api.example.com", "/batch")
            .with_content_type("application/x-ndjson");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert_eq!(result.prompts.len(), 2);
        assert!(!result.prompts[0].is_current);
        assert!(result.prompts[1].is_current);
    }

    // ==================== SseParser Tests ====================

    #[test]
    fn sse_parser_can_parse() {
        let parser = SseParser;
        assert!(parser.can_parse("text/event-stream", "example.com"));
        assert!(!parser.can_parse("application/json", "example.com"));
    }

    #[test]
    fn sse_parser_simple() {
        let parser = SseParser;
        let body = "data: {\"text\": \"Hello from SSE\"}\n\ndata: [DONE]\n\n";
        let ctx =
            ParseContext::new("api.example.com", "/stream").with_content_type("text/event-stream");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
    }

    // ==================== FallbackParser Tests ====================

    #[test]
    fn fallback_parser_always_can_parse() {
        let parser = FallbackParser;
        assert!(parser.can_parse("anything/unknown", "example.com"));
    }

    #[test]
    fn fallback_parser_extracts_json() {
        let parser = FallbackParser;
        let body = r#"{"unknown_field": "Some interesting text content here"}"#;
        let ctx = ParseContext::new("unknown.com", "/");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert!(result.warnings.contains(&ParseWarning::FallbackUsed));
    }

    #[test]
    fn fallback_parser_skips_binary() {
        let parser = FallbackParser;
        let body = vec![0x00, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00];
        let ctx = ParseContext::new("unknown.com", "/");
        let result = parser.parse(&body, &ctx);

        assert!(!result.has_prompts());
        assert!(result.warnings.contains(&ParseWarning::BinarySkipped));
    }

    // ==================== ParserRegistry Tests ====================

    #[test]
    fn parser_registry_default() {
        let registry = ParserRegistry::with_defaults();
        assert!(registry.parsers.len() >= 6);
    }

    #[test]
    fn parser_registry_priority_ordering() {
        let registry = ParserRegistry::with_defaults();
        // JSON should be before fallback
        let json_idx = registry.parsers.iter().position(|p| p.name() == "json");
        let fallback_idx = registry.parsers.iter().position(|p| p.name() == "fallback");
        assert!(json_idx.unwrap() < fallback_idx.unwrap());
    }

    #[test]
    fn parser_registry_parse() {
        let registry = ParserRegistry::with_defaults();
        let body = r#"{"messages": [{"role": "user", "content": "Test message"}]}"#;
        let ctx =
            ParseContext::new("api.openai.com", "/v1/chat").with_content_type("application/json");
        let result = registry.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.parser_name, "json");
    }

    #[test]
    fn parser_registry_sniff_json() {
        assert_eq!(
            ParserRegistry::sniff_content_type(b"{\"key\": \"value\"}"),
            Some("application/json")
        );
        assert_eq!(
            ParserRegistry::sniff_content_type(b"[1, 2, 3]"),
            Some("application/json")
        );
    }

    #[test]
    fn parser_registry_sniff_html() {
        assert_eq!(
            ParserRegistry::sniff_content_type(b"<html>"),
            Some("text/html")
        );
    }

    #[test]
    fn parser_registry_sniff_form() {
        assert_eq!(
            ParserRegistry::sniff_content_type(b"key=value"),
            Some("application/x-www-form-urlencoded")
        );
    }

    // ==================== StreamAccumulator Tests ====================

    #[test]
    fn stream_accumulator_new() {
        let acc = StreamAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.max_size, DEFAULT_MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn stream_accumulator_append() {
        let mut acc = StreamAccumulator::new();
        assert!(acc.append(b"Hello "));
        assert!(acc.append(b"World"));
        assert_eq!(acc.content(), b"Hello World");
    }

    #[test]
    fn stream_accumulator_max_size() {
        let mut acc = StreamAccumulator::with_max_size(10);
        assert!(acc.append(b"12345"));
        assert!(!acc.append(b"67890123")); // Exceeds limit
        assert_eq!(acc.len(), 10);
    }

    #[test]
    fn stream_accumulator_looks_complete_json() {
        let mut acc = StreamAccumulator::new();
        acc.append(b"{\"key\": \"value\"}");
        assert!(acc.looks_complete());
    }

    #[test]
    fn stream_accumulator_looks_complete_sse() {
        let mut acc = StreamAccumulator::new();
        acc.append(b"data: {\"text\": \"hello\"}\n\ndata: [DONE]\n\n");
        assert!(acc.looks_complete());
    }

    // ==================== SmartParser Tests ====================

    #[test]
    fn smart_parser_new() {
        let parser = SmartParser::new();
        assert!(parser.registry.parsers.len() >= 6);
    }

    #[test]
    fn smart_parser_parse_json() {
        let parser = SmartParser::new();
        let body = r#"{"messages": [{"role": "user", "content": "Hello smart parser!"}]}"#;
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions")
            .with_content_type("application/json");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
        assert_eq!(result.prompts[0].text, "Hello smart parser!");
    }

    #[test]
    fn smart_parser_sniff_content_type() {
        let parser = SmartParser::new();
        let body = r#"{"messages": [{"role": "user", "content": "Sniffed JSON"}]}"#;
        // No content type set
        let ctx = ParseContext::new("api.openai.com", "/v1/chat/completions");
        let result = parser.parse(body.as_bytes(), &ctx);

        assert!(result.has_prompts());
    }

    #[test]
    fn smart_parser_extract_prompt_compat() {
        let parser = SmartParser::new();
        let body = r#"{"messages": [{"role": "user", "content": "Compat test"}]}"#;
        let result = parser.extract_prompt("api.openai.com", "/v1/chat", body.as_bytes());

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.text, "Compat test");
        assert_eq!(info.service, "ChatGPT");
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn is_binary_content_false_for_text() {
        assert!(!is_binary_content(b"Hello, world!"));
        assert!(!is_binary_content(b"{\"key\": \"value\"}"));
    }

    #[test]
    fn is_binary_content_true_for_binary() {
        let binary = vec![0x00, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(is_binary_content(&binary));
    }

    #[test]
    fn looks_like_id_true() {
        assert!(looks_like_id("a1b2c3d4-e5f6-7890-abcd-ef1234567890"));
        assert!(looks_like_id("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn looks_like_id_false() {
        assert!(!looks_like_id("Hello, world!"));
        assert!(!looks_like_id("This is a normal sentence."));
    }

    #[test]
    fn urlencoding_decode_basic() {
        assert_eq!(urlencoding_decode("Hello%20World"), "Hello World");
        assert_eq!(urlencoding_decode("Hello+World"), "Hello World");
        assert_eq!(urlencoding_decode("100%25"), "100%");
    }
}
