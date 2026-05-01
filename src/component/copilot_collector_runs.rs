use sqlx::Row;
use thiserror::Error;
use time::{Date, OffsetDateTime};

use crate::store::Store;

#[derive(Error, Debug)]
pub enum CopilotCollectorRunsError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct CopilotCollectorRunsHandler {
    pub store: Store,
}

#[derive(Debug, Clone)]
pub struct CopilotCollectorRun {
    pub org_slug: String,
    pub last_successful_run_started_at: OffsetDateTime,
    pub last_successful_run_completed_at: OffsetDateTime,
    pub last_completed_report_day: Date,
}

impl CopilotCollectorRunsHandler {
    pub async fn fetch_last_successful_collector_run(
        &self,
        org_slug: &str,
    ) -> Result<Option<CopilotCollectorRun>, CopilotCollectorRunsError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        let row = sqlx::query(
            r#"
            SELECT org_slug, last_successful_run_started_at, last_successful_run_completed_at, last_completed_report_day
            FROM engineering_metrics.copilot_collector_runs
            WHERE org_slug = $1
            ORDER BY last_successful_run_started_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_slug)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row.map(|row| CopilotCollectorRun {
            org_slug: row.get(0),
            last_successful_run_started_at: row.get(1),
            last_successful_run_completed_at: row.get(2),
            last_completed_report_day: row.get(3),
        }))
    }

    pub async fn persist_successful_run(
        &self,
        run: &CopilotCollectorRun,
    ) -> Result<(), CopilotCollectorRunsError> {
        let mut conn = self.store.conn_pool.acquire().await?;
        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.copilot_collector_runs (
                org_slug,
                last_successful_run_started_at,
                last_successful_run_completed_at,
                last_completed_report_day
            )
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&run.org_slug)
        .bind(run.last_successful_run_started_at)
        .bind(run.last_successful_run_completed_at)
        .bind(run.last_completed_report_day)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}
