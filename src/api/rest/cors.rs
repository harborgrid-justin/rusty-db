// # CORS Security Module
//
// Implements secure CORS configuration with trie-based origin matching
// to prevent CSRF attacks. This module provides:
//
// - Efficient O(n) origin validation using a trie data structure
// - Support for exact origin matches and subdomain wildcards
// - Safe defaults for development and production
// - Protection against CSRF and unauthorized cross-origin requests
//
// ## Security Considerations
//
// NEVER use `allow_origin(Any)` in production as it allows ANY origin
// to make authenticated requests, enabling CSRF attacks. Always specify
// a whitelist of trusted origins.

use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use axum::http::{Method, HeaderValue, header};

/// Trie node for efficient origin matching
#[derive(Debug, Clone, Default)]
struct TrieNode {
    /// Child nodes indexed by character
    children: HashMap<char, Box<TrieNode>>,
    /// Whether this node represents the end of a valid origin
    is_terminal: bool,
    /// Whether this node allows all subdomains (wildcard)
    is_wildcard: bool,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_terminal: false,
            is_wildcard: false,
        }
    }
}

/// Trie-based origin matcher for efficient O(n) lookups
/// where n is the length of the origin string
#[derive(Debug, Clone)]
pub struct OriginMatcher {
    /// Root of the trie (stores reversed origins for suffix matching)
    root: TrieNode,
    /// Number of origins stored
    count: usize,
}

impl OriginMatcher {
    /// Create a new empty origin matcher
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            count: 0,
        }
    }

    /// Create an origin matcher from a list of allowed origins
    pub fn from_origins(origins: &[String]) -> Self {
        let mut matcher = Self::new();
        for origin in origins {
            matcher.add_origin(origin);
        }
        matcher
    }

    /// Add an origin to the matcher
    ///
    /// Examples:
    /// - "https://example.com" - exact match
    /// - "https://*.example.com" - wildcard subdomain match
    pub fn add_origin(&mut self, origin: &str) {
        let origin = origin.trim();

        // Handle wildcard patterns like "https://*.example.com"
        let (pattern, is_wildcard) = if origin.contains("*.") {
            (origin.replace("*.", ""), true)
        } else {
            (origin.to_string(), false)
        };

        // Reverse the string for suffix matching (makes subdomain matching easier)
        let reversed: String = pattern.chars().rev().collect();

        let mut node = &mut self.root;
        for ch in reversed.chars() {
            node = node.children
                .entry(ch)
                .or_insert_with(|| Box::new(TrieNode::new()));
        }

        node.is_terminal = true;
        node.is_wildcard = is_wildcard;
        self.count += 1;
    }

    /// Check if an origin is allowed
    ///
    /// Returns true if the origin matches an exact pattern or a wildcard pattern
    pub fn is_allowed(&self, origin: &str) -> bool {
        let origin = origin.trim();

        // Exact match first
        if self.exact_match(origin) {
            return true;
        }

        // Check wildcard match (e.g., subdomain.example.com matches *.example.com)
        if self.wildcard_match(origin) {
            return true;
        }

        false
    }

    /// Check for exact match
    fn exact_match(&self, origin: &str) -> bool {
        let reversed: String = origin.chars().rev().collect();
        let mut node = &self.root;

        for ch in reversed.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return false,
            }
        }

        node.is_terminal && !node.is_wildcard
    }

    /// Check for wildcard match (subdomain matching)
    fn wildcard_match(&self, origin: &str) -> bool {
        let reversed: String = origin.chars().rev().collect();

        // Try to find a wildcard terminal at any position
        self.find_wildcard_terminal(&self.root, &reversed, 0)
    }

    /// Recursively search for wildcard terminal nodes
    fn find_wildcard_terminal(&self, node: &TrieNode, reversed: &str, pos: usize) -> bool {
        // If we found a wildcard terminal, this is a match
        if node.is_terminal && node.is_wildcard {
            return true;
        }

        // If we've exhausted the string, no match
        if pos >= reversed.len() {
            return false;
        }

        // Try to continue matching
        if let Some(ch) = reversed.chars().nth(pos) {
            if let Some(child) = node.children.get(&ch) {
                return self.find_wildcard_terminal(child, reversed, pos + 1);
            }
        }

        false
    }

    /// Get the number of origins in the matcher
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for OriginMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a secure CORS layer with origin validation
pub fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    // If allowed origins is empty or contains "*", use permissive CORS for development
    // WARNING: This should NEVER be used in production!
    if allowed_origins.is_empty() || allowed_origins.contains(&"*".to_string()) {
        tracing::warn!(
            "CORS configured with allow_origin(*) - THIS IS INSECURE! \
             Only use this in development. In production, specify exact origins."
        );

        return CorsLayer::permissive();
    }

    // Build the origin matcher
    let matcher = OriginMatcher::from_origins(allowed_origins);

    tracing::info!(
        "CORS configured with {} allowed origins",
        matcher.count()
    );

    // Create CORS layer with dynamic origin validation
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
        ])
        .allow_credentials(true)
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            move |origin: &HeaderValue, _request_parts| {
                if let Ok(origin_str) = origin.to_str() {
                    matcher.is_allowed(origin_str)
                } else {
                    false
                }
            }
        ))
}

