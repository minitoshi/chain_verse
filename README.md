# Chain Verse

A blockchain poetry generator that creates daily poems from Solana blockchain data.

## Concept

- Every ~90 minutes, a keyword is derived from Solana blockchain data
- Throughout the day, 15-20 keywords are collected
- An AI generates a poem using these keywords
- Each day's poem is unique and verifiable through blockchain data

## How It Works

1. **Blockchain Derivation**: Fetches Solana blocks and deterministically maps block hashes to words from a curated dictionary of 1,290 poetic words
2. **Keyword Collection**: A scheduler runs every 90 minutes to collect one keyword
3. **Poem Generation**: When enough keywords are collected, an AI (via OpenRouter) generates a 20-30 line poem
4. **Archive**: All poems are stored with their source blockchain data for verification

     <img width="1972" height="2414" alt="frontend" src="https://github.com/user-attachments/assets/04badf08-eafb-4d86-85a4-0e4273b1a7ed" />

## Project Structure

```
chain_verse/
├── backend/          # Rust backend - keyword derivation & poem generation
│   ├── src/
│   │   ├── main.rs           # Entry point with multiple run modes
│   │   ├── blockchain.rs     # Solana RPC client
│   │   ├── derivation.rs     # Hash → word mapping
│   │   ├── words.rs          # Word dictionary (1,290 words)
│   │   ├── database.rs       # SQLite storage
│   │   ├── scheduler.rs      # Keyword collector
│   │   ├── poem_generator.rs # OpenRouter AI integration
│   │   └── api.rs            # REST API server
│   ├── words.json            # Curated word dictionary
│   └── Cargo.toml
├── frontend/         # React frontend - display poems and archive
│   ├── src/
│   │   ├── App.jsx          # Main app with "Today" and "Archive" views
│   │   └── App.css          # Styling
│   └── package.json
└── README.md
```

## Tech Stack

- **Backend**: Rust (async/await with Tokio)
- **Frontend**: React + Vite
- **Database**: SQLite
- **Poem Generation**: OpenRouter API (moonshotai/kimi-k2:free)
- **Blockchain**: Solana mainnet (public RPC)
- **API**: Axum web framework with CORS

## Setup & Running

### Prerequisites
- Rust
- Node.js
- OpenRouter API key (get one free at https://openrouter.ai)

### Environment Setup

**IMPORTANT:** Create a `.env` file in the `backend/` directory:

```bash
cd backend
cp .env.example .env
# Edit .env and add your OpenRouter API key
```

Your `.env` file should look like:
```
OPENROUTER_API_KEY=your_api_key_here
OPENROUTER_MODEL=moonshotai/kimi-k2:free
KEYWORD_INTERVAL_MINUTES=90
```

### Backend

The backend has multiple run modes:

```bash
cd backend

# Test mode - collect one keyword and exit
cargo run

# API server only
cargo run -- api

# Keyword collector daemon only (runs every 90 minutes)
cargo run -- daemon

# Full system - both collector and API server
cargo run -- full
```

**Recommended for development:**
```bash
# Terminal 1: Run API server
cd backend
cargo run -- api

# Terminal 2: Test keyword collection
cd backend
cargo run  # Runs once for testing
```

### Frontend

```bash
cd frontend
npm install  # Already done ✅
npm run dev  # Starts on http://localhost:5173
```

## API Endpoints

- `GET /api/poems/today` - Today's poem status (in-progress or complete)
- `GET /api/poems` - All poems (latest first)
- `GET /api/poems/{date}` - Specific poem by date (YYYY-MM-DD)
- `GET /api/keywords/today` - Keywords collected today

## Current Status

✅ **Complete!** All core features implemented:
- ✅ Blockchain keyword derivation
- ✅ SQLite database storage
- ✅ Poem generation with OpenRouter
- ✅ 90-minute keyword collector
- ✅ REST API server
- ✅ React frontend with Today/Archive views

## What's Working

**Backend:**
- Deriving words from Solana blockchain ✅
- Storing keywords in database ✅
- Poem generation via OpenRouter ✅ (rate limits may apply on free tier)
- Keyword collector scheduler ✅
- REST API serving poems and keywords ✅

**Frontend:**
- "Today" view showing in-progress poems ✅
- Progress bar showing keyword collection ✅
- Keywords displayed with blockchain slot info ✅
- "Archive" view for past poems ✅
- Clean, dark-themed UI ✅

## Database

SQLite database (`chain_verse.db`) contains:
- **keywords** table: word, slot, blockhash, block_time, word_index
- **poems** table: date, title, content, keyword_ids

## Next Steps (Future Enhancements)

- Better error handling for rate-limited API calls
- Retry logic for poem generation
- Custom UI design (current UI is functional but basic)
- Deployment configuration
- Analytics/stats page
- Social sharing features
- On-chain poem storage (optional)

## Notes

- Free OpenRouter models may have rate limits; wait and retry if needed
- Keyword collection requires the daemon to be running
- Each day needs 15+ keywords before a poem is generated
- All blockchain data is verifiable and deterministic

---

**Chain Verse** - Where blockchain meets poetry 🔗✨
