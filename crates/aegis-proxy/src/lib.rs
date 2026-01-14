//! Aegis Proxy - MITM proxy for intercepting LLM traffic (F016).
//!
//! This crate provides a transparent HTTPS proxy that intercepts traffic to LLM
//! services (ChatGPT, Claude, Gemini, etc.) and applies Aegis filtering rules.
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

pub use ca::{CaManager, CaManagerError};
pub use domains::{is_llm_domain, LLM_DOMAINS};
pub use error::{ProxyError, Result};
pub use extractor::{extract_prompt, PromptInfo};
pub use handler::ProxyHandler;
pub use proxy::{ProxyConfig, ProxyServer};

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
