use anyhow::Result;
use chain_verse::blockchain::SolanaClient;
use chain_verse::database::Database;
use chain_verse::derivation::KeywordDerivation;
use chain_verse::poem_generator::PoemGenerator;
use chain_verse::words::WordDictionary;
use chrono::{NaiveDate, Duration, Utc};

const SLOTS_PER_DAY: u64 = 216_000; // ~2.5 slots/second * 86400 seconds
const KEYWORDS_PER_DAY: usize = 12; // Collect 12 keywords per day for good poems

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔗 Chain Verse - Historical Backfill\n");

    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();

    let (start_date, end_date) = if args.len() >= 3 {
        (args[1].clone(), args[2].clone())
    } else if args.len() == 2 {
        // Single date - backfill just that day
        (args[1].clone(), args[1].clone())
    } else {
        // Default: from Jan 1, 2026 to today
        let today = Utc::now().format("%Y-%m-%d").to_string();
        ("2026-01-01".to_string(), today)
    };

    println!("📅 Backfilling from {} to {}\n", start_date, end_date);

    // Initialize components
    let db = Database::new("sqlite:chain_verse.db").await?;
    let dictionary = WordDictionary::load()?;
    let derivation = KeywordDerivation::new(dictionary);
    let solana = SolanaClient::new();

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set in .env file");
    let model = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| "meta-llama/llama-3.2-3b-instruct:free".to_string());
    let generator = PoemGenerator::new(api_key, model);

    // Get current slot as reference point
    let current_slot = solana.get_current_slot().await?;
    let now = Utc::now();
    println!("📍 Current slot: {} ({})\n", current_slot, now.format("%Y-%m-%d %H:%M UTC"));

    // Parse dates
    let start = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")?;
    let end = NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")?;

    let mut current = start;
    let mut days_processed = 0;
    let mut poems_generated = 0;

    while current <= end {
        let date_str = current.format("%Y-%m-%d").to_string();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📆 Processing: {}", date_str);

        // Check if we already have a poem for this date
        if let Some(existing) = db.get_poem_by_date(&date_str).await? {
            println!("   ✅ Poem already exists, skipping");
            println!("   Preview: {}...", &existing.content.chars().take(50).collect::<String>());
            current = current + Duration::days(1);
            days_processed += 1;
            continue;
        }

        // Get existing keywords for this date
        let existing_keywords = db.get_keywords_for_date(&date_str).await?;
        let keywords_needed = KEYWORDS_PER_DAY.saturating_sub(existing_keywords.len());

        println!("   Existing keywords: {}", existing_keywords.len());

        if keywords_needed > 0 {
            println!("   Collecting {} more keywords...", keywords_needed);

            // Calculate slot range for this date
            let days_ago = (now.date_naive() - current).num_days();
            let base_slot = current_slot.saturating_sub((days_ago as u64) * SLOTS_PER_DAY);

            // Collect keywords spread throughout the day
            let slot_interval = SLOTS_PER_DAY / (keywords_needed as u64 + 1);
            let mut collected = 0;

            for i in 0..keywords_needed {
                let target_slot = base_slot + (i as u64 * slot_interval);

                // Try to get a block at this slot (with retry for nearby slots)
                for offset in 0..50 {
                    let try_slot = target_slot.saturating_sub(offset);
                    match solana.get_block(try_slot).await {
                        Ok(block) => {
                            let keyword = derivation.derive_keyword(&block)?;

                            // Store with the target date
                            match db.insert_keyword_with_date(&keyword, &date_str).await {
                                Ok(_) => {
                                    println!("   + \"{}\" (slot {})", keyword.word, keyword.slot);
                                    collected += 1;
                                }
                                Err(e) => {
                                    // Probably duplicate slot, skip
                                    if !e.to_string().contains("UNIQUE") {
                                        eprintln!("   Error storing keyword: {}", e);
                                    }
                                }
                            }
                            break;
                        }
                        Err(_) => continue, // Try next slot
                    }
                }

                // Small delay to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            println!("   Collected {} new keywords", collected);
        }

        // Get all keywords for this date (existing + new)
        let all_keywords = db.get_keywords_for_date(&date_str).await?;
        println!("   Total keywords: {}", all_keywords.len());

        if all_keywords.len() >= 8 {
            println!("   Generating poem...");

            let keyword_strings: Vec<String> = all_keywords.iter().map(|k| k.word.clone()).collect();
            println!("   Words: {}", keyword_strings.join(", "));

            match generator.generate_poem(&keyword_strings).await {
                Ok(poem) => {
                    let keyword_ids: Vec<i64> = all_keywords.iter().map(|k| k.id).collect();
                    db.insert_poem(&date_str, None, &poem, &keyword_ids).await?;
                    println!("   ✅ Poem generated!");
                    poems_generated += 1;
                }
                Err(e) => {
                    eprintln!("   ❌ Failed to generate poem: {}", e);
                    // Wait a bit before next attempt (rate limiting)
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        } else {
            println!("   ⚠️  Not enough keywords for poem (need 8, have {})", all_keywords.len());
        }

        current = current + Duration::days(1);
        days_processed += 1;

        // Delay between days to avoid overwhelming APIs
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Backfill complete!");
    println!("   Days processed: {}", days_processed);
    println!("   Poems generated: {}", poems_generated);

    Ok(())
}
