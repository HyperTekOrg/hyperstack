//! Search utilities for IDL specs with fuzzy matching

use crate::types::IdlSpec;
use strsim::levenshtein;

/// A fuzzy match suggestion with candidate name and edit distance.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub candidate: String,
    pub distance: usize,
}

/// Which section of the IDL a search result came from.
#[derive(Debug, Clone)]
pub enum IdlSection {
    Instruction,
    Account,
    Type,
    Error,
    Event,
    Constant,
}

/// How a search result was matched.
#[derive(Debug, Clone)]
pub enum MatchType {
    Exact,
    CaseInsensitive,
    Contains,
    Fuzzy(usize),
}

/// A single search result from `search_idl`.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub section: IdlSection,
    pub match_type: MatchType,
}

/// Suggest similar names from a list of candidates using fuzzy matching.
///
/// Returns candidates sorted by edit distance (closest first).
/// Exact matches are excluded. Case-insensitive matches get distance 0,
/// substring matches get distance 1, and Levenshtein matches use their
/// actual edit distance.
pub fn suggest_similar(name: &str, candidates: &[&str], max_distance: usize) -> Vec<Suggestion> {
    let name_lower = name.to_lowercase();
    let mut suggestions: Vec<Suggestion> = candidates
        .iter()
        .filter_map(|&candidate| {
            // Skip exact matches
            if candidate == name {
                return None;
            }
            let candidate_lower = candidate.to_lowercase();
            // Case-insensitive match
            if candidate_lower == name_lower {
                return Some(Suggestion {
                    candidate: candidate.to_string(),
                    distance: 0,
                });
            }
            // Substring match
            if candidate_lower.contains(&name_lower) || name_lower.contains(&candidate_lower) {
                return Some(Suggestion {
                    candidate: candidate.to_string(),
                    distance: 1,
                });
            }
            // Levenshtein distance
            let dist = levenshtein(name, candidate);
            if dist <= max_distance {
                Some(Suggestion {
                    candidate: candidate.to_string(),
                    distance: dist,
                })
            } else {
                None
            }
        })
        .collect();
    suggestions.sort_by_key(|s| s.distance);
    suggestions
}

/// Search across all sections of an IDL spec for names matching the query.
///
/// Performs case-insensitive substring matching against instruction names,
/// account names, type names, error names, event names, and constant names.
pub fn search_idl(idl: &IdlSpec, query: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let q = query.to_lowercase();

    for ix in &idl.instructions {
        if ix.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: ix.name.clone(),
                section: IdlSection::Instruction,
                match_type: MatchType::Contains,
            });
        }
    }
    for acc in &idl.accounts {
        if acc.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: acc.name.clone(),
                section: IdlSection::Account,
                match_type: MatchType::Contains,
            });
        }
    }
    for ty in &idl.types {
        if ty.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: ty.name.clone(),
                section: IdlSection::Type,
                match_type: MatchType::Contains,
            });
        }
    }
    for err in &idl.errors {
        if err.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: err.name.clone(),
                section: IdlSection::Error,
                match_type: MatchType::Contains,
            });
        }
    }
    for ev in &idl.events {
        if ev.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: ev.name.clone(),
                section: IdlSection::Event,
                match_type: MatchType::Contains,
            });
        }
    }
    for c in &idl.constants {
        if c.name.to_lowercase().contains(&q) {
            results.push(SearchResult {
                name: c.name.clone(),
                section: IdlSection::Constant,
                match_type: MatchType::Contains,
            });
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_suggestions() {
        let candidates = ["initialize", "close", "deposit"];
        let suggestions = suggest_similar("initlize", &candidates, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].candidate, "initialize");
    }

    #[test]
    fn test_fuzzy_case_insensitive() {
        let candidates = ["Initialize", "close"];
        let suggestions = suggest_similar("initialize", &candidates, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].candidate, "Initialize");
        assert_eq!(suggestions[0].distance, 0);
    }

    #[test]
    fn test_fuzzy_no_exact_match() {
        let candidates = ["initialize"];
        let suggestions = suggest_similar("initialize", &candidates, 3);
        assert!(suggestions.is_empty(), "exact matches should be excluded");
    }

    #[test]
    fn test_fuzzy_substring() {
        let candidates = ["swap_exact_in", "close"];
        let suggestions = suggest_similar("swap", &candidates, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].candidate, "swap_exact_in");
    }

    #[test]
    fn test_search_idl() {
        use crate::parse::parse_idl_file;
        use std::path::PathBuf;
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/meteora_dlmm.json");
        let idl = parse_idl_file(&path).expect("should parse");
        let results = search_idl(&idl, "swap");
        assert!(!results.is_empty(), "should find results for 'swap'");
    }
}
