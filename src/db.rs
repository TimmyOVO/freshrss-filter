use anyhow::Result;
use sqlx::{sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}, Pool, Sqlite};
use std::str::FromStr;
use std::sync::Arc;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Database(pub Arc<Pool<Sqlite>>);

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;

        let db = Database(Arc::new(pool));
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS reviews (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_id TEXT NOT NULL,
                hash TEXT NOT NULL,
                is_ad INTEGER NOT NULL,
                confidence REAL NOT NULL,
                reason TEXT NOT NULL,
                reviewed_at TEXT NOT NULL
            );"#,
        ).execute(self.pool()).await?;

        sqlx::query(
            r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_item ON reviews(item_id);"#,
        ).execute(self.pool()).await?;

        Ok(())
    }

    pub fn pool(&self) -> &Pool<Sqlite> { &self.0 }

    pub async fn has_reviewed(&self, item_id: &str) -> Result<bool> {
        let rec: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM reviews WHERE item_id = ? LIMIT 1")
            .bind(item_id)
            .fetch_optional(self.pool())
            .await?;
        Ok(rec.is_some())
    }

    pub async fn save_review(
        &self,
        item_id: &str,
        hash: &str,
        is_ad: bool,
        confidence: f32,
        reason: &str,
    ) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        sqlx::query("INSERT OR REPLACE INTO reviews(item_id, hash, is_ad, confidence, reason, reviewed_at) VALUES(?,?,?,?,?,?)")
            .bind(item_id)
            .bind(hash)
            .bind(if is_ad {1} else {0})
            .bind(confidence)
            .bind(reason)
            .bind(now.to_rfc3339())
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

