//! Dynamic Site Registry (F027).
//!
//! Replaces hardcoded LLM domain lists with a flexible, extensible registry.
//! Supports:
//! - Bundled default sites (compile-time)
//! - Custom user-added sites
//! - Wildcard patterns (`*.domain.com`, `**.domain.com`)
//! - Parser ID mapping (links to F026)
//! - Enable/disable per site
//! - LRU cache for fast lookups

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

// =============================================================================
// Site Entry Types
// =============================================================================

/// Category of LLM site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteCategory {
    /// Web chat interfaces (chatgpt.com, claude.ai).
    #[default]
    Consumer,
    /// Developer APIs (api.openai.com, api.anthropic.com).
    Api,
    /// Self-hosted, corporate (Azure OpenAI, Bedrock).
    Enterprise,
    /// AI image generation services (DALL-E, Stable Diffusion, Midjourney).
    ImageGen,
}

impl SiteCategory {
    /// Returns the category name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            SiteCategory::Consumer => "consumer",
            SiteCategory::Api => "api",
            SiteCategory::Enterprise => "enterprise",
            SiteCategory::ImageGen => "image_gen",
        }
    }

    /// Parses a category from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "consumer" => Some(SiteCategory::Consumer),
            "api" => Some(SiteCategory::Api),
            "enterprise" => Some(SiteCategory::Enterprise),
            "image_gen" => Some(SiteCategory::ImageGen),
            _ => None,
        }
    }

    /// Returns true if this category is for image generation services.
    pub fn is_image_gen(&self) -> bool {
        matches!(self, SiteCategory::ImageGen)
    }
}

/// Source of a site entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteSource {
    /// Bundled with the application (compile-time).
    #[default]
    Bundled,
    /// Downloaded from remote update server.
    Remote,
    /// Added by parent/user.
    Custom,
}

impl SiteSource {
    /// Returns the source name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            SiteSource::Bundled => "bundled",
            SiteSource::Remote => "remote",
            SiteSource::Custom => "custom",
        }
    }

    /// Parses a source from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bundled" => Some(SiteSource::Bundled),
            "remote" => Some(SiteSource::Remote),
            "custom" => Some(SiteSource::Custom),
            _ => None,
        }
    }
}

/// A site entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteEntry {
    /// Domain pattern (exact or wildcard like `*.domain.com`).
    pub pattern: String,
    /// Human-friendly display name.
    pub name: String,
    /// Site category.
    pub category: SiteCategory,
    /// Parser ID for F026 integration (None = auto-detect).
    pub parser_id: Option<String>,
    /// Whether this site is enabled for monitoring.
    pub enabled: bool,
    /// Source of this entry.
    pub source: SiteSource,
    /// Priority for pattern matching (higher = checked first).
    pub priority: i32,
}

impl SiteEntry {
    /// Creates a new site entry with default values.
    pub fn new(pattern: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            name: name.into(),
            category: SiteCategory::default(),
            parser_id: None,
            enabled: true,
            source: SiteSource::default(),
            priority: 0,
        }
    }

    /// Creates a bundled site entry.
    pub fn bundled(
        pattern: impl Into<String>,
        name: impl Into<String>,
        category: SiteCategory,
    ) -> Self {
        Self {
            pattern: pattern.into(),
            name: name.into(),
            category,
            parser_id: None,
            enabled: true,
            source: SiteSource::Bundled,
            priority: 0,
        }
    }

    /// Creates a custom site entry.
    pub fn custom(pattern: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            name: name.into(),
            category: SiteCategory::Enterprise,
            parser_id: None,
            enabled: true,
            source: SiteSource::Custom,
            priority: 100, // Custom sites have higher priority
        }
    }

    /// Sets the category.
    pub fn with_category(mut self, category: SiteCategory) -> Self {
        self.category = category;
        self
    }

    /// Sets the parser ID.
    pub fn with_parser_id(mut self, parser_id: impl Into<String>) -> Self {
        self.parser_id = Some(parser_id.into());
        self
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the source.
    pub fn with_source(mut self, source: SiteSource) -> Self {
        self.source = source;
        self
    }

    /// Checks if this is a wildcard pattern.
    pub fn is_wildcard(&self) -> bool {
        self.pattern.starts_with('*')
    }

    /// Checks if this is a double wildcard pattern (any depth).
    pub fn is_double_wildcard(&self) -> bool {
        self.pattern.starts_with("**.")
    }

    /// Checks if this is a single wildcard pattern (one level).
    pub fn is_single_wildcard(&self) -> bool {
        self.pattern.starts_with("*.") && !self.pattern.starts_with("**.")
    }

    /// Gets the base domain from a wildcard pattern.
    pub fn base_domain(&self) -> &str {
        if self.pattern.starts_with("**.") {
            &self.pattern[3..]
        } else if self.pattern.starts_with("*.") {
            &self.pattern[2..]
        } else {
            &self.pattern
        }
    }

    /// Checks if a host matches this pattern.
    pub fn matches(&self, host: &str) -> bool {
        let host = host.split(':').next().unwrap_or(host); // Remove port

        if self.is_double_wildcard() {
            // **.domain.com matches any depth subdomain
            let base = self.base_domain();
            host == base || host.ends_with(&format!(".{}", base))
        } else if self.is_single_wildcard() {
            // *.domain.com matches one level subdomain
            let base = self.base_domain();
            if host == base {
                return true;
            }
            if let Some(prefix) = host.strip_suffix(&format!(".{}", base)) {
                // Only match if there's exactly one subdomain level
                !prefix.contains('.')
            } else {
                false
            }
        } else {
            // Exact match
            host == self.pattern
        }
    }
}

