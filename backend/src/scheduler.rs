use anyhow::Result;
use std::time::Duration;
use tokio::time;

use crate::blockchain::SolanaClient;
use crate::consts::MIN_KEYWORDS_FOR_POEM;
use crate::database::Database;
use crate::derivation::KeywordDerivation;
use crate::poem_generator::PoemGenerator;
use crate::words::WordDictionary;

pub struct KeywordCollector {
    solana_client: SolanaClient,
    derivation: KeywordDerivation,
    database: Database,
    poem_generator: PoemGenerator,
    interval_minutes: u64,
}

impl KeywordCollector {
    pub fn new(
        dictionary: WordDictionary,
        database: Database,
        api_key: String,
        model: String,
        interval_minutes: u64,
    ) -> Self {
        Self {
            solana_client: SolanaClient::new(),
            derivation: KeywordDerivation::new(dictionary),
            database,
            poem_generator: PoemGenerator::new(api_key, model),
            interval_minutes,
        }
    }

    /// Start the keyword collection loop
    pub async fn start(&self) -> Result<()> {
        println!("🚀 Starting keyword collector...");
        println!("   Collecting keywords every {} minutes\n", self.interval_minutes);

        let mut interval = time::interval(Duration::from_secs(self.interval_minutes * 60));

        loop {
            interval.tick().await;

            match self.collect_keyword().await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("❌ Error collecting keyword: {}", e);
                }
            }

            // Check if we should generate today's poem
            match self.maybe_generate_daily_poem().await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("❌ Error generating daily poem: {}", e);
                }
            }
        }
    }

    /// Collect a single keyword from the blockchain
    async fn collect_keyword(&self) -> Result<()> {
        println!("🔗 Fetching latest block from Solana...");

        // Fetch block with retry
        let block = match self.solana_client.get_latest_block().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("❌ Failed to fetch block from Solana: {}", e);
                eprintln!("   Will retry on next interval");
                anyhow::bail!("Solana RPC error: {}", e);
            }
        };

        // Derive keyword (this should not fail unless word dictionary is corrupted)
        let keyword = self.derivation.derive_keyword(&block)?;

        println!("   Derived keyword: \"{}\" from slot {}", keyword.word, keyword.slot);

        // Store in database with error handling
        match self.database.insert_keyword(&keyword).await {
            Ok(_) => {
                println!("   ✅ Keyword stored\n");
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Failed to store keyword in database: {}", e);
                eprintln!("   Keyword: {} (slot: {})", keyword.word, keyword.slot);
                anyhow::bail!("Database error: {}", e);
            }
        }
    }

    /// Check if we should generate today's poem and do it if needed
    async fn maybe_generate_daily_poem(&self) -> Result<()> {
        let today = Database::today();

        // Check if we already have a poem for today
        if let Some(_) = self.database.get_poem_by_date(&today).await? {
            return Ok(()); // Already have today's poem
        }

        // Get today's keywords
        let keywords = self.database.get_keywords_for_date(&today).await?;

        // Need minimum keywords to generate a poem
        if keywords.len() < MIN_KEYWORDS_FOR_POEM {
            return Ok(()); // Not enough keywords yet
        }

        println!("🎨 Generating poem for {}...", today);
        println!("   Using {} keywords", keywords.len());

        let keyword_strings: Vec<String> = keywords.iter().map(|k| k.word.clone()).collect();

        match self.poem_generator.generate_poem(&keyword_strings).await {
            Ok(poem) => {
                let keyword_ids: Vec<i64> = keywords.iter().map(|k| k.id).collect();

                self.database
                    .insert_poem(&today, None, &poem, &keyword_ids)
                    .await?;

                println!("   ✅ Poem generated and stored!");
                println!("\n✨ POEM OF THE DAY: {} ✨", today);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("{}", poem);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            }
            Err(e) => {
                eprintln!("   ⚠️  Failed to generate poem: {}", e);
            }
        }

        Ok(())
    }

    /// Run once to collect a keyword immediately (for testing)
    pub async fn run_once(&self) -> Result<()> {
        self.collect_keyword().await?;
        self.maybe_generate_daily_poem().await?;
        Ok(())
    }
}
