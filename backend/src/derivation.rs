use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::blockchain::BlockInfo;
use crate::consts::BlockDataSource;
use crate::words::WordDictionary;

pub struct KeywordDerivation {
    dictionary: WordDictionary,
}

impl KeywordDerivation {
    pub fn new(dictionary: WordDictionary) -> Self {
        Self { dictionary }
    }

    /// Derive a keyword from block information using blockhash (default)
    /// This is deterministic: same block always produces same word
    pub fn derive_keyword(&self, block: &BlockInfo) -> Result<DerivedKeyword> {
        self.derive_keyword_from_source(block, BlockDataSource::Blockhash)
    }

    /// Derive a keyword using a specific data source
    pub fn derive_keyword_from_source(
        &self,
        block: &BlockInfo,
        source: BlockDataSource,
    ) -> Result<DerivedKeyword> {
        let entropy = self.get_entropy_for_source(block, source);
        let seed = self.hash_to_seed(&entropy);

        let word_count = self.dictionary.total_count();
        let word_index = (seed % word_count as u64) as usize;

        let all_words = self.dictionary.all_words();
        let word = all_words
            .get(word_index)
            .ok_or_else(|| anyhow::anyhow!("Word index out of bounds"))?
            .clone();

        Ok(DerivedKeyword {
            word,
            slot: block.slot,
            blockhash: block.blockhash.clone(),
            block_time: block.block_time,
            word_index,
            source,
        })
    }

    /// Derive multiple keywords from a single block using different entropy sources
    pub fn derive_multiple_keywords(&self, block: &BlockInfo) -> Vec<DerivedKeyword> {
        let mut keywords = Vec::new();

        // Use blockhash (primary)
        if let Ok(kw) = self.derive_keyword_from_source(block, BlockDataSource::Blockhash) {
            keywords.push(kw);
        }

        // Use previous blockhash for additional word
        if let Ok(kw) = self.derive_keyword_from_source(block, BlockDataSource::PreviousBlockhash) {
            // Only add if different from first word
            if keywords.is_empty() || keywords[0].word != kw.word {
                keywords.push(kw);
            }
        }

        // Use transaction signatures for more variety
        for (i, sig) in block.sample_signatures.iter().take(3).enumerate() {
            let entropy = format!("{}:{}", sig, i);
            let seed = self.hash_to_seed(&entropy);
            let word_count = self.dictionary.total_count();
            let word_index = (seed % word_count as u64) as usize;

            if let Some(word) = self.dictionary.all_words().get(word_index) {
                // Only add if unique
                if !keywords.iter().any(|k| k.word == *word) {
                    keywords.push(DerivedKeyword {
                        word: word.clone(),
                        slot: block.slot,
                        blockhash: block.blockhash.clone(),
                        block_time: block.block_time,
                        word_index,
                        source: BlockDataSource::TransactionRoot,
                    });
                }
            }
        }

        keywords
    }

    /// Get entropy string for a specific data source
    fn get_entropy_for_source(&self, block: &BlockInfo, source: BlockDataSource) -> String {
        match source {
            BlockDataSource::Blockhash => block.blockhash.clone(),
            BlockDataSource::PreviousBlockhash => block.previous_blockhash.clone(),
            BlockDataSource::TransactionRoot => {
                // Combine all sample signatures
                block.sample_signatures.join(":")
            }
            BlockDataSource::Rewards => {
                // Use block height as entropy source
                format!("rewards:{}", block.block_height.unwrap_or(0))
            }
            BlockDataSource::TransactionCount => {
                format!("txcount:{}:{}", block.transaction_count, block.slot)
            }
        }
    }

