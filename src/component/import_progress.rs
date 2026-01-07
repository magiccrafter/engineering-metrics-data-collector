use sqlx::Row;
use std::str::FromStr;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::store::Store;

#[derive(Error, Debug)]
pub enum ImportProgressError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct ImportProgressHandler {
    pub store: Store,
}

#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub id: Uuid,
    pub group_full_path: String,
    pub import_type: String,
    pub updated_after: OffsetDateTime,
    pub last_cursor: Option<String>,
    pub total_processed: i32,
    pub status: ImportStatus,
    pub started_at: OffsetDateTime,
    pub last_activity_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    InProgress,
    Completed,
    Failed,
}

impl ImportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportStatus::InProgress => "in_progress",
            ImportStatus::Completed => "completed",
            ImportStatus::Failed => "failed",
        }
    }
}

impl FromStr for ImportStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in_progress" => Ok(ImportStatus::InProgress),
            "completed" => Ok(ImportStatus::Completed),
            "failed" => Ok(ImportStatus::Failed),
            _ => Err(format!("Unknown import status: {}", s)),
        }
    }
}

impl ImportProgressHandler {
    /// Find an existing in-progress import to resume, or create a new one
    pub async fn get_or_create_import(
        &self,
        group_full_path: &str,
        import_type: &str,
        updated_after: OffsetDateTime,
    ) -> Result<ImportProgress, ImportProgressError> {
        // First, try to find an existing in-progress import
        if let Some(existing) = self
            .find_in_progress_import(group_full_path, import_type)
            .await?
        {
            println!(
                "Resuming existing import for group={}, type={}, cursor={:?}, processed={}",
                group_full_path, import_type, existing.last_cursor, existing.total_processed
            );
            return Ok(existing);
        }

        // Create a new import progress record
        self.create_import(group_full_path, import_type, updated_after)
            .await
    }

    /// Find an in-progress import for the given group and type
    pub async fn find_in_progress_import(
        &self,
        group_full_path: &str,
        import_type: &str,
    ) -> Result<Option<ImportProgress>, ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let row = sqlx::query(
            r#"
            SELECT id, group_full_path, import_type, updated_after, last_cursor, 
                   total_processed, status, started_at, last_activity_at, completed_at, error_message
            FROM engineering_metrics.import_progress
            WHERE group_full_path = $1 
              AND import_type = $2 
              AND status = 'in_progress'
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(group_full_path)
        .bind(import_type)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row.map(|row| {
            let status_str: String = row.get(6);
            ImportProgress {
                id: row.get(0),
                group_full_path: row.get(1),
                import_type: row.get(2),
                updated_after: row.get(3),
                last_cursor: row.get(4),
                total_processed: row.get(5),
                status: ImportStatus::from_str(&status_str).unwrap_or(ImportStatus::Failed),
                started_at: row.get(7),
                last_activity_at: row.get(8),
                completed_at: row.get(9),
                error_message: row.get(10),
            }
        }))
    }

    /// Create a new import progress record
    pub async fn create_import(
        &self,
        group_full_path: &str,
        import_type: &str,
        updated_after: OffsetDateTime,
    ) -> Result<ImportProgress, ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let now = OffsetDateTime::now_utc();
        let id = Uuid::now_v7();

        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.import_progress 
                (id, group_full_path, import_type, updated_after, status, started_at, last_activity_at)
            VALUES ($1, $2, $3, $4, 'in_progress', $5, $5)
            "#,
        )
        .bind(id)
        .bind(group_full_path)
        .bind(import_type)
        .bind(updated_after)
        .bind(now)
        .execute(&mut *conn)
        .await?;

        Ok(ImportProgress {
            id,
            group_full_path: group_full_path.to_string(),
            import_type: import_type.to_string(),
            updated_after,
            last_cursor: None,
            total_processed: 0,
            status: ImportStatus::InProgress,
            started_at: now,
            last_activity_at: now,
            completed_at: None,
            error_message: None,
        })
    }

    /// Update the cursor and processed count after successfully processing a page
    pub async fn update_progress(
        &self,
        import_id: Uuid,
        last_cursor: Option<&str>,
        items_processed: i32,
    ) -> Result<(), ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let now = OffsetDateTime::now_utc();

        sqlx::query(
            r#"
            UPDATE engineering_metrics.import_progress
            SET last_cursor = $2,
                total_processed = total_processed + $3,
                last_activity_at = $4
            WHERE id = $1
            "#,
        )
        .bind(import_id)
        .bind(last_cursor)
        .bind(items_processed)
        .bind(now)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Mark an import as completed
    pub async fn mark_completed(&self, import_id: Uuid) -> Result<(), ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let now = OffsetDateTime::now_utc();

        sqlx::query(
            r#"
            UPDATE engineering_metrics.import_progress
            SET status = 'completed',
                completed_at = $2,
                last_activity_at = $2
            WHERE id = $1
            "#,
        )
        .bind(import_id)
        .bind(now)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Mark an import as failed with an error message
    pub async fn mark_failed(
        &self,
        import_id: Uuid,
        error_message: &str,
    ) -> Result<(), ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let now = OffsetDateTime::now_utc();

        sqlx::query(
            r#"
            UPDATE engineering_metrics.import_progress
            SET status = 'failed',
                error_message = $2,
                last_activity_at = $3
            WHERE id = $1
            "#,
        )
        .bind(import_id)
        .bind(error_message)
        .bind(now)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Clean up old completed/failed imports (optional, for maintenance)
    pub async fn cleanup_old_imports(&self, days_to_keep: i32) -> Result<u64, ImportProgressError> {
        let mut conn = self.store.conn_pool.acquire().await?;

        let result = sqlx::query(
            r#"
            DELETE FROM engineering_metrics.import_progress
            WHERE status IN ('completed', 'failed')
              AND last_activity_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days_to_keep)
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected())
    }
}