// =============================================================================
// Bundled Default Sites
// =============================================================================

/// Version hash for update detection.
pub const BUNDLED_VERSION: &str = "v1.0.0";

/// Returns the bundled default sites.
pub fn bundled_sites() -> Vec<SiteEntry> {
    vec![
        // OpenAI - API
        SiteEntry::bundled("api.openai.com", "OpenAI API", SiteCategory::Api)
            .with_parser_id("openai_json")
            .with_priority(10),
        // OpenAI - Consumer
        SiteEntry::bundled("chat.openai.com", "ChatGPT", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("chatgpt.com", "ChatGPT", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("*.chatgpt.com", "ChatGPT", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Anthropic - API
        SiteEntry::bundled("api.anthropic.com", "Anthropic API", SiteCategory::Api)
            .with_parser_id("anthropic_json")
            .with_priority(10),
        // Anthropic - Consumer
        SiteEntry::bundled("claude.ai", "Claude", SiteCategory::Consumer)
            .with_parser_id("anthropic_json")
            .with_priority(10),
        SiteEntry::bundled("*.claude.ai", "Claude", SiteCategory::Consumer)
            .with_parser_id("anthropic_json")
            .with_priority(5),
        // Google - API
        SiteEntry::bundled(
            "generativelanguage.googleapis.com",
            "Google AI API",
            SiteCategory::Api,
        )
        .with_parser_id("google_json")
        .with_priority(10),
        // Google - Consumer
        SiteEntry::bundled("gemini.google.com", "Gemini", SiteCategory::Consumer)
            .with_parser_id("google_json")
            .with_priority(10),
        SiteEntry::bundled("aistudio.google.com", "AI Studio", SiteCategory::Consumer)
            .with_parser_id("google_json")
            .with_priority(10),
        // xAI (Grok)
        SiteEntry::bundled("api.x.ai", "xAI API", SiteCategory::Api)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("grok.x.ai", "Grok", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("x.ai", "xAI", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Perplexity
        SiteEntry::bundled("api.perplexity.ai", "Perplexity API", SiteCategory::Api)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("perplexity.ai", "Perplexity", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("*.perplexity.ai", "Perplexity", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Mistral
        SiteEntry::bundled("api.mistral.ai", "Mistral API", SiteCategory::Api)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("chat.mistral.ai", "Mistral Chat", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("mistral.ai", "Mistral", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Cohere
        SiteEntry::bundled("api.cohere.ai", "Cohere API", SiteCategory::Api)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("coral.cohere.com", "Cohere Coral", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        // Meta AI
        SiteEntry::bundled("meta.ai", "Meta AI", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("*.meta.ai", "Meta AI", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Microsoft Copilot
        SiteEntry::bundled("copilot.microsoft.com", "Copilot", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(10),
        SiteEntry::bundled("*.copilot.microsoft.com", "Copilot", SiteCategory::Consumer)
            .with_parser_id("openai_json")
            .with_priority(5),
        // Character AI
        SiteEntry::bundled("character.ai", "Character AI", SiteCategory::Consumer)
            .with_priority(10),
        SiteEntry::bundled("*.character.ai", "Character AI", SiteCategory::Consumer)
            .with_priority(5),
        // Hugging Face
        SiteEntry::bundled("huggingface.co", "Hugging Face", SiteCategory::Consumer)
            .with_priority(10),
        SiteEntry::bundled("*.huggingface.co", "Hugging Face", SiteCategory::Consumer)
            .with_priority(5),
        // =================================================================
        // Image Generation Services (F033)
        // =================================================================
        // Stability AI (Stable Diffusion)
        SiteEntry::bundled("api.stability.ai", "Stability AI", SiteCategory::ImageGen)
            .with_parser_id("stability_json")
            .with_priority(10),
        // Leonardo.ai
        SiteEntry::bundled("cloud.leonardo.ai", "Leonardo.ai", SiteCategory::ImageGen)
            .with_parser_id("leonardo_json")
            .with_priority(10),
        // Ideogram
        SiteEntry::bundled("api.ideogram.ai", "Ideogram", SiteCategory::ImageGen)
            .with_parser_id("ideogram_json")
            .with_priority(10),
        // Runway ML
        SiteEntry::bundled("api.runwayml.com", "Runway ML", SiteCategory::ImageGen)
            .with_parser_id("runway_json")
            .with_priority(10),
        // Black Forest Labs (Flux)
        SiteEntry::bundled("api.bfl.ml", "Black Forest Labs", SiteCategory::ImageGen)
            .with_parser_id("bfl_json")
            .with_priority(10),
        // Together AI (hosts Flux and other models)
        SiteEntry::bundled("api.together.xyz", "Together AI", SiteCategory::ImageGen)
            .with_parser_id("together_json")
            .with_priority(10),
        // Replicate (hosts many image models)
        SiteEntry::bundled("api.replicate.com", "Replicate", SiteCategory::ImageGen)
            .with_parser_id("replicate_json")
            .with_priority(10),
        // FAL.ai (hosts Flux and other models)
        SiteEntry::bundled("fal.run", "FAL.ai", SiteCategory::ImageGen)
            .with_parser_id("fal_json")
            .with_priority(10),
        SiteEntry::bundled("*.fal.run", "FAL.ai", SiteCategory::ImageGen)
            .with_parser_id("fal_json")
            .with_priority(5),
    ]
}

/// Returns bundled image generation sites only.
pub fn bundled_image_gen_sites() -> Vec<SiteEntry> {
    bundled_sites()
        .into_iter()
        .filter(|s| s.category == SiteCategory::ImageGen)
        .collect()
}

// =============================================================================
// LRU Cache
// =============================================================================

/// Simple LRU cache for lookup results.
#[derive(Debug)]
struct LruCache<K, V> {
    capacity: usize,
    entries: HashMap<K, V>,
    order: Vec<K>,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
        }
    }

    fn get(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.entries.get(key) {
            // Move to front (most recently used)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
                self.order.push(key.clone());
            }
            Some(value.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.entries.contains_key(&key) {
            // Update existing
            self.entries.insert(key.clone(), value);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push(key);
        } else {
            // Evict oldest if at capacity
            if self.entries.len() >= self.capacity {
                if let Some(oldest) = self.order.first().cloned() {
                    self.entries.remove(&oldest);
                    self.order.remove(0);
                }
            }
            self.entries.insert(key.clone(), value);
            self.order.push(key);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

// =============================================================================
// Site Registry
// =============================================================================

/// Default LRU cache size.
pub const DEFAULT_CACHE_SIZE: usize = 1000;

/// Result of a site lookup.
#[derive(Debug, Clone)]
pub struct SiteLookup {
    /// The matched site entry.
    pub entry: SiteEntry,
    /// Whether this was an exact match.
    pub exact_match: bool,
}

/// Thread-safe dynamic site registry.
#[derive(Debug)]
pub struct SiteRegistry {
    /// Exact match sites (O(1) lookup).
    exact_sites: RwLock<HashMap<String, SiteEntry>>,
    /// Wildcard patterns (checked in priority order).
    wildcard_sites: RwLock<Vec<SiteEntry>>,
    /// Disabled bundled site patterns.
    disabled_bundled: RwLock<Vec<String>>,
    /// LRU cache for resolved lookups.
    cache: RwLock<LruCache<String, Option<SiteLookup>>>,
}

impl Default for SiteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SiteRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            exact_sites: RwLock::new(HashMap::new()),
            wildcard_sites: RwLock::new(Vec::new()),
            disabled_bundled: RwLock::new(Vec::new()),
            cache: RwLock::new(LruCache::new(DEFAULT_CACHE_SIZE)),
        }
    }

    /// Creates a registry with bundled default sites.
    pub fn with_defaults() -> Self {
        let registry = Self::new();
        registry.load_bundled();
        registry
    }

    /// Loads bundled default sites.
    pub fn load_bundled(&self) {
        let sites = bundled_sites();
        let disabled = self.disabled_bundled.read().unwrap();

        let mut exact = self.exact_sites.write().unwrap();
        let mut wildcard = self.wildcard_sites.write().unwrap();

        for site in sites {
            // Skip disabled bundled sites
            if disabled.contains(&site.pattern) {
                continue;
            }

            if site.is_wildcard() {
                wildcard.push(site);
            } else {
                exact.insert(site.pattern.clone(), site);
            }
        }

        // Sort wildcards by priority (descending)
        wildcard.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Clear cache after reload
        self.cache.write().unwrap().clear();
    }

    /// Checks if a host is monitored (enabled site).
    pub fn is_monitored(&self, host: &str) -> bool {
        self.get_site(host)
            .is_some_and(|lookup| lookup.entry.enabled)
    }

    /// Checks if a host is an image generation domain (F033).
    pub fn is_image_gen_domain(&self, host: &str) -> bool {
        self.get_site(host)
            .is_some_and(|lookup| lookup.entry.enabled && lookup.entry.category.is_image_gen())
    }

    /// Gets the site entry for a host.
    pub fn get_site(&self, host: &str) -> Option<SiteLookup> {
        let host = host.split(':').next().unwrap_or(host).to_lowercase();

        // Check cache first
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(cached) = cache.get(&host) {
                return cached;
            }
        }

        // Try exact match first (O(1))
        let exact_result = {
            let exact = self.exact_sites.read().unwrap();
            exact.get(&host).cloned()
        };

        if let Some(entry) = exact_result {
            let lookup = SiteLookup {
                entry,
                exact_match: true,
            };
            let mut cache = self.cache.write().unwrap();
            cache.insert(host, Some(lookup.clone()));
            return Some(lookup);
        }

        // Try wildcard patterns (in priority order)
        let wildcard_result = {
            let wildcards = self.wildcard_sites.read().unwrap();
            wildcards.iter().find(|site| site.matches(&host)).cloned()
        };

        if let Some(entry) = wildcard_result {
            let lookup = SiteLookup {
                entry,
                exact_match: false,
            };
            let mut cache = self.cache.write().unwrap();
            cache.insert(host, Some(lookup.clone()));
            return Some(lookup);
        }

        // No match
        let mut cache = self.cache.write().unwrap();
        cache.insert(host, None);
        None
    }

    /// Gets the human-friendly service name for a host.
    pub fn service_name(&self, host: &str) -> &str {
        // Check for site entry
        if let Some(lookup) = self.get_site(host) {
            // We can't return a reference to the entry's name because it would
            // outlive the read lock. Instead, return static strings for known services.
            return Self::static_service_name(&lookup.entry.name);
        }
        "Unknown LLM"
    }

    /// Maps dynamic name to static string (for lifetime compatibility).
    fn static_service_name(name: &str) -> &'static str {
        match name {
            "OpenAI API" | "ChatGPT" => "ChatGPT",
            "Anthropic API" | "Claude" => "Claude",
            "Google AI API" | "Gemini" | "AI Studio" => "Gemini",
            "xAI API" | "Grok" | "xAI" => "Grok",
            "Perplexity API" | "Perplexity" => "Perplexity",
            "Mistral API" | "Mistral Chat" | "Mistral" => "Mistral",
            "Cohere API" | "Cohere Coral" => "Cohere",
            "Meta AI" => "Meta AI",
            "Copilot" => "Copilot",
            "Character AI" => "Character AI",
            "Hugging Face" => "Hugging Face",
            // Image Generation Services (F033)
            "Stability AI" => "Stability AI",
            "Leonardo.ai" => "Leonardo.ai",
            "Ideogram" => "Ideogram",
            "Runway ML" => "Runway ML",
            "Black Forest Labs" => "Black Forest Labs",
            "Together AI" => "Together AI",
            "Replicate" => "Replicate",
            "FAL.ai" => "FAL.ai",
            _ => "Unknown LLM",
        }
    }

    /// Gets the parser ID for a host.
    pub fn parser_id(&self, host: &str) -> Option<String> {
        self.get_site(host)
            .and_then(|lookup| lookup.entry.parser_id.clone())
    }

    /// Adds a custom site entry.
    pub fn add_custom(&self, entry: SiteEntry) {
        if entry.is_wildcard() {
            let mut wildcards = self.wildcard_sites.write().unwrap();
            wildcards.push(entry);
            wildcards.sort_by(|a, b| b.priority.cmp(&a.priority));
        } else {
            let mut exact = self.exact_sites.write().unwrap();
            exact.insert(entry.pattern.clone(), entry);
        }
        self.cache.write().unwrap().clear();
    }

    /// Sets the enabled state for a site pattern.
    pub fn set_enabled(&self, pattern: &str, enabled: bool) {
        // Check exact sites
        {
            let mut exact = self.exact_sites.write().unwrap();
            if let Some(entry) = exact.get_mut(pattern) {
                entry.enabled = enabled;
                self.cache.write().unwrap().clear();
                return;
            }
        }

        // Check wildcard sites
        {
            let mut wildcards = self.wildcard_sites.write().unwrap();
            if let Some(entry) = wildcards.iter_mut().find(|e| e.pattern == pattern) {
                entry.enabled = enabled;
                self.cache.write().unwrap().clear();
                return;
            }
        }

        // Handle disabling bundled sites
        if !enabled {
            let mut disabled = self.disabled_bundled.write().unwrap();
            if !disabled.contains(&pattern.to_string()) {
                disabled.push(pattern.to_string());
            }
        } else {
            let mut disabled = self.disabled_bundled.write().unwrap();
            disabled.retain(|p| p != pattern);
        }
    }

    /// Disables a bundled site (doesn't delete it).
    pub fn disable_bundled(&self, pattern: &str) {
        // Add to disabled list
        {
            let mut disabled = self.disabled_bundled.write().unwrap();
            if !disabled.contains(&pattern.to_string()) {
                disabled.push(pattern.to_string());
            }
        }
        // Also set enabled to false if it's loaded
        self.set_enabled(pattern, false);
    }

    /// Re-enables a bundled site.
    pub fn enable_bundled(&self, pattern: &str) {
        // Remove from disabled list
        {
            let mut disabled = self.disabled_bundled.write().unwrap();
            disabled.retain(|p| p != pattern);
        }

        // Check if it needs to be added back
        let needs_add = {
            let exact = self.exact_sites.read().unwrap();
            let wildcards = self.wildcard_sites.read().unwrap();
            !exact.contains_key(pattern) && !wildcards.iter().any(|e| e.pattern == pattern)
        };

        if needs_add {
            // Find in bundled list and add
            for site in bundled_sites() {
                if site.pattern == pattern {
                    if site.is_wildcard() {
                        let mut wildcards = self.wildcard_sites.write().unwrap();
                        wildcards.push(site);
                        wildcards.sort_by(|a, b| b.priority.cmp(&a.priority));
                    } else {
                        let mut exact = self.exact_sites.write().unwrap();
                        exact.insert(site.pattern.clone(), site);
                    }
                    break;
                }
            }
        }

        self.set_enabled(pattern, true);
        self.cache.write().unwrap().clear();
    }

    /// Reloads all sites (bundled + custom from callback).
    pub fn reload<F>(&self, load_custom: F)
    where
        F: FnOnce() -> Vec<SiteEntry>,
    {
        // Clear existing
        self.exact_sites.write().unwrap().clear();
        self.wildcard_sites.write().unwrap().clear();

        // Load bundled
        self.load_bundled();

        // Load custom
        let custom_sites = load_custom();
        for site in custom_sites {
            self.add_custom(site);
        }
    }

    /// Returns all exact match sites.
    pub fn exact_sites(&self) -> Vec<SiteEntry> {
        self.exact_sites.read().unwrap().values().cloned().collect()
    }

    /// Returns all wildcard sites.
    pub fn wildcard_sites(&self) -> Vec<SiteEntry> {
        self.wildcard_sites.read().unwrap().clone()
    }

    /// Returns all sites (exact + wildcard).
    pub fn all_sites(&self) -> Vec<SiteEntry> {
        let mut sites = self.exact_sites();
        sites.extend(self.wildcard_sites());
        sites
    }

    /// Returns disabled bundled patterns.
    pub fn disabled_bundled_patterns(&self) -> Vec<String> {
        self.disabled_bundled.read().unwrap().clone()
    }

    /// Validates a domain pattern.
    pub fn validate_pattern(pattern: &str) -> Result<(), String> {
        if pattern.is_empty() {
            return Err("Pattern cannot be empty".to_string());
        }

        // No scheme allowed
        if pattern.contains("://") {
            return Err("Pattern should not include scheme (http:// or https://)".to_string());
        }

        // No path allowed
        if pattern.contains('/') {
            return Err("Pattern should not include path".to_string());
        }

        // Valid characters check
        let pattern_without_wildcards = pattern.replace("**.", "").replace("*.", "");
        if !pattern_without_wildcards
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            return Err("Pattern contains invalid characters".to_string());
        }

        // Must have at least one dot (TLD)
        if !pattern.contains('.') && !pattern.starts_with('*') {
            return Err("Pattern must include a TLD (e.g., .com)".to_string());
        }

        Ok(())
    }

    /// Removes a custom site.
    pub fn remove_custom(&self, pattern: &str) {
        // Only allow removing custom sites
        {
            let mut exact = self.exact_sites.write().unwrap();
            if let Some(entry) = exact.get(pattern) {
                if entry.source == SiteSource::Custom {
                    exact.remove(pattern);
                    self.cache.write().unwrap().clear();
                    return;
                }
            }
        }

        {
            let mut wildcards = self.wildcard_sites.write().unwrap();
            let len_before = wildcards.len();
            wildcards.retain(|e| !(e.pattern == pattern && e.source == SiteSource::Custom));
            if wildcards.len() != len_before {
                self.cache.write().unwrap().clear();
            }
        }
    }
}

/// Creates a shared registry instance.
pub fn shared_registry() -> Arc<SiteRegistry> {
    Arc::new(SiteRegistry::with_defaults())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SiteCategory Tests ====================

    #[test]
    fn site_category_as_str() {
        assert_eq!(SiteCategory::Consumer.as_str(), "consumer");
        assert_eq!(SiteCategory::Api.as_str(), "api");
        assert_eq!(SiteCategory::Enterprise.as_str(), "enterprise");
    }

    #[test]
    fn site_category_from_str() {
        assert_eq!(
            SiteCategory::parse("consumer"),
            Some(SiteCategory::Consumer)
        );
        assert_eq!(SiteCategory::parse("API"), Some(SiteCategory::Api));
        assert_eq!(
            SiteCategory::parse("ENTERPRISE"),
            Some(SiteCategory::Enterprise)
        );
        assert_eq!(SiteCategory::parse("unknown"), None);
    }

    // ==================== SiteSource Tests ====================

    #[test]
    fn site_source_as_str() {
        assert_eq!(SiteSource::Bundled.as_str(), "bundled");
        assert_eq!(SiteSource::Remote.as_str(), "remote");
        assert_eq!(SiteSource::Custom.as_str(), "custom");
    }

    #[test]
    fn site_source_from_str() {
        assert_eq!(SiteSource::parse("bundled"), Some(SiteSource::Bundled));
        assert_eq!(SiteSource::parse("REMOTE"), Some(SiteSource::Remote));
        assert_eq!(SiteSource::parse("Custom"), Some(SiteSource::Custom));
        assert_eq!(SiteSource::parse("unknown"), None);
    }

    // ==================== SiteEntry Tests ====================

    #[test]
    fn site_entry_new() {
        let entry = SiteEntry::new("example.com", "Example");
        assert_eq!(entry.pattern, "example.com");
        assert_eq!(entry.name, "Example");
        assert!(entry.enabled);
        assert_eq!(entry.source, SiteSource::Bundled);
    }

    #[test]
    fn site_entry_custom() {
        let entry = SiteEntry::custom("custom.com", "Custom Site");
        assert_eq!(entry.source, SiteSource::Custom);
        assert_eq!(entry.category, SiteCategory::Enterprise);
        assert_eq!(entry.priority, 100);
    }

    #[test]
    fn site_entry_is_wildcard() {
        assert!(!SiteEntry::new("example.com", "").is_wildcard());
        assert!(SiteEntry::new("*.example.com", "").is_wildcard());
        assert!(SiteEntry::new("**.example.com", "").is_wildcard());
    }

    #[test]
    fn site_entry_is_single_wildcard() {
        assert!(!SiteEntry::new("example.com", "").is_single_wildcard());
        assert!(SiteEntry::new("*.example.com", "").is_single_wildcard());
        assert!(!SiteEntry::new("**.example.com", "").is_single_wildcard());
    }

    #[test]
    fn site_entry_is_double_wildcard() {
        assert!(!SiteEntry::new("example.com", "").is_double_wildcard());
        assert!(!SiteEntry::new("*.example.com", "").is_double_wildcard());
        assert!(SiteEntry::new("**.example.com", "").is_double_wildcard());
    }

    #[test]
    fn site_entry_base_domain() {
        assert_eq!(
            SiteEntry::new("example.com", "").base_domain(),
            "example.com"
        );
        assert_eq!(
            SiteEntry::new("*.example.com", "").base_domain(),
            "example.com"
        );
        assert_eq!(
            SiteEntry::new("**.example.com", "").base_domain(),
            "example.com"
        );
    }

    #[test]
    fn site_entry_matches_exact() {
        let entry = SiteEntry::new("api.openai.com", "");
        assert!(entry.matches("api.openai.com"));
        assert!(entry.matches("api.openai.com:443"));
        assert!(!entry.matches("openai.com"));
        assert!(!entry.matches("www.api.openai.com"));
    }

    #[test]
    fn site_entry_matches_single_wildcard() {
        let entry = SiteEntry::new("*.openai.com", "");
        assert!(entry.matches("api.openai.com"));
        assert!(entry.matches("chat.openai.com"));
        assert!(entry.matches("openai.com")); // Base domain matches
        assert!(!entry.matches("api.chat.openai.com")); // Two levels deep
    }

    #[test]
    fn site_entry_matches_double_wildcard() {
        let entry = SiteEntry::new("**.openai.com", "");
        assert!(entry.matches("api.openai.com"));
        assert!(entry.matches("chat.openai.com"));
        assert!(entry.matches("openai.com")); // Base domain matches
        assert!(entry.matches("api.chat.openai.com")); // Any depth
        assert!(entry.matches("a.b.c.openai.com")); // Any depth
    }

    // ==================== LruCache Tests ====================

    #[test]
    fn lru_cache_basic() {
        let mut cache = LruCache::new(3);
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
        assert_eq!(cache.get(&"d"), None);
    }

    #[test]
    fn lru_cache_eviction() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3); // Should evict "a"

        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
    }

    #[test]
    fn lru_cache_update_moves_to_front() {
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.get(&"a"); // Access "a" to make it recently used
        cache.insert("c", 3); // Should evict "b" (oldest)

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(3));
    }

    // ==================== SiteRegistry Tests ====================

    #[test]
    fn site_registry_new() {
        let registry = SiteRegistry::new();
        assert!(registry.exact_sites().is_empty());
        assert!(registry.wildcard_sites().is_empty());
    }

    #[test]
    fn site_registry_with_defaults() {
        let registry = SiteRegistry::with_defaults();
        assert!(!registry.exact_sites().is_empty());
        assert!(!registry.wildcard_sites().is_empty());
    }

    #[test]
    fn site_registry_is_monitored() {
        let registry = SiteRegistry::with_defaults();
        assert!(registry.is_monitored("api.openai.com"));
        assert!(registry.is_monitored("claude.ai"));
        assert!(registry.is_monitored("gemini.google.com"));
        assert!(!registry.is_monitored("example.com"));
    }

    #[test]
    fn site_registry_is_monitored_with_port() {
        let registry = SiteRegistry::with_defaults();
        assert!(registry.is_monitored("api.openai.com:443"));
        assert!(registry.is_monitored("claude.ai:443"));
    }

    #[test]
    fn site_registry_is_monitored_case_insensitive() {
        let registry = SiteRegistry::with_defaults();
        assert!(registry.is_monitored("API.OPENAI.COM"));
        assert!(registry.is_monitored("Claude.AI"));
    }

    #[test]
    fn site_registry_wildcard_matching() {
        let registry = SiteRegistry::with_defaults();
        assert!(registry.is_monitored("www.chatgpt.com"));
        assert!(registry.is_monitored("cdn.chatgpt.com"));
    }

    #[test]
    fn site_registry_get_site() {
        let registry = SiteRegistry::with_defaults();

        let lookup = registry.get_site("api.openai.com").unwrap();
        assert!(lookup.exact_match);
        assert_eq!(lookup.entry.name, "OpenAI API");

        let lookup = registry.get_site("www.chatgpt.com").unwrap();
        assert!(!lookup.exact_match); // Wildcard match
    }

    #[test]
    fn site_registry_service_name() {
        let registry = SiteRegistry::with_defaults();
        assert_eq!(registry.service_name("api.openai.com"), "ChatGPT");
        assert_eq!(registry.service_name("claude.ai"), "Claude");
        assert_eq!(registry.service_name("gemini.google.com"), "Gemini");
        assert_eq!(registry.service_name("example.com"), "Unknown LLM");
    }

    #[test]
    fn site_registry_parser_id() {
        let registry = SiteRegistry::with_defaults();
        assert_eq!(
            registry.parser_id("api.openai.com"),
            Some("openai_json".to_string())
        );
        assert_eq!(
            registry.parser_id("api.anthropic.com"),
            Some("anthropic_json".to_string())
        );
    }

    #[test]
    fn site_registry_add_custom() {
        let registry = SiteRegistry::with_defaults();

        let custom =
            SiteEntry::custom("myai.example.com", "My AI").with_category(SiteCategory::Enterprise);
        registry.add_custom(custom);

        assert!(registry.is_monitored("myai.example.com"));
        let lookup = registry.get_site("myai.example.com").unwrap();
        assert_eq!(lookup.entry.name, "My AI");
        assert_eq!(lookup.entry.source, SiteSource::Custom);
    }

    #[test]
    fn site_registry_add_custom_wildcard() {
        let registry = SiteRegistry::with_defaults();

        let custom = SiteEntry::custom("*.mycompany.com", "My Company AI");
        registry.add_custom(custom);

        assert!(registry.is_monitored("ai.mycompany.com"));
        assert!(registry.is_monitored("llm.mycompany.com"));
    }

    #[test]
    fn site_registry_set_enabled() {
        let registry = SiteRegistry::with_defaults();

        assert!(registry.is_monitored("api.openai.com"));

        registry.set_enabled("api.openai.com", false);
        assert!(!registry.is_monitored("api.openai.com"));

        registry.set_enabled("api.openai.com", true);
        assert!(registry.is_monitored("api.openai.com"));
    }

    #[test]
    fn site_registry_disable_bundled() {
        let registry = SiteRegistry::with_defaults();

        registry.disable_bundled("api.openai.com");
        assert!(!registry.is_monitored("api.openai.com"));
        assert!(registry
            .disabled_bundled_patterns()
            .contains(&"api.openai.com".to_string()));
    }

    #[test]
    fn site_registry_enable_bundled() {
        let registry = SiteRegistry::with_defaults();

        registry.disable_bundled("api.openai.com");
        assert!(!registry.is_monitored("api.openai.com"));

        registry.enable_bundled("api.openai.com");
        assert!(registry.is_monitored("api.openai.com"));
        assert!(!registry
            .disabled_bundled_patterns()
            .contains(&"api.openai.com".to_string()));
    }

    #[test]
    fn site_registry_remove_custom() {
        let registry = SiteRegistry::with_defaults();

        let custom = SiteEntry::custom("myai.example.com", "My AI");
        registry.add_custom(custom);
        assert!(registry.is_monitored("myai.example.com"));

        registry.remove_custom("myai.example.com");
        assert!(!registry.is_monitored("myai.example.com"));
    }

    #[test]
    fn site_registry_remove_custom_doesnt_remove_bundled() {
        let registry = SiteRegistry::with_defaults();

        // Try to remove bundled site
        registry.remove_custom("api.openai.com");
        // Should still be monitored
        assert!(registry.is_monitored("api.openai.com"));
    }

    #[test]
    fn site_registry_reload() {
        let registry = SiteRegistry::with_defaults();

        // Add custom site
        registry.add_custom(SiteEntry::custom("custom.com", "Custom"));
        assert!(registry.is_monitored("custom.com"));

        // Reload without custom sites
        registry.reload(Vec::new);
        assert!(!registry.is_monitored("custom.com"));
        assert!(registry.is_monitored("api.openai.com")); // Bundled still works
    }

    #[test]
    fn site_registry_reload_with_custom() {
        let registry = SiteRegistry::with_defaults();

        registry.reload(|| vec![SiteEntry::custom("reloaded.com", "Reloaded")]);

        assert!(registry.is_monitored("reloaded.com"));
        assert!(registry.is_monitored("api.openai.com"));
    }

    #[test]
    fn site_registry_caching() {
        let registry = SiteRegistry::with_defaults();

        // First lookup (miss)
        let result1 = registry.get_site("api.openai.com");
        assert!(result1.is_some());

        // Second lookup (hit from cache)
        let result2 = registry.get_site("api.openai.com");
        assert!(result2.is_some());

        // Both should return same result
        assert_eq!(result1.unwrap().entry.name, result2.unwrap().entry.name);
    }

    // ==================== Validation Tests ====================

    #[test]
    fn validate_pattern_valid() {
        assert!(SiteRegistry::validate_pattern("example.com").is_ok());
        assert!(SiteRegistry::validate_pattern("api.example.com").is_ok());
        assert!(SiteRegistry::validate_pattern("*.example.com").is_ok());
        assert!(SiteRegistry::validate_pattern("**.example.com").is_ok());
        assert!(SiteRegistry::validate_pattern("my-site.example.com").is_ok());
    }

    #[test]
    fn validate_pattern_empty() {
        assert!(SiteRegistry::validate_pattern("").is_err());
    }

    #[test]
    fn validate_pattern_with_scheme() {
        assert!(SiteRegistry::validate_pattern("https://example.com").is_err());
        assert!(SiteRegistry::validate_pattern("http://example.com").is_err());
    }

    #[test]
    fn validate_pattern_with_path() {
        assert!(SiteRegistry::validate_pattern("example.com/api").is_err());
    }

    #[test]
    fn validate_pattern_invalid_chars() {
        assert!(SiteRegistry::validate_pattern("example.com?query").is_err());
        assert!(SiteRegistry::validate_pattern("example.com#hash").is_err());
    }

    // ==================== Priority Tests ====================

    #[test]
    fn site_registry_exact_match_priority_over_wildcard() {
        let registry = SiteRegistry::new();

        // Add wildcard first
        registry.add_custom(
            SiteEntry::new("*.example.com", "Wildcard")
                .with_source(SiteSource::Custom)
                .with_priority(5),
        );

        // Add exact match
        registry.add_custom(
            SiteEntry::new("api.example.com", "Exact")
                .with_source(SiteSource::Custom)
                .with_priority(5),
        );

        let lookup = registry.get_site("api.example.com").unwrap();
        assert!(lookup.exact_match);
        assert_eq!(lookup.entry.name, "Exact");
    }

    #[test]
    fn site_registry_more_specific_wildcard_wins() {
        let registry = SiteRegistry::new();

        // Add broad wildcard
        registry.add_custom(
            SiteEntry::new("**.example.com", "Broad")
                .with_source(SiteSource::Custom)
                .with_priority(1),
        );

        // Add specific wildcard with higher priority
        registry.add_custom(
            SiteEntry::new("*.api.example.com", "Specific")
                .with_source(SiteSource::Custom)
                .with_priority(10),
        );

        let lookup = registry.get_site("v1.api.example.com").unwrap();
        assert_eq!(lookup.entry.name, "Specific");
    }

    // ==================== Bundled Sites Tests ====================

    #[test]
    fn bundled_sites_not_empty() {
        let sites = bundled_sites();
        assert!(!sites.is_empty());
    }

    #[test]
    fn bundled_sites_all_have_names() {
        for site in bundled_sites() {
            assert!(!site.name.is_empty(), "Site {} has no name", site.pattern);
        }
    }

    #[test]
    fn bundled_sites_all_valid_patterns() {
        for site in bundled_sites() {
            assert!(
                SiteRegistry::validate_pattern(&site.pattern).is_ok(),
                "Invalid pattern: {}",
                site.pattern
            );
        }
    }

    #[test]
    fn bundled_sites_cover_major_llms() {
        let registry = SiteRegistry::with_defaults();

        // Major LLM services should be covered
        assert!(registry.is_monitored("api.openai.com"));
        assert!(registry.is_monitored("chatgpt.com"));
        assert!(registry.is_monitored("claude.ai"));
        assert!(registry.is_monitored("api.anthropic.com"));
        assert!(registry.is_monitored("gemini.google.com"));
        assert!(registry.is_monitored("perplexity.ai"));
        assert!(registry.is_monitored("mistral.ai"));
    }

    // ==================== Image Generation Tests (F033) ====================

    #[test]
    fn site_category_image_gen() {
        assert_eq!(SiteCategory::ImageGen.as_str(), "image_gen");
        assert_eq!(
            SiteCategory::parse("image_gen"),
            Some(SiteCategory::ImageGen)
        );
        assert!(SiteCategory::ImageGen.is_image_gen());
        assert!(!SiteCategory::Consumer.is_image_gen());
        assert!(!SiteCategory::Api.is_image_gen());
    }

    #[test]
    fn bundled_sites_cover_image_gen_services() {
        let registry = SiteRegistry::with_defaults();

        // Image generation services should be covered
        assert!(registry.is_monitored("api.stability.ai"));
        assert!(registry.is_monitored("cloud.leonardo.ai"));
        assert!(registry.is_monitored("api.ideogram.ai"));
        assert!(registry.is_monitored("api.runwayml.com"));
        assert!(registry.is_monitored("api.bfl.ml"));
        assert!(registry.is_monitored("api.together.xyz"));
        assert!(registry.is_monitored("api.replicate.com"));
        assert!(registry.is_monitored("fal.run"));
    }

    #[test]
    fn is_image_gen_domain_works() {
        let registry = SiteRegistry::with_defaults();

        // Image gen domains
        assert!(registry.is_image_gen_domain("api.stability.ai"));
        assert!(registry.is_image_gen_domain("cloud.leonardo.ai"));
        assert!(registry.is_image_gen_domain("api.ideogram.ai"));

        // Non-image gen domains (regular LLM APIs)
        assert!(!registry.is_image_gen_domain("api.openai.com"));
        assert!(!registry.is_image_gen_domain("claude.ai"));
        assert!(!registry.is_image_gen_domain("chatgpt.com"));

        // Unknown domains
        assert!(!registry.is_image_gen_domain("example.com"));
    }

    #[test]
    fn bundled_image_gen_sites_filter() {
        let image_gen_sites = bundled_image_gen_sites();
        assert!(!image_gen_sites.is_empty());

        // All sites should be ImageGen category
        for site in &image_gen_sites {
            assert_eq!(
                site.category,
                SiteCategory::ImageGen,
                "Site {} should be ImageGen category",
                site.pattern
            );
        }
    }

    #[test]
    fn image_gen_sites_have_parser_ids() {
        let image_gen_sites = bundled_image_gen_sites();

        // Most image gen sites should have parser IDs
        let sites_with_parser: Vec<_> = image_gen_sites
            .iter()
            .filter(|s| s.parser_id.is_some())
            .collect();

        assert!(
            !sites_with_parser.is_empty(),
            "At least some image gen sites should have parser IDs"
        );
    }
}
