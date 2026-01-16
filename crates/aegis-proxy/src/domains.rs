//! LLM domain filtering.
//!
//! Defines which domains should be intercepted for content filtering
//! and which should be passed through.
//!
//! This module now uses the dynamic SiteRegistry (F027) for flexible
//! site management while maintaining backward compatibility.

use std::sync::Arc;

use aegis_core::site_registry::{bundled_sites, SiteRegistry};
use once_cell::sync::Lazy;

/// Global site registry instance.
static SITE_REGISTRY: Lazy<Arc<SiteRegistry>> =
    Lazy::new(|| Arc::new(SiteRegistry::with_defaults()));

/// List of LLM service domains to intercept.
///
/// Traffic to these domains will be inspected for prompt content.
/// All other domains are passed through without inspection.
///
/// Note: This is kept for backward compatibility. New code should use
/// `SITE_REGISTRY.is_monitored()` instead.
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
/// This function now uses the dynamic SiteRegistry (F027) for checking,
/// which supports wildcards, custom sites, and disabled bundled sites.
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
    SITE_REGISTRY.is_monitored(host)
}

/// Returns the global site registry instance.
///
/// Use this for advanced operations like adding custom sites,
/// disabling bundled sites, or getting parser IDs.
pub fn get_registry() -> Arc<SiteRegistry> {
    Arc::clone(&SITE_REGISTRY)
}

/// Gets a clone of the default bundled sites.
///
/// This returns the compiled-in default list, useful for
/// resetting or inspecting the bundled sites.
pub fn get_bundled_sites() -> Vec<aegis_core::site_registry::SiteEntry> {
    bundled_sites()
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
///
/// This function now uses the dynamic SiteRegistry (F027) which
/// provides service names from site entries.
pub fn service_name(host: &str) -> &'static str {
    SITE_REGISTRY.service_name(host)
}

/// Gets the parser ID for a host from the registry.
///
/// This links to F026 Smart Content Parsing - each site can specify
/// which parser to use for its payloads.
pub fn parser_id(host: &str) -> Option<String> {
    SITE_REGISTRY.parser_id(host)
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

    // ==================== Registry Tests ====================

    #[test]
    fn get_registry_returns_registry() {
        let registry = get_registry();
        assert!(registry.is_monitored("api.openai.com"));
    }

    #[test]
    fn get_bundled_sites_not_empty() {
        let sites = get_bundled_sites();
        assert!(!sites.is_empty());
    }

    #[test]
    fn parser_id_openai() {
        let id = parser_id("api.openai.com");
        assert_eq!(id, Some("openai_json".to_string()));
    }

    #[test]
    fn parser_id_anthropic() {
        let id = parser_id("api.anthropic.com");
        assert_eq!(id, Some("anthropic_json".to_string()));
    }

    #[test]
    fn parser_id_unknown() {
        let id = parser_id("example.com");
        assert!(id.is_none());
    }
}
