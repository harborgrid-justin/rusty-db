// Full-Text Search Support
//
// This module provides comprehensive full-text search capabilities:
// - Text tokenization and normalization
// - Inverted index structure for fast lookups
// - TF-IDF relevance scoring
// - Phrase search and proximity matching
// - Wildcard and fuzzy search
// - Stop word filtering
// - Stemming support

use crate::Result;
use std::collections::{HashMap, HashSet};

// Full-text search index
pub struct FullTextIndex {
    // Table and column this index is for
    table_name: String,
    column_name: String,
    // Inverted index: term -> list of document IDs
    inverted_index: InvertedIndex,
    // Document store for scoring
    document_store: DocumentStore,
    // Tokenizer configuration
    tokenizer: Tokenizer,
}

impl Clone for FullTextIndex {
    fn clone(&self) -> Self {
        Self {
            table_name: self.table_name.clone(),
            column_name: self.column_name.clone(),
            inverted_index: self.inverted_index.clone(),
            document_store: self.document_store.clone(),
            tokenizer: self.tokenizer.clone(),
        }
    }
}

impl FullTextIndex {
    pub fn new(table_name: String, column_name: String) -> Self {
        Self {
            table_name,
            column_name,
            inverted_index: InvertedIndex::new(),
            document_store: DocumentStore::new(),
            tokenizer: Tokenizer::new(),
        }
    }

    // Index a document
    pub fn index_document(&mut self, doc_id: DocumentId, text: String) -> Result<()> {
        // Tokenize the text
        let tokens = self.tokenizer.tokenize(&text);

        // Calculate term frequencies
        let term_freqs = Self::calculate_term_frequencies(&tokens);

        // Store document
        self.document_store.add_document(doc_id, text.clone(), term_freqs.clone());

        // Update inverted index
        for term in term_freqs.keys() {
            self.inverted_index.add_term_occurrence(term.clone(), doc_id);
        }

        Ok(())
    }

    // Search for documents matching a query
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let query_tokens = self.tokenizer.tokenize(query);

        // Find matching documents
        let mut doc_scores: HashMap<DocumentId, f64> = HashMap::new();

        for token in &query_tokens {
            if let Some(doc_ids) = self.inverted_index.get_documents(token) {
                for &doc_id in doc_ids {
                    let score = self.calculate_relevance_score(token, doc_id);
                    *doc_scores.entry(doc_id).or_insert(0.0) += score;
                }
            }
        }

