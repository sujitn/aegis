//! LLM domain filtering.
//!
//! Defines which domains should be intercepted for content filtering
//! and which should be passed through.

/// List of LLM service domains to intercept.
///
/// Traffic to these domains will be inspected for prompt content.
/// All other domains are passed through without inspection.
pub const LLM_DOMAINS: &[&str] = &[
    // OpenAI
    "api.openai.com",
    "chat.openai.com",
    "chatgpt.com",
    // Anthropic
    "claude.ai",
    "api.anthropic.com",
    // Google
    "gemini.google.com",
    "generativelanguage.googleapis.com",
    "aistudio.google.com",
    // xAI (Grok)
    "grok.x.ai",
    "api.x.ai",
    "x.ai",
    // Perplexity
    "perplexity.ai",
    "api.perplexity.ai",
    // Mistral
    "mistral.ai",
    "chat.mistral.ai",
    "api.mistral.ai",
    // Cohere
    "coral.cohere.com",
    "api.cohere.ai",
    // Meta AI
    "meta.ai",
    // Microsoft Copilot
    "copilot.microsoft.com",
    // Character AI
    "character.ai",
    // Hugging Face
    "huggingface.co",
];

/// Checks if the given host is an LLM domain that should be intercepted.
///
/// # Examples
///
/// ```
/// use aegis_proxy::is_llm_domain;
///
/// assert!(is_llm_domain("api.openai.com"));
/// assert!(is_llm_domain("claude.ai"));
/// assert!(!is_llm_domain("google.com"));
/// assert!(!is_llm_domain("example.com"));
/// ```
pub fn is_llm_domain(host: &str) -> bool {
    // Remove port if present
    let host = host.split(':').next().unwrap_or(host);

    // Check exact match
    if LLM_DOMAINS.contains(&host) {
        return true;
    }

    // Check subdomain match (e.g., www.chatgpt.com -> chatgpt.com)
    for domain in LLM_DOMAINS {
        if host.ends_with(&format!(".{}", domain)) {
            return true;
        }
    }

    false
}

/// Returns the primary domain name for logging purposes.
///
/// Strips common subdomains like "www" and "api".
#[allow(dead_code)]
pub fn normalize_domain(host: &str) -> &str {
    // Remove port if present
    let host = host.split(':').next().unwrap_or(host);

    // Strip common prefixes
    if let Some(stripped) = host.strip_prefix("www.") {
        return stripped;
    }
    if host.starts_with("api.") {
        // Keep api. for API endpoints as they're significant
        return host;
    }

    host
}

/// Returns a human-friendly name for the LLM service.
pub fn service_name(host: &str) -> &'static str {
    let host = host.split(':').next().unwrap_or(host);

    if host.contains("openai.com") || host.contains("chatgpt.com") {
        "ChatGPT"
    } else if host.contains("claude.ai") || host.contains("anthropic.com") {
        "Claude"
    } else if host.contains("gemini.google.com")
        || host.contains("googleapis.com")
        || host.contains("aistudio.google.com")
    {
        "Gemini"
    } else if host.contains("x.ai") || host.contains("grok") {
        "Grok"
    } else if host.contains("perplexity.ai") {
        "Perplexity"
    } else if host.contains("mistral.ai") {
        "Mistral"
    } else if host.contains("cohere") {
        "Cohere"
    } else if host.contains("meta.ai") {
        "Meta AI"
    } else if host.contains("copilot.microsoft.com") {
        "Copilot"
    } else if host.contains("character.ai") {
        "Character AI"
    } else if host.contains("huggingface.co") {
        "Hugging Face"
    } else {
        "Unknown LLM"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== is_llm_domain Tests ====================

    #[test]
    fn is_llm_domain_openai() {
        assert!(is_llm_domain("api.openai.com"));
        assert!(is_llm_domain("chat.openai.com"));
        assert!(is_llm_domain("chatgpt.com"));
    }

    #[test]
    fn is_llm_domain_anthropic() {
        assert!(is_llm_domain("claude.ai"));
        assert!(is_llm_domain("api.anthropic.com"));
    }

    #[test]
    fn is_llm_domain_google() {
        assert!(is_llm_domain("gemini.google.com"));
        assert!(is_llm_domain("generativelanguage.googleapis.com"));
    }

    #[test]
    fn is_llm_domain_with_port() {
        assert!(is_llm_domain("api.openai.com:443"));
        assert!(is_llm_domain("claude.ai:443"));
    }

    #[test]
    fn is_llm_domain_subdomains() {
        assert!(is_llm_domain("www.chatgpt.com"));
        assert!(is_llm_domain("cdn.chatgpt.com"));
    }

    #[test]
    fn is_not_llm_domain() {
        assert!(!is_llm_domain("google.com"));
        assert!(!is_llm_domain("example.com"));
        assert!(!is_llm_domain("openai.org")); // Different TLD
        assert!(!is_llm_domain("notchatgpt.com")); // Contains but not subdomain
    }

    // ==================== normalize_domain Tests ====================

    #[test]
    fn normalize_domain_strips_port() {
        assert_eq!(normalize_domain("example.com:443"), "example.com");
    }

    #[test]
    fn normalize_domain_strips_www() {
        assert_eq!(normalize_domain("www.example.com"), "example.com");
    }

    #[test]
    fn normalize_domain_keeps_api() {
        assert_eq!(normalize_domain("api.openai.com"), "api.openai.com");
    }

    #[test]
    fn normalize_domain_unchanged() {
        assert_eq!(normalize_domain("example.com"), "example.com");
    }

    // ==================== service_name Tests ====================

    #[test]
    fn service_name_openai() {
        assert_eq!(service_name("api.openai.com"), "ChatGPT");
        assert_eq!(service_name("chat.openai.com"), "ChatGPT");
        assert_eq!(service_name("chatgpt.com"), "ChatGPT");
    }

    #[test]
    fn service_name_anthropic() {
        assert_eq!(service_name("claude.ai"), "Claude");
        assert_eq!(service_name("api.anthropic.com"), "Claude");
    }

    #[test]
    fn service_name_google() {
        assert_eq!(service_name("gemini.google.com"), "Gemini");
        assert_eq!(service_name("generativelanguage.googleapis.com"), "Gemini");
    }

    #[test]
    fn service_name_unknown() {
        assert_eq!(service_name("example.com"), "Unknown LLM");
    }
}
