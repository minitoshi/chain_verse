use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{TransactionDetails, UiTransactionEncoding};
use std::sync::Arc;

use crate::consts::{CONFIRMATION_SLOTS, MAINNET_RPC_URL};

/// Rich block information from Solana
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub slot: u64,
    pub blockhash: String,
    pub previous_blockhash: String,
    pub block_time: Option<i64>,
    pub block_height: Option<u64>,
    pub parent_slot: u64,
    pub transaction_count: usize,
    /// Sample transaction signatures for additional entropy
    pub sample_signatures: Vec<String>,
}

impl BlockInfo {
    /// Get multiple entropy sources from the block
    pub fn entropy_sources(&self) -> Vec<String> {
        let mut sources = vec![
            self.blockhash.clone(),
            self.previous_blockhash.clone(),
            format!("{}", self.slot),
            format!("{}", self.transaction_count),
        ];

        // Add sample signatures
        sources.extend(self.sample_signatures.iter().cloned());

        sources
    }
}

/// Solana blockchain client using official SDK
/// Uses Arc to allow sharing across async tasks
pub struct SolanaClient {
    client: Arc<RpcClient>,
    rpc_url: String,
}

impl SolanaClient {
    /// Create a new client with default mainnet RPC
    pub fn new() -> Self {
        Self::with_url(MAINNET_RPC_URL)
    }

    /// Create a new client with custom RPC URL
    pub fn with_url(url: &str) -> Self {
        let client = RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        );
        Self {
            client: Arc::new(client),
            rpc_url: url.to_string(),
        }
    }

    /// Get the RPC URL being used
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get the current slot number (async wrapper)
    pub async fn get_current_slot(&self) -> Result<u64> {
        let client = Arc::clone(&self.client);
        tokio::task::spawn_blocking(move || {
            client.get_slot().context("Failed to get current slot")
        })
        .await?
    }

    /// Get the current epoch info (async wrapper)
    pub async fn get_epoch_info(&self) -> Result<solana_sdk::epoch_info::EpochInfo> {
        let client = Arc::clone(&self.client);
        tokio::task::spawn_blocking(move || {
            client.get_epoch_info().context("Failed to get epoch info")
        })
        .await?
    }

    /// Get rich block information for a specific slot (async wrapper)
    pub async fn get_block(&self, slot: u64) -> Result<BlockInfo> {
        let client = Arc::clone(&self.client);
        tokio::task::spawn_blocking(move || {
            Self::get_block_sync(&client, slot)
        })
        .await?
    }

    /// Synchronous block fetch (internal)
    fn get_block_sync(client: &RpcClient, slot: u64) -> Result<BlockInfo> {
        let config = RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            transaction_details: Some(TransactionDetails::Signatures),
            rewards: Some(false),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let block = client
            .get_block_with_config(slot, config)
            .context(format!("Failed to get block for slot {}", slot))?;

        // Extract sample signatures (up to 5 for entropy)
        let sample_signatures: Vec<String> = block
            .signatures
            .clone()
            .unwrap_or_default()
            .into_iter()
            .take(5)
            .collect();

        let transaction_count = block.signatures.as_ref().map(|s| s.len()).unwrap_or(0);

        Ok(BlockInfo {
            slot,
            blockhash: block.blockhash,
            previous_blockhash: block.previous_blockhash,
            block_time: block.block_time,
            block_height: block.block_height,
            parent_slot: block.parent_slot,
            transaction_count,
            sample_signatures,
        })
    }

    /// Get the most recent confirmed block (async wrapper)
    pub async fn get_latest_block(&self) -> Result<BlockInfo> {
        let slot = self.get_current_slot().await?;
        // Go back to ensure the block is confirmed and available
        let confirmed_slot = slot.saturating_sub(CONFIRMATION_SLOTS);
        self.get_block(confirmed_slot).await
    }

    /// Get multiple blocks for richer data (async wrapper)
    pub async fn get_recent_blocks(&self, count: usize) -> Result<Vec<BlockInfo>> {
        let current_slot = self.get_current_slot().await?;
        let client = Arc::clone(&self.client);

        tokio::task::spawn_blocking(move || {
            let mut blocks = Vec::with_capacity(count);
            let interval = 100; // ~40 seconds apart

            for i in 0..count {
                let target_slot = current_slot.saturating_sub(CONFIRMATION_SLOTS + (i as u64 * interval));
                match Self::get_block_sync(&client, target_slot) {
                    Ok(block) => blocks.push(block),
                    Err(e) => {
                        eprintln!("Slot {} unavailable: {}, trying nearby", target_slot, e);
                        for offset in 1..=5 {
                            if let Ok(block) = Self::get_block_sync(&client, target_slot.saturating_sub(offset)) {
                                blocks.push(block);
                                break;
                            }
                        }
                    }
                }
            }

            Ok(blocks)
        })
        .await?
    }

    /// Check if the RPC connection is healthy (async wrapper)
    pub async fn health_check(&self) -> Result<bool> {
        let client = Arc::clone(&self.client);
        tokio::task::spawn_blocking(move || {
            match client.get_health() {
                Ok(_) => Ok(true),
                Err(e) => {
                    eprintln!("RPC health check failed: {}", e);
                    Ok(false)
                }
            }
        })
        .await?
    }

    /// Get the current block production rate (slots per second) (async wrapper)
    pub async fn get_block_production_rate(&self) -> Result<f64> {
        let client = Arc::clone(&self.client);
        tokio::task::spawn_blocking(move || {
            let samples = client
                .get_recent_performance_samples(Some(1))
                .context("Failed to get performance samples")?;

            if let Some(sample) = samples.first() {
                let slots_per_second = sample.num_slots as f64 / sample.sample_period_secs as f64;
                Ok(slots_per_second)
            } else {
                Ok(2.0)
            }
        })
        .await?
    }
}

impl Default for SolanaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_current_slot() {
        let client = SolanaClient::new();
        let slot = client.get_current_slot().await.unwrap();
        assert!(slot > 0);
        println!("Current slot: {}", slot);
    }

    #[tokio::test]
    async fn test_get_latest_block() {
        let client = SolanaClient::new();
        let block = client.get_latest_block().await.unwrap();
        assert!(!block.blockhash.is_empty());
        assert!(block.slot > 0);
        println!("Latest block: {:?}", block);
    }

    #[test]
    fn test_entropy_sources() {
        let block = BlockInfo {
            slot: 12345,
            blockhash: "abc123".to_string(),
            previous_blockhash: "xyz789".to_string(),
            block_time: Some(1234567890),
            block_height: Some(1000),
            parent_slot: 12344,
            transaction_count: 50,
            sample_signatures: vec!["sig1".to_string(), "sig2".to_string()],
        };

        let sources = block.entropy_sources();
        assert!(sources.len() >= 4);
        assert!(sources.contains(&"abc123".to_string()));
        assert!(sources.contains(&"xyz789".to_string()));
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = SolanaClient::new();
        let healthy = client.health_check().await.unwrap();
        println!("RPC healthy: {}", healthy);
    }
}
