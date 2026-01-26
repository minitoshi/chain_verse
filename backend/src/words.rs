use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordDictionary {
    pub nouns: Vec<String>,
    pub verbs: Vec<String>,
    pub adjectives: Vec<String>,
}

impl WordDictionary {
    /// Load the word dictionary from the JSON file
    pub fn load() -> Result<Self> {
        let content = fs::read_to_string("words.json")?;
        let dict: WordDictionary = serde_json::from_str(&content)?;
        Ok(dict)
    }

    /// Get all words as a single flat list
    pub fn all_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        words.extend(self.nouns.clone());
        words.extend(self.verbs.clone());
        words.extend(self.adjectives.clone());
        words
    }

    /// Get total word count
    pub fn total_count(&self) -> usize {
        self.nouns.len() + self.verbs.len() + self.adjectives.len()
    }

    /// Get a word by index
    pub fn get_word(&self, index: usize) -> Option<String> {
        let all = self.all_words();
        all.get(index).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_dictionary() {
        let dict = WordDictionary::load().unwrap();
        assert!(dict.total_count() > 0);
        assert!(!dict.nouns.is_empty());
        assert!(!dict.verbs.is_empty());
        assert!(!dict.adjectives.is_empty());
    }
}
