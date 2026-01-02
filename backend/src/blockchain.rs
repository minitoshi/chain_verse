use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub slot: u64,
    pub blockhash: String,
    pub block_time: Option<i64>,
}

pub struct SolanaClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl SolanaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url: SOLANA_RPC_URL.to_string(),
        }
    }

    /// Get the current slot number
    pub async fn get_current_slot(&self) -> Result<u64> {
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getSlot"
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let slot = response["result"]
            .as_u64()
            .context("Failed to parse slot number")?;

        Ok(slot)
    }

    /// Get block information for a specific slot
    pub async fn get_block(&self, slot: u64) -> Result<BlockInfo> {
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getBlock",
                "params": [
                    slot,
                    {
                        "encoding": "json",
                        "maxSupportedTransactionVersion": 0,
                        "transactionDetails": "none",
                        "rewards": false
                    }
                ]
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let result = &response["result"];

        if result.is_null() {
            anyhow::bail!("Block not found for slot {}", slot);
        }

        let blockhash = result["blockhash"]
            .as_str()
            .context("Failed to get blockhash")?
            .to_string();

        let block_time = result["blockTime"].as_i64();

        Ok(BlockInfo {
            slot,
            blockhash,
            block_time,
        })
    }

    /// Get the most recent confirmed block
    pub async fn get_latest_block(&self) -> Result<BlockInfo> {
        let slot = self.get_current_slot().await?;
        // Go back a few slots to ensure the block is confirmed
        let confirmed_slot = slot.saturating_sub(5);
        self.get_block(confirmed_slot).await
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
    }

    #[tokio::test]
    async fn test_get_latest_block() {
        let client = SolanaClient::new();
        let block = client.get_latest_block().await.unwrap();
        assert!(!block.blockhash.is_empty());
        assert!(block.slot > 0);
    }
}