/// Get safe default CORS origins for development
pub fn development_origins() -> Vec<String> {
    vec![
        "http://localhost:3000".to_string(),
        "http://localhost:8080".to_string(),
        "http://127.0.0.1:3000".to_string(),
        "http://127.0.0.1:8080".to_string(),
    ]
}

/// Get safe default CORS origins for production
///
/// Returns an empty vector - production deployments MUST
/// explicitly configure allowed origins
pub fn production_origins() -> Vec<String> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_origin_match() {
        let mut matcher = OriginMatcher::new();
        matcher.add_origin("https://example.com");

        assert!(matcher.is_allowed("https://example.com"));
        assert!(!matcher.is_allowed("https://evil.com"));
        assert!(!matcher.is_allowed("https://subdomain.example.com"));
    }

    #[test]
    fn test_wildcard_subdomain_match() {
        let mut matcher = OriginMatcher::new();
        matcher.add_origin("https://*.example.com");

        assert!(matcher.is_allowed("https://subdomain.example.com"));
        assert!(matcher.is_allowed("https://api.example.com"));
        assert!(matcher.is_allowed("https://www.example.com"));
        assert!(!matcher.is_allowed("https://example.com")); // base domain not included
        assert!(!matcher.is_allowed("https://evil.com"));
    }

    #[test]
    fn test_multiple_origins() {
        let matcher = OriginMatcher::from_origins(&[
            "https://example.com".to_string(),
            "https://api.example.com".to_string(),
            "http://localhost:3000".to_string(),
        ]);

        assert_eq!(matcher.count(), 3);
        assert!(matcher.is_allowed("https://example.com"));
        assert!(matcher.is_allowed("https://api.example.com"));
        assert!(matcher.is_allowed("http://localhost:3000"));
        assert!(!matcher.is_allowed("https://evil.com"));
    }

    #[test]
    fn test_localhost_origins() {
        let matcher = OriginMatcher::from_origins(&[
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ]);

        assert!(matcher.is_allowed("http://localhost:3000"));
        assert!(matcher.is_allowed("http://127.0.0.1:3000"));
        assert!(!matcher.is_allowed("http://localhost:8080"));
        assert!(!matcher.is_allowed("https://localhost:3000")); // Different protocol
    }

    #[test]
    fn test_development_origins() {
        let origins = development_origins();
        assert!(origins.len() >= 4);
        assert!(origins.contains(&"http://localhost:3000".to_string()));
    }

    #[test]
    fn test_production_origins() {
        let origins = production_origins();
        assert!(origins.is_empty(), "Production should require explicit configuration");
    }

    #[test]
    fn test_trie_efficiency() {
        // Test that the trie can handle many origins efficiently
        let mut matcher = OriginMatcher::new();

        for i in 0..1000 {
            matcher.add_origin(&format!("https://subdomain{}.example.com", i));
        }

        assert_eq!(matcher.count(), 1000);
        assert!(matcher.is_allowed("https://subdomain500.example.com"));
        assert!(!matcher.is_allowed("https://subdomain1001.example.com"));
    }

    #[test]
    fn test_protocol_sensitivity() {
        let mut matcher = OriginMatcher::new();
        matcher.add_origin("https://example.com");

        assert!(matcher.is_allowed("https://example.com"));
        assert!(!matcher.is_allowed("http://example.com")); // Different protocol
    }

    #[test]
    fn test_port_sensitivity() {
        let mut matcher = OriginMatcher::new();
        matcher.add_origin("http://localhost:3000");

        assert!(matcher.is_allowed("http://localhost:3000"));
        assert!(!matcher.is_allowed("http://localhost:3001")); // Different port
    }
}