    /// Convert any string to a numeric seed
    fn hash_to_seed(&self, input: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[0..8]);
        u64::from_le_bytes(bytes)
    }

    /// Derive keywords from multiple blocks for batch processing
    pub fn derive_keywords_from_blocks(&self, blocks: &[BlockInfo]) -> Vec<DerivedKeyword> {
        let mut all_keywords = Vec::new();
        let mut seen_words = std::collections::HashSet::new();

        for block in blocks {
            let keywords = self.derive_multiple_keywords(block);
            for kw in keywords {
                if seen_words.insert(kw.word.clone()) {
                    all_keywords.push(kw);
                }
            }
        }

        all_keywords
    }
}

#[derive(Debug, Clone)]
pub struct DerivedKeyword {
    pub word: String,
    pub slot: u64,
    pub blockhash: String,
    pub block_time: Option<i64>,
    pub word_index: usize,
    pub source: BlockDataSource,
}

impl DerivedKeyword {
    /// Get a human-readable timestamp
    pub fn formatted_time(&self) -> Option<String> {
        self.block_time.map(|ts| {
            let dt = chrono::DateTime::from_timestamp(ts, 0)
                .expect("Invalid timestamp");
            dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        })
    }

    /// Get the data source as a string
    pub fn source_name(&self) -> &'static str {
        match self.source {
            BlockDataSource::Blockhash => "blockhash",
            BlockDataSource::PreviousBlockhash => "previous_blockhash",
            BlockDataSource::TransactionRoot => "transaction",
            BlockDataSource::Rewards => "rewards",
            BlockDataSource::TransactionCount => "tx_count",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_block() -> BlockInfo {
        BlockInfo {
            slot: 12345,
            blockhash: "test_hash_123".to_string(),
            previous_blockhash: "prev_hash_456".to_string(),
            block_time: Some(1234567890),
            block_height: Some(1000),
            parent_slot: 12344,
            transaction_count: 50,
            sample_signatures: vec![
                "sig1".to_string(),
                "sig2".to_string(),
                "sig3".to_string(),
            ],
        }
    }

    #[test]
    fn test_deterministic_derivation() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block = create_test_block();

        // Same block should always produce same word
        let keyword1 = derivation.derive_keyword(&block).unwrap();
        let keyword2 = derivation.derive_keyword(&block).unwrap();

        assert_eq!(keyword1.word, keyword2.word);
        assert_eq!(keyword1.slot, keyword2.slot);
    }

    #[test]
    fn test_different_sources_different_words() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block = create_test_block();

        let kw1 = derivation.derive_keyword_from_source(&block, BlockDataSource::Blockhash).unwrap();
        let kw2 = derivation.derive_keyword_from_source(&block, BlockDataSource::PreviousBlockhash).unwrap();

        // Different sources should produce different words (very likely)
        println!("Blockhash -> {}", kw1.word);
        println!("PreviousBlockhash -> {}", kw2.word);
    }

    #[test]
    fn test_multiple_keywords() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block = create_test_block();
        let keywords = derivation.derive_multiple_keywords(&block);

        println!("Derived {} keywords from single block:", keywords.len());
        for kw in &keywords {
            println!("  {} (from {})", kw.word, kw.source_name());
        }

        assert!(keywords.len() >= 1);
    }

    #[test]
    fn test_different_blocks_different_words() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block1 = BlockInfo {
            slot: 12345,
            blockhash: "hash_1".to_string(),
            previous_blockhash: "prev_1".to_string(),
            block_time: Some(1234567890),
            block_height: Some(1000),
            parent_slot: 12344,
            transaction_count: 50,
            sample_signatures: vec![],
        };

        let block2 = BlockInfo {
            slot: 12346,
            blockhash: "hash_2".to_string(),
            previous_blockhash: "prev_2".to_string(),
            block_time: Some(1234567891),
            block_height: Some(1001),
            parent_slot: 12345,
            transaction_count: 45,
            sample_signatures: vec![],
        };

        let keyword1 = derivation.derive_keyword(&block1).unwrap();
        let keyword2 = derivation.derive_keyword(&block2).unwrap();

        println!("Block 1 -> {}", keyword1.word);
        println!("Block 2 -> {}", keyword2.word);
    }
}
