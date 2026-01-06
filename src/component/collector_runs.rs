use sqlx::Row;
use thiserror::Error;
use time::OffsetDateTime;

use crate::store::Store;

#[derive(Error, Debug)]
pub enum CollectorRunsError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct CollectorRunsHandler {
    pub store: Store,
}

#[derive(Debug)]
pub struct CollectorRun {
    pub last_successful_run_started_at: OffsetDateTime,
    pub last_successful_run_completed_at: OffsetDateTime,
}

impl CollectorRunsHandler {
    pub async fn fetch_last_successfull_collector_run(
        &self,
    ) -> Result<Option<CollectorRun>, CollectorRunsError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let row = sqlx::query(
            r#"
            SELECT last_successful_run_started_at, last_successful_run_completed_at
            FROM engineering_metrics.collector_runs
            ORDER BY last_successful_run_started_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row.map(|row| {
            let last_successful_run_started_at: OffsetDateTime = row.get(0);
            let last_successful_run_completed_at: OffsetDateTime = row.get(1);
            CollectorRun {
                last_successful_run_started_at,
                last_successful_run_completed_at,
            }
        }))
    }

    pub async fn persist_successful_run(
        &self,
        run: &CollectorRun,
    ) -> Result<(), CollectorRunsError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.collector_runs (last_successful_run_started_at, last_successful_run_completed_at)
            VALUES ($1, $2)
            "#)
            .bind(run.last_successful_run_started_at)
            .bind(run.last_successful_run_completed_at)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}
