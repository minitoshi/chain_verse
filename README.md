# Chain Verse

A blockchain poetry generator that creates daily poems from Solana blockchain data and posts them to Bluesky.

## Concept

- Every day, keywords are derived from Solana blockchain data
- Keywords come from the **BIP-39 wordlist** (2,048 words used in cryptocurrency seed phrases)
- An AI generates a poem using these blockchain-derived keywords
- The poem is automatically posted to Bluesky as a thread with an image
- Keywords are verifiable - anyone can derive the same words from the blockchain data

## How It Works

1. **Blockchain Data**: Fetches 12 Solana blocks spread across the past 24 hours
2. **BIP-39 Derivation**: Block hashes are SHA-256 hashed and mapped to words from the BIP-39 wordlist (the same 2,048 words used for cryptocurrency wallet seed phrases)
3. **Keyword Extraction**: Multiple entropy sources per block (blockhash, previousBlockhash, transaction signatures) yield 15-16 unique keywords
4. **Poem Generation**: An AI (via OpenRouter) generates a 20-30 line poem incorporating the keywords
5. **Bluesky Posting**: The poem is split into a thread (to fit 300-char limit) and posted with a random image
6. **Archival**: Poem data is saved for the static website

## Why BIP-39?

The [BIP-39 wordlist](https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt) contains 2,048 carefully selected English words used for generating cryptocurrency wallet seed phrases. These words are:

- **Unambiguous**: No similar-looking words (e.g., no "man" and "men")
- **Common**: Recognizable everyday English words
- **Diverse**: Covers nouns, verbs, adjectives across many themes
- **Poetic potential**: Words like "voyage", "crystal", "thunder", "whisper"

By deriving keywords from this list using blockchain entropy, each poem is cryptographically tied to real Solana blockchain state.

## Automation

The entire process runs automatically via **GitHub Actions**:

- **Schedule**: Runs daily at 12:00 UTC
- **Manual trigger**: Can also be triggered manually from GitHub Actions
- **Zero maintenance**: Once set up, poems are generated and posted automatically

## Project Structure

```
chain_verse/
├── scripts/              # Daily poem generation
│   ├── daily-poem.js     # Main script - fetches blocks, generates poem, posts to Bluesky
│   └── package.json
├── poem-images/          # Images for Bluesky posts (randomly selected each day)
├── backend/              # Rust backend (for local development/API)
│   ├── src/
│   └── words.json        # BIP-39 wordlist (2,048 words)
├── frontend/             # React frontend - display poems and archive
│   ├── src/
│   └── public/data/      # Generated poem data (today.json, archive.json)
├── .github/workflows/    # GitHub Actions automation
│   └── daily-poem.yml
└── README.md
```

## Tech Stack

- **Daily Script**: Node.js
- **Poem Generation**: OpenRouter API (with fallback models)
- **Social Posting**: Bluesky (AT Protocol)
- **Blockchain**: Solana mainnet (public RPC)
- **Automation**: GitHub Actions
- **Frontend**: React + Vite (static site)

## Setup

### Prerequisites

- Node.js 18+
- GitHub repository with Actions enabled
- OpenRouter API key
- Bluesky account with app password

### GitHub Secrets

Add these secrets to your repository (Settings > Secrets > Actions):

```
OPENROUTER_API_KEY=your_openrouter_api_key
BLUESKY_HANDLE=your.handle.bsky.social
BLUESKY_APP_PASSWORD=your_app_password
WEBSITE_URL=https://your-website.com (optional)
```

### Local Development

```bash
# Install dependencies
cd scripts
npm install

# Run locally (requires .env file with secrets)
node daily-poem.js
```

### Adding Images

Add images to the `poem-images/` folder:
- Supported formats: `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`
- A random image is selected each day (different from the previous day)
- Images are attached to the first post of the Bluesky thread

## Bluesky Output

Each day's poem is posted as a thread:
- **First post**: Opening lines + image + thread indicator
- **Middle posts**: Continuation of the poem
- **Last post**: Closing lines + link to website

## Derivation Example

```
Block hash: 5xYz...abc
    ↓ SHA-256
Hash bytes: [142, 67, ...]
    ↓ First 8 bytes as uint64
Seed: 9847362510283
    ↓ mod 2048
Word index: 1847
    ↓ BIP-39 lookup
Keyword: "voyage"
```

## Keyword Verification

Anyone can verify that the keywords came from real blockchain data:
1. Get the block hash for a given slot from Solana
2. SHA-256 hash it
3. Take first 8 bytes as little-endian uint64
4. Modulo 2048 gives the BIP-39 word index

## API Endpoints (Local Backend)

- `GET /api/poems/today` - Today's poem status
- `GET /api/poems` - All poems (latest first)
- `GET /api/poems/{date}` - Specific poem by date
- `GET /api/keywords/today` - Keywords collected today

## Links

- **Bluesky**: [@chainverse.bsky.social](https://bsky.app/profile/chainverse.bsky.social)
- **Website**: Coming soon

---

**Chain Verse** - Where blockchain meets poetry
