mod api;
mod blockchain;
mod database;
mod derivation;
mod poem_generator;
mod scheduler;
mod words;

use anyhow::Result;
use database::Database;
use scheduler::KeywordCollector;
use words::WordDictionary;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔗 Chain Verse - Blockchain Poetry Generator\n");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Configuration from environment variables
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set in .env file");
    let model = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| "moonshotai/kimi-k2:free".to_string());
    let interval_minutes: u64 = std::env::var("KEYWORD_INTERVAL_MINUTES")
        .unwrap_or_else(|_| "90".to_string())
        .parse()
        .unwrap_or(90);

    // Load word dictionary
    println!("📚 Loading word dictionary...");
    let dictionary = WordDictionary::load()?;
    println!("   Loaded {} words\n", dictionary.total_count());

    // Initialize database
    println!("💾 Initializing database...");
    let db = Database::new("sqlite:chain_verse.db").await?;
    println!("   Database ready\n");

    // Create keyword collector
    let collector = KeywordCollector::new(
        dictionary,
        db,
        api_key,
        model,
        interval_minutes,
    );

    // Check command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("test");

    match mode {
        "daemon" => {
            // Run keyword collector continuously
            println!("🔄 Starting keyword collector daemon...\n");
            collector.start().await?;
        }
        "api" => {
            // Run API server only
            println!("🌐 Starting API server...\n");
            let db = Database::new("sqlite:chain_verse.db").await?;
            api::serve(db, 3000).await?;
        }
        "full" => {
            // Run both collector and API server
            println!("🚀 Starting full system (collector + API)...\n");

            // Spawn collector in background
            let collector_handle = tokio::spawn(async move {
                if let Err(e) = collector.start().await {
                    eprintln!("Collector error: {}", e);
                }
            });

            // Run API server in foreground
            let db = Database::new("sqlite:chain_verse.db").await?;
            let api_handle = tokio::spawn(async move {
                if let Err(e) = api::serve(db, 3000).await {
                    eprintln!("API error: {}", e);
                }
            });

            // Wait for both
            tokio::try_join!(collector_handle, api_handle)?;
        }
        _ => {
            // Run once for testing
            println!("🧪 Running in test mode (collecting one keyword)...\n");
            collector.run_once().await?;
            println!("\n✅ Test complete!");
            println!("\n💡 Available modes:");
            println!("   cargo run           - Test mode (collect one keyword)");
            println!("   cargo run -- daemon - Run keyword collector continuously");
            println!("   cargo run -- api    - Run API server only");
            println!("   cargo run -- full   - Run collector + API server");
        }
    }

    Ok(())
}
