-- Keywords table: stores words derived from blockchain
CREATE TABLE IF NOT EXISTS keywords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,
    slot INTEGER NOT NULL UNIQUE,
    blockhash TEXT NOT NULL,
    block_time INTEGER,
    word_index INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_keywords_created_at ON keywords(created_at);
CREATE INDEX IF NOT EXISTS idx_keywords_slot ON keywords(slot);

-- Poems table: stores daily generated poems
CREATE TABLE IF NOT EXISTS poems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,  -- Format: YYYY-MM-DD
    title TEXT,
    content TEXT NOT NULL,
    keyword_ids TEXT NOT NULL,  -- JSON array of keyword IDs
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_poems_date ON poems(date);
