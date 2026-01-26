/// Chain Verse Constants
/// Inspired by ORE's well-organized constants pattern

// =============================================================================
// TIME CONSTANTS (in seconds)
// =============================================================================

/// One minute in seconds
pub const ONE_MINUTE: u64 = 60;

/// One hour in seconds
pub const ONE_HOUR: u64 = 60 * ONE_MINUTE;

/// One day in seconds
pub const ONE_DAY: u64 = 24 * ONE_HOUR;

/// One week in seconds
pub const ONE_WEEK: u64 = 7 * ONE_DAY;

// =============================================================================
// SLOT CONSTANTS
// Solana produces ~2.5 slots per second (400ms per slot)
// =============================================================================

/// Slots per second (approximate)
pub const SLOTS_PER_SECOND: u64 = 2;

/// One minute in slots (~150 slots)
pub const ONE_MINUTE_SLOTS: u64 = 60 * SLOTS_PER_SECOND;

/// One hour in slots (~7,200 slots)
pub const ONE_HOUR_SLOTS: u64 = 60 * ONE_MINUTE_SLOTS;

/// One day in slots (~172,800 slots)
pub const ONE_DAY_SLOTS: u64 = 24 * ONE_HOUR_SLOTS;

/// One week in slots (~1,209,600 slots)
pub const ONE_WEEK_SLOTS: u64 = 7 * ONE_DAY_SLOTS;

// =============================================================================
// RPC ENDPOINTS
// =============================================================================

/// Solana Mainnet RPC URL
pub const MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Solana Devnet RPC URL (for testing)
pub const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";

/// Solana Testnet RPC URL
pub const TESTNET_RPC_URL: &str = "https://api.testnet.solana.com";

// =============================================================================
// CHAIN VERSE CONFIGURATION
// =============================================================================

/// Default number of keywords needed to generate a poem
pub const DEFAULT_KEYWORDS_PER_POEM: usize = 16;

/// Minimum keywords required before poem generation
pub const MIN_KEYWORDS_FOR_POEM: usize = 8;

/// Maximum keywords to use in a single poem
pub const MAX_KEYWORDS_FOR_POEM: usize = 24;

/// Default keyword collection interval in minutes
pub const DEFAULT_COLLECTION_INTERVAL_MINUTES: u64 = 90;

/// Number of slots to go back for confirmed blocks
pub const CONFIRMATION_SLOTS: u64 = 32;

/// Default poem line count range
pub const POEM_MIN_LINES: usize = 20;
pub const POEM_MAX_LINES: usize = 30;

// =============================================================================
// DATABASE
// =============================================================================

/// Default SQLite database path
pub const DEFAULT_DATABASE_URL: &str = "sqlite:chain_verse.db";

// =============================================================================
// API SERVER
// =============================================================================

/// Default API server port
pub const DEFAULT_API_PORT: u16 = 3000;

/// API version prefix
pub const API_VERSION: &str = "v1";

// =============================================================================
// BLOCKCHAIN DATA SOURCES
// Each source provides different entropy for keyword derivation
// =============================================================================

/// Data sources for keyword derivation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDataSource {
    /// Use blockhash (default)
    Blockhash,
    /// Use previous blockhash
    PreviousBlockhash,
    /// Use merkle root of transactions
    TransactionRoot,
    /// Use block rewards
    Rewards,
    /// Use number of transactions
    TransactionCount,
}

impl BlockDataSource {
    /// Get all available data sources
    pub fn all() -> &'static [BlockDataSource] {
        &[
            BlockDataSource::Blockhash,
            BlockDataSource::PreviousBlockhash,
            BlockDataSource::TransactionRoot,
            BlockDataSource::TransactionCount,
        ]
    }
}
