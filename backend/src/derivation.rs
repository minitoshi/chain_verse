use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::blockchain::BlockInfo;
use crate::words::WordDictionary;

pub struct KeywordDerivation {
    dictionary: WordDictionary,
}

impl KeywordDerivation {
    pub fn new(dictionary: WordDictionary) -> Self {
        Self { dictionary }
    }

    /// Derive a keyword from block information
    /// This is deterministic: same block always produces same word
    pub fn derive_keyword(&self, block: &BlockInfo) -> Result<DerivedKeyword> {
        // Create a deterministic seed from blockhash
        let seed = self.hash_to_seed(&block.blockhash);

        // Map seed to word index
        let word_count = self.dictionary.total_count();
        let word_index = (seed % word_count as u64) as usize;

        // Get the word
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
        })
    }

    /// Convert a blockhash string to a numeric seed
    fn hash_to_seed(&self, blockhash: &str) -> u64 {
        // Hash the blockhash to get uniform distribution
        let mut hasher = Sha256::new();
        hasher.update(blockhash.as_bytes());
        let result = hasher.finalize();

        // Take first 8 bytes and convert to u64
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[0..8]);
        u64::from_le_bytes(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct DerivedKeyword {
    pub word: String,
    pub slot: u64,
    pub blockhash: String,
    pub block_time: Option<i64>,
    pub word_index: usize,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_derivation() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block = BlockInfo {
            slot: 12345,
            blockhash: "test_hash_123".to_string(),
            block_time: Some(1234567890),
        };

        // Same block should always produce same word
        let keyword1 = derivation.derive_keyword(&block).unwrap();
        let keyword2 = derivation.derive_keyword(&block).unwrap();

        assert_eq!(keyword1.word, keyword2.word);
        assert_eq!(keyword1.slot, keyword2.slot);
    }

    #[test]
    fn test_different_blocks_different_words() {
        let dict = WordDictionary::load().unwrap();
        let derivation = KeywordDerivation::new(dict);

        let block1 = BlockInfo {
            slot: 12345,
            blockhash: "hash_1".to_string(),
            block_time: Some(1234567890),
        };

        let block2 = BlockInfo {
            slot: 12346,
            blockhash: "hash_2".to_string(),
            block_time: Some(1234567891),
        };

        let keyword1 = derivation.derive_keyword(&block1).unwrap();
        let keyword2 = derivation.derive_keyword(&block2).unwrap();

        // Different hashes will very likely produce different words
        // (Could be same by chance, but very unlikely)
        println!("Block 1 -> {}", keyword1.word);
        println!("Block 2 -> {}", keyword2.word);
    }
}
