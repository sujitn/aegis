//! Aegis Proxy - MITM proxy for intercepting LLM traffic (F016, F026).
//!
//! This crate provides a transparent HTTPS proxy that intercepts traffic to LLM
//! services (ChatGPT, Claude, Gemini, etc.) and applies Aegis filtering rules.
//!
//! ## Smart Content Parsing (F026)
//!
//! The [`smart_parser`] module provides robust prompt extraction from diverse
//! LLM payload formats with an extensible parser registry supporting JSON,
//! form data, multipart, NDJSON, SSE, and raw text.
//!
//! ## Features
//!
//! - Generates root CA certificate on first run
//! - Creates per-domain certificates on the fly
//! - Intercepts only LLM domains (passthrough for others)
//! - Extracts prompts from request bodies
//! - Applies classification and rules (F007)
//! - Blocks or forwards based on rule evaluation
//! - Injects block page for blocked requests
//! - Logs events to storage (F008)
//!
//! ## Architecture
//!
//! ```text
//! Client Request → Proxy → Domain Check → LLM Domain?
//!                                           │
//!                         ┌─────────────────┴─────────────────┐
//!                         │ No                                │ Yes
//!                         ▼                                   ▼
//!                    Passthrough                       Extract Prompt
//!                                                            │
//!                                                            ▼
//!                                                     Classify (F007)
//!                                                            │
//!                                           ┌────────────────┴────────────────┐
//!                                           │ Allow/Warn                      │ Block
//!                                           ▼                                 ▼
//!                                      Forward Request                   Block Page
//! ```

mod ca;
mod domains;
mod error;
mod extractor;
mod handler;
mod proxy;
pub mod setup;
pub mod smart_parser;
pub mod state_cache;

pub use ca::{CaManager, CaManagerError};
pub use domains::{get_bundled_sites, get_registry, is_llm_domain, parser_id, LLM_DOMAINS};
pub use error::{ProxyError, Result};
pub use extractor::{extract_prompt, PromptInfo};
pub use handler::{FilteringState, HandlerConfig, ProxyHandler};
pub use proxy::{ProxyConfig, ProxyServer};
pub use setup::{
    disable_system_proxy, enable_system_proxy, install_ca_certificate, is_ca_installed,
    is_proxy_enabled, setup_proxy, teardown_proxy, uninstall_ca_certificate, ProxySetup,
    SetupResult,
};
pub use smart_parser::{
    ExtractedPrompt, ParseContext, ParseResult, ParseWarning, ParserRegistry, PayloadParser,
    SmartParser, StreamAccumulator,
};
pub use state_cache::{StateCache, DEFAULT_POLL_INTERVAL};

/// Default proxy port.
pub const DEFAULT_PROXY_PORT: u16 = 8766;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_is_correct() {
        assert_eq!(DEFAULT_PROXY_PORT, 8766);
    }
}
