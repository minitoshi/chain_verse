use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;

use crate::derivation::DerivedKeyword;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredKeyword {
    pub id: i64,
    pub word: String,
    pub slot: i64,
    pub blockhash: String,
    pub block_time: Option<i64>,
    pub word_index: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPoem {
    pub id: i64,
    pub date: String,
    pub title: Option<String>,
    pub content: String,
    pub keyword_ids: Vec<i64>,
    pub created_at: String,
}

impl Database {
    /// Create a new database connection and initialize schema
    pub async fn new(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Run migrations
        sqlx::query(include_str!("../schema.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Insert a derived keyword into the database
    pub async fn insert_keyword(&self, keyword: &DerivedKeyword) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO keywords (word, slot, blockhash, block_time, word_index)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(slot) DO NOTHING
            "#,
        )
        .bind(&keyword.word)
        .bind(keyword.slot as i64)
        .bind(&keyword.blockhash)
        .bind(keyword.block_time)
        .bind(keyword.word_index as i64)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert a derived keyword with a specific date (for backfilling historical data)
    pub async fn insert_keyword_with_date(&self, keyword: &DerivedKeyword, date: &str) -> Result<i64> {
        // Create a timestamp for noon on the specified date
        let created_at = format!("{} 12:00:00", date);

        let result = sqlx::query(
            r#"
            INSERT INTO keywords (word, slot, blockhash, block_time, word_index, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(slot) DO NOTHING
            "#,
        )
        .bind(&keyword.word)
        .bind(keyword.slot as i64)
        .bind(&keyword.blockhash)
        .bind(keyword.block_time)
        .bind(keyword.word_index as i64)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all keywords for a specific date
    pub async fn get_keywords_for_date(&self, date: &str) -> Result<Vec<StoredKeyword>> {
        let keywords = sqlx::query_as::<_, (i64, String, i64, String, Option<i64>, i64, String)>(
            r#"
            SELECT id, word, slot, blockhash, block_time, word_index, created_at
            FROM keywords
            WHERE DATE(created_at) = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(date)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, word, slot, blockhash, block_time, word_index, created_at)| StoredKeyword {
            id,
            word,
            slot,
            blockhash,
            block_time,
            word_index,
            created_at,
        })
        .collect();

        Ok(keywords)
    }

    /// Get recent keywords (for today's poem in progress)
    pub async fn get_recent_keywords(&self, limit: i64) -> Result<Vec<StoredKeyword>> {
        let keywords = sqlx::query_as::<_, (i64, String, i64, String, Option<i64>, i64, String)>(
            r#"
            SELECT id, word, slot, blockhash, block_time, word_index, created_at
            FROM keywords
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, word, slot, blockhash, block_time, word_index, created_at)| StoredKeyword {
            id,
            word,
            slot,
            blockhash,
            block_time,
            word_index,
            created_at,
        })
        .collect();

        Ok(keywords)
    }

    /// Insert a poem into the database
    pub async fn insert_poem(
        &self,
        date: &str,
        title: Option<&str>,
        content: &str,
        keyword_ids: &[i64],
    ) -> Result<i64> {
        let keyword_ids_json = serde_json::to_string(keyword_ids)?;

        let result = sqlx::query(
            r#"
            INSERT INTO poems (date, title, content, keyword_ids)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(date) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                keyword_ids = excluded.keyword_ids
            "#,
        )
        .bind(date)
        .bind(title)
        .bind(content)
        .bind(keyword_ids_json)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a poem by date
    pub async fn get_poem_by_date(&self, date: &str) -> Result<Option<StoredPoem>> {
        let row = sqlx::query(
            r#"
            SELECT id, date, title, content, keyword_ids, created_at
            FROM poems
            WHERE date = ?
            "#,
        )
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let keyword_ids: Vec<i64> =
                serde_json::from_str(&row.get::<String, _>("keyword_ids"))?;

            Ok(Some(StoredPoem {
                id: row.get("id"),
                date: row.get("date"),
                title: row.get("title"),
                content: row.get("content"),
                keyword_ids,
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all poems, ordered by date descending
    pub async fn get_all_poems(&self) -> Result<Vec<StoredPoem>> {
        let rows = sqlx::query(
            r#"
            SELECT id, date, title, content, keyword_ids, created_at
            FROM poems
            ORDER BY date DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let poems = rows
            .into_iter()
            .map(|row| {
                let keyword_ids: Vec<i64> =
                    serde_json::from_str(&row.get::<String, _>("keyword_ids")).unwrap_or_default();

                StoredPoem {
                    id: row.get("id"),
                    date: row.get("date"),
                    title: row.get("title"),
                    content: row.get("content"),
                    keyword_ids,
                    created_at: row.get("created_at"),
                }
            })
            .collect();

        Ok(poems)
    }

    /// Get today's date in YYYY-MM-DD format
    pub fn today() -> String {
        Utc::now().format("%Y-%m-%d").to_string()
    }
}
