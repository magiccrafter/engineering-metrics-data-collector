use sqlx::Row;
use time::OffsetDateTime;

use crate::store::Store;

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
    pub async fn fetch_last_successfull_collector_run(&self) -> Option<CollectorRun> {
        let mut conn = self.store.conn_pool.acquire().await.unwrap();
        let row = sqlx::query(
            r#"
            SELECT last_successful_run_started_at, last_successful_run_completed_at
            FROM engineering_metrics.collector_runs
            ORDER BY last_successful_run_started_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *conn)
        .await
        .unwrap();

        row.map(|row| {
            let last_successful_run_started_at: OffsetDateTime = row.get(0);
            let last_successful_run_completed_at: OffsetDateTime = row.get(1);
            CollectorRun {
                last_successful_run_started_at,
                last_successful_run_completed_at,
            }
        })
    }

    pub async fn persist_successful_run(&self, run: &CollectorRun) {
        let mut conn = self.store.conn_pool.acquire().await.unwrap();
        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.collector_runs (last_successful_run_started_at, last_successful_run_completed_at)
            VALUES ($1, $2)
            "#)
            .bind(run.last_successful_run_started_at)
            .bind(run.last_successful_run_completed_at)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
}