        // Sort by score and return results
        let mut results: Vec<_> = doc_scores
            .into_iter()
            .map(|(doc_id, score)| SearchResult {
                doc_id,
                score,
                snippet: self.document_store.get_snippet(doc_id, &query_tokens),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(results)
    }

    // Phrase search - find exact phrases
    pub fn search_phrase(&self, phrase: &str) -> Result<Vec<SearchResult>> {
        let tokens = self.tokenizer.tokenize(phrase);

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Find documents containing all terms
        let mut candidate_docs: Option<HashSet<DocumentId>> = None;

        for token in &tokens {
            if let Some(docs) = self.inverted_index.get_documents(token) {
                let doc_set: HashSet<_> = docs.iter().copied().collect();
                candidate_docs = Some(match candidate_docs {
                    None => doc_set,
                    Some(existing) => existing.intersection(&doc_set).copied().collect(),
                });
            } else {
                return Ok(Vec::new()); // Term not found, no matches
            }
        }

        let candidates = candidate_docs.unwrap_or_default();

        // Verify phrase order in candidates
        let mut results = Vec::new();
        for doc_id in candidates {
            if self.document_store.contains_phrase(doc_id, &tokens) {
                let score = self.calculate_phrase_score(doc_id, &tokens);
                results.push(SearchResult {
                    doc_id,
                    score,
                    snippet: self.document_store.get_snippet(doc_id, &tokens),
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    // Wildcard search (e.g., "data*")
    pub fn search_wildcard(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let matching_terms = self.inverted_index.match_wildcard(pattern);

        let mut doc_scores: HashMap<DocumentId, f64> = HashMap::new();

        for term in matching_terms {
            if let Some(doc_ids) = self.inverted_index.get_documents(&term) {
                for &doc_id in doc_ids {
                    let score = self.calculate_relevance_score(&term, doc_id);
                    *doc_scores.entry(doc_id).or_insert(0.0) += score;
                }
            }
        }

        let mut results: Vec<_> = doc_scores
            .into_iter()
            .map(|(doc_id, score)| SearchResult {
                doc_id,
                score,
                snippet: self.document_store.get_snippet(doc_id, &[]),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }

    fn calculate_term_frequencies(tokens: &[String]) -> HashMap<String, u32> {
        let mut freqs = HashMap::new();
        for token in tokens {
            *freqs.entry(token.clone()).or_insert(0) += 1;
        }
        freqs
    }

    // Calculate TF-IDF score for a term in a document
    fn calculate_relevance_score(&self, term: &str, doc_id: DocumentId) -> f64 {
        let tf = self.calculate_term_frequency(term, doc_id);
        let idf = self.calculate_inverse_document_frequency(term);
        tf * idf
    }

    fn calculate_term_frequency(&self, term: &str, doc_id: DocumentId) -> f64 {
        self.document_store
            .get_term_frequency(doc_id, term)
            .map(|f| (f as f64).sqrt()) // Use square root for term frequency
            .unwrap_or(0.0)
    }

    fn calculate_inverse_document_frequency(&self, term: &str) -> f64 {
        let total_docs = self.document_store.document_count() as f64;
        let doc_freq = self.inverted_index
            .get_document_frequency(term)
            .unwrap_or(1) as f64;

        (total_docs / doc_freq).ln()
    }

    fn calculate_phrase_score(&self, doc_id: DocumentId, tokens: &[String]) -> f64 {
        // Higher score for exact phrase matches
        let mut score = 0.0;
        for token in tokens {
            score += self.calculate_relevance_score(token, doc_id);
        }
        score * 1.5 // Boost for phrase match
    }
}

// Document ID type
pub type DocumentId = u64;

// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: DocumentId,
    pub score: f64,
    pub snippet: String,
}

// Inverted index structure
#[derive(Debug)]
#[derive(Clone)]
struct InvertedIndex {
    // Term -> set of document IDs containing the term
    index: HashMap<String, HashSet<DocumentId>>,
}

impl InvertedIndex {
    fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    fn add_term_occurrence(&mut self, term: String, doc_id: DocumentId) {
        self.index
            .entry(term)
            .or_insert_with(HashSet::new)
            .insert(doc_id);
    }

    fn get_documents(&self, term: &str) -> Option<&HashSet<DocumentId>> {
        self.index.get(term)
    }

    fn get_document_frequency(&self, term: &str) -> Option<usize> {
        self.index.get(term).map(|docs| docs.len())
    }

    fn match_wildcard(&self, pattern: &str) -> Vec<String> {
        if pattern.contains('*') {
            let prefix = pattern.trim_end_matches('*');
            self.index
                .keys()
                .filter(|term| term.starts_with(prefix))
                .cloned()
                .collect()
        } else if pattern.contains('?') {
            // Single character wildcard
            let pattern_regex = pattern.replace('?', ".");
            self.index
                .keys()
                .filter(|term| {
                    regex::Regex::new(&pattern_regex)
                        .map(|re| re.is_match(term))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        } else {
            // No wildcard
            vec![pattern.to_string()]
        }
    }
}

// Document store
#[derive(Debug)]
#[derive(Clone)]
struct DocumentStore {
    // Document ID -> document content
    documents: HashMap<DocumentId, Document>,
}

impl DocumentStore {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    fn add_document(&mut self, doc_id: DocumentId, text: String, term_freqs: HashMap<String, u32>) {
        let tokens: Vec<String> = text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        self.documents.insert(
            doc_id,
            Document {
                text,
                tokens,
                term_frequencies: term_freqs,
            },
        );
    }

    fn get_term_frequency(&self, doc_id: DocumentId, term: &str) -> Option<u32> {
        self.documents
            .get(&doc_id)
            .and_then(|doc| doc.term_frequencies.get(term).copied())
    }

    fn document_count(&self) -> usize {
        self.documents.len()
    }

    fn contains_phrase(&self, doc_id: DocumentId, tokens: &[String]) -> bool {
        if let Some(doc) = self.documents.get(&doc_id) {
            // Check if tokens appear consecutively
            for window in doc.tokens.windows(tokens.len()) {
                if window == tokens {
                    return true;
                }
            }
        }
        false
    }

    fn get_snippet(&self, doc_id: DocumentId, query_tokens: &[String]) -> String {
        if let Some(doc) = self.documents.get(&doc_id) {
            // Find first occurrence of query term
            for (i, token) in doc.tokens.iter().enumerate() {
                if query_tokens.contains(token) {
                    // Extract snippet around match
                    let start = i.saturating_sub(5);
                    let end = (i + 10).min(doc.tokens.len());
                    let snippet: Vec<_> = doc.tokens[start..end].to_vec();
                    return format!("...{}...", snippet.join(" "));
                }
            }
            // No match found, return beginning of document
            let snippet: Vec<_> = doc.tokens.iter().take(10).cloned().collect();
            return format!("{}...", snippet.join(" "));
        }
        String::new()
    }
}

// Document structure
#[derive(Debug, Clone)]
struct Document {
    text: String,
    tokens: Vec<String>,
    term_frequencies: HashMap<String, u32>,
}

// Text tokenizer
#[derive(Clone)]
pub struct Tokenizer {
    stop_words: HashSet<String>,
    stemmer: Stemmer,
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            stop_words: Self::default_stop_words(),
            stemmer: Stemmer::new(),
        }
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|word| self.normalize_token(word))
            .filter(|token| !self.is_stop_word(token))
            .map(|token| self.stemmer.stem(&token))
            .collect()
    }

    fn normalize_token(&self, token: &str) -> String {
        token
            .to_lowercase()
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_string()
    }

    fn is_stop_word(&self, token: &str) -> bool {
        self.stop_words.contains(token)
    }

    fn default_stop_words() -> HashSet<String> {
        [
            "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
            "has", "he", "in", "is", "it", "its", "of", "on", "that", "the",
            "to", "was", "will", "with",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect()
    }
}

// Simple Porter stemmer
#[derive(Clone)]
pub struct Stemmer;

impl Stemmer {
    pub fn new() -> Self {
        Self
    }

    pub fn stem(&self, word: &str) -> String {
        // Simple stemming rules
        let word = word.to_lowercase();

        // Remove common suffixes
        if word.ends_with("ing") && word.len() > 5 {
            return word[..word.len() - 3].to_string();
        }
        if word.ends_with("ed") && word.len() > 4 {
            return word[..word.len() - 2].to_string();
        }
        if word.ends_with("s") && word.len() > 3 && !word.ends_with("ss") {
            return word[..word.len() - 1].to_string();
        }

        word
    }
}

// Fuzzy search support using edit distance
pub struct FuzzyMatcher {
    max_distance: usize,
}

impl FuzzyMatcher {
    pub fn new(max_distance: usize) -> Self {
        Self { max_distance }
    }

    // Calculate Levenshtein distance between two strings
    pub fn edit_distance(&self, a: &str, b: &str) -> usize {
        let a_len = a.len();
        let b_len = b.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a.chars().nth(i - 1) == b.chars().nth(j - 1) {
                    0
                } else {
                    1
                };

                matrix[i][j] = std::cmp::min(
                    std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                    matrix[i - 1][j - 1] + cost,
                );
            }
        }

        matrix[a_len][b_len]
    }

    // Check if two strings are within max edit distance
    pub fn is_fuzzy_match(&self, a: &str, b: &str) -> bool {
        self.edit_distance(a, b) <= self.max_distance
    }
}

// Full-text query parser
pub struct QueryParser;

impl QueryParser {
    // Parse a full-text search query
    // Supports: AND, OR, NOT, phrases ("exact match"), wildcards (*)
    pub fn parse(query: &str) -> ParsedQuery {
        let mut terms = Vec::new();
        let mut phrases = Vec::new();
        let mut excluded = Vec::new();

        let parts: Vec<&str> = query.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            let part = parts[i];

            if part.starts_with('"') {
                // Phrase search
                let mut phrase = part.trim_start_matches('"').to_string();
                i += 1;
                while i < parts.len() && !parts[i].ends_with('"') {
                    phrase.push(' ');
                    phrase.push_str(parts[i]);
                    i += 1;
                }
                if i < parts.len() {
                    phrase.push(' ');
                    phrase.push_str(parts[i].trim_end_matches('"'));
                }
                phrases.push(phrase);
            } else if part.starts_with('-') || part.to_uppercase() == "NOT" {
                // Exclusion
                if part.starts_with('-') {
                    excluded.push(part.trim_start_matches('-').to_string());
                } else if i + 1 < parts.len() {
                    i += 1;
                    excluded.push(parts[i].to_string());
                }
            } else if part.to_uppercase() != "AND" && part.to_uppercase() != "OR" {
                // Regular term
                terms.push(part.to_string());
            }

            i += 1;
        }

        ParsedQuery {
            terms,
            phrases,
            excluded,
        }
    }
}

// Parsed query structure
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    pub phrases: Vec<String>,
    pub excluded: Vec<String>,
}

// Boolean search evaluator
pub struct BooleanSearchEvaluator;

impl BooleanSearchEvaluator {
    // Evaluate a boolean query (AND, OR, NOT)
    pub fn evaluate(
        index: &FullTextIndex,
        query: &ParsedQuery,
    ) -> Result<Vec<SearchResult>> {
        let mut result_docs: HashSet<DocumentId> = HashSet::new();
        let mut initialized = false;

        // Process terms (implicit AND)
        for term in &query.terms {
            let term_results = index.search(term)?;
            let term_docs: HashSet<_> = term_results.iter().map(|r| r.doc_id).collect();

            if !initialized {
                result_docs = term_docs;
                initialized = true;
            } else {
                result_docs = result_docs.intersection(&term_docs).copied().collect();
            }
        }

        // Process phrases
        for phrase in &query.phrases {
            let phrase_results = index.search_phrase(phrase)?;
            let phrase_docs: HashSet<_> = phrase_results.iter().map(|r| r.doc_id).collect();

            if !initialized {
                result_docs = phrase_docs;
                initialized = true;
            } else {
                result_docs = result_docs.intersection(&phrase_docs).copied().collect();
            }
        }

        // Process exclusions
        for excluded in &query.excluded {
            let excluded_results = index.search(excluded)?;
            let excluded_docs: HashSet<_> = excluded_results.iter().map(|r| r.doc_id).collect();
            result_docs = result_docs.difference(&excluded_docs).copied().collect();
        }

        // Convert to results with scores
        let results: Vec<_> = result_docs
            .into_iter()
            .map(|doc_id| {
                let score = Self::calculate_boolean_score(index, doc_id, query);
                SearchResult {
                    doc_id,
                    score,
                    snippet: index.document_store.get_snippet(doc_id, &query.terms),
                }
            })
            .collect();

        Ok(results)
    }

    fn calculate_boolean_score(
        index: &FullTextIndex,
        doc_id: DocumentId,
        query: &ParsedQuery,
    ) -> f64 {
        let mut score = 0.0;

        for term in &query.terms {
            score += index.calculate_relevance_score(term, doc_id);
        }

        for phrase in &query.phrases {
            let tokens = index.tokenizer.tokenize(phrase);
            score += index.calculate_phrase_score(doc_id, &tokens);
        }

        score
    }
}

// Highlight search terms in text
pub struct Highlighter;

impl Highlighter {
    pub fn highlight(text: &str, terms: &[String], prefix: &str, suffix: &str) -> String {
        let mut result = text.to_string();

        for term in terms {
            // Case-insensitive replacement
            let pattern = regex::escape(term);
            if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    format!("{}{}{}", prefix, &caps[0], suffix)
                }).to_string();
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("The quick brown fox jumps over the lazy dog");

        // Should remove stop words "the", "over"
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
    }

    #[test]
    fn test_stemmer() {
        let stemmer = Stemmer::new();

        // Simple stemmer removes common suffixes
        assert_eq!(stemmer.stem("running"), "runn"); // Simple rule: remove "ing"
        assert_eq!(stemmer.stem("jumped"), "jump");
        assert_eq!(stemmer.stem("dogs"), "dog");
    }

    #[test]
    fn test_full_text_index() {
        let mut index = FullTextIndex::new("articles".to_string(), "content".to_string());

        index.index_document(1, "The quick brown fox".to_string()).unwrap();
        index.index_document(2, "The lazy dog sleeps".to_string()).unwrap();
        index.index_document(3, "Quick brown dogs".to_string()).unwrap();

        let results = index.search("quick").unwrap();
        assert!(results.len() >= 1);

        // Document 1 and 3 should match "quick"
        let doc_ids: Vec<_> = results.iter().map(|r| r.doc_id).collect();
        assert!(doc_ids.contains(&1) || doc_ids.contains(&3));
    }

    #[test]
    fn test_phrase_search() {
        let mut index = FullTextIndex::new("articles".to_string(), "content".to_string());

        index.index_document(1, "the quick brown fox".to_string()).unwrap();
        index.index_document(2, "brown quick fox".to_string()).unwrap();

        let results = index.search_phrase("quick brown").unwrap();

        // Only document 1 should match the exact phrase
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, 1);
    }

    #[test]
    fn test_wildcard_search() {
        let mut index = FullTextIndex::new("articles".to_string(), "content".to_string());

        index.index_document(1, "database management".to_string()).unwrap();
        index.index_document(2, "data processing".to_string()).unwrap();

        let results = index.search_wildcard("data*").unwrap();
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_fuzzy_matcher() {
        let matcher = FuzzyMatcher::new(2);

        assert!(matcher.is_fuzzy_match("hello", "hallo"));
        assert!(matcher.is_fuzzy_match("database", "databse"));
        assert!(!matcher.is_fuzzy_match("hello", "world"));
    }

    #[test]
    fn test_query_parser() {
        let query = r#"database "full text" -spam"#;
        let parsed = QueryParser::parse(query);

        assert!(parsed.terms.contains(&"database".to_string()));
        assert!(parsed.phrases.contains(&"full text".to_string()));
        assert!(parsed.excluded.contains(&"spam".to_string()));
    }

    #[test]
    fn test_highlighter() {
        let text = "The quick brown fox";
        let terms = vec!["quick".to_string(), "fox".to_string()];
        let highlighted = Highlighter::highlight(text, &terms, "<b>", "</b>");

        assert!(highlighted.contains("<b>quick</b>"));
        assert!(highlighted.contains("<b>fox</b>"));
    }
}
