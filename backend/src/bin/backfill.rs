use anyhow::Result;
use chain_verse::database::Database;
use chain_verse::poem_generator::PoemGenerator;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run --bin backfill <date>");
        println!("Example: cargo run --bin backfill 2026-01-01");
        return Ok(());
    }

    let date = &args[1];

    println!("🎨 Generating poem for {}...\n", date);

    let db = Database::new("sqlite:chain_verse.db").await?;

    // Get keywords for this date
    let keywords = db.get_keywords_for_date(date).await?;

    if keywords.is_empty() {
        println!("❌ No keywords found for {}!", date);
        println!("💡 Tip: First update some keywords to this date in the database");
        return Ok(());
    }

    println!("Found {} keywords for {}", keywords.len(), date);

    if keywords.len() < 8 {
        println!("⚠️  Warning: Only {} keywords (recommended: 15+)", keywords.len());
        println!("The poem might be shorter or less coherent.\n");
    }

    // Check if poem already exists
    if let Some(existing) = db.get_poem_by_date(date).await? {
        println!("✅ Poem already exists for {}!", date);
        println!("\n{}\n", existing.content);
        return Ok(());
    }

    // Generate poem
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set in .env file");
    let model = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| "meta-llama/llama-3.2-3b-instruct:free".to_string());

    let generator = PoemGenerator::new(api_key, model);
    let keyword_strings: Vec<String> = keywords.iter().map(|k| k.word.clone()).collect();

    println!("Keywords: {}\n", keyword_strings.join(", "));
    println!("Generating poem... (this may take a moment)\n");

    match generator.generate_poem(&keyword_strings).await {
        Ok(poem) => {
            let keyword_ids: Vec<i64> = keywords.iter().map(|k| k.id).collect();
            db.insert_poem(date, None, &poem, &keyword_ids).await?;

            println!("✨ POEM FOR {} ✨", date);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("{}", poem);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            println!("✅ Poem saved!");
        }
        Err(e) => {
            println!("❌ Failed to generate poem: {}", e);
            println!("\nThis might be due to API rate limits. Try again in a few moments.");
        }
    }

    Ok(())
}
