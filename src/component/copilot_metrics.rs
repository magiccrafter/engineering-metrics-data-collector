use crate::client::copilot_usage_metrics_client::{
    CopilotDailyUserMetricsRecord, CopilotFeatureMetrics, CopilotIdeMetrics,
    CopilotLanguageFeatureMetrics, CopilotLanguageModelMetrics, CopilotModelFeatureMetrics,
    CopilotUsageMetricsError,
};
use crate::component::copilot_collector_runs::{
    CopilotCollectorRunsError, CopilotCollectorRunsHandler,
};
use crate::component::import_progress::{ImportProgressError, ImportProgressHandler};
use crate::context::CopilotContext;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

const DAY_FORMAT: &[time::format_description::FormatItem<'static>] =
    format_description!("[year]-[month]-[day]");
const IMPORT_TYPE: &str = "copilot_users_1_day";

#[derive(Error, Debug)]
pub enum CopilotMetricsError {
    #[error("GitHub Copilot usage metrics error: {0}")]
    ClientError(#[from] CopilotUsageMetricsError),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Date parsing error: {0}")]
    DateParseError(#[from] time::error::Parse),
    #[error("Date formatting error: {0}")]
    DateFormatError(#[from] time::error::Format),
    #[error("Import progress error: {0}")]
    ImportProgressError(#[from] ImportProgressError),
    #[error("Copilot collector runs error: {0}")]
    CollectorRunsError(#[from] CopilotCollectorRunsError),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

#[derive(Debug, Clone)]
pub struct CopilotMetricsHandler {
    pub context: CopilotContext,
}

#[derive(Debug, Default, Clone)]
pub struct CopilotMetricsImportSummary {
    pub days_advanced: i32,
    pub records_persisted: i64,
    pub last_completed_report_day: Option<Date>,
}

impl CopilotMetricsHandler {
    pub async fn import_org_users_usage_metrics(
        &self,
        org_slug: &str,
        updated_after: &str,
    ) -> Result<CopilotMetricsImportSummary, CopilotMetricsError> {
        let updated_after_time = OffsetDateTime::parse(updated_after, &Rfc3339)?;
        let import_progress_handler = ImportProgressHandler {
            store: self.context.store.clone(),
        };
        let group_key = Self::import_progress_group_key(org_slug);
        let import_progress = import_progress_handler
            .get_or_create_import(&group_key, IMPORT_TYPE, updated_after_time)
            .await?;

        let last_successful_run = CopilotCollectorRunsHandler {
            store: self.context.store.clone(),
        }
        .fetch_last_successful_collector_run(org_slug)
        .await?;

        let mut current_day = match import_progress.last_cursor.as_deref() {
            Some(last_cursor) => next_day(Date::parse(last_cursor, DAY_FORMAT)?)?,
            None => last_successful_run
                .as_ref()
                .map(|run| next_day(run.last_completed_report_day))
                .transpose()?
                .unwrap_or(import_progress.updated_after.date()),
        };

        let end_day =
            OffsetDateTime::now_utc().date() - Duration::days(self.context.report_lag_days.max(0));

        let mut summary = CopilotMetricsImportSummary::default();

        if current_day > end_day {
            import_progress_handler
                .mark_completed(import_progress.id)
                .await?;
            return Ok(summary);
        }

        while current_day <= end_day {
            let day_string = current_day.format(DAY_FORMAT)?;

            let report_for_day = match self
                .context
                .copilot_usage_metrics_client
                .fetch_org_users_usage_report_for_day(org_slug, &day_string)
                .await
            {
                Ok(report) => report,
                Err(error) => {
                    let _ = import_progress_handler
                        .mark_failed(import_progress.id, &error.to_string())
                        .await;
                    return Err(error.into());
                }
            };

            match report_for_day {
                Some(report) => {
                    let mut records = Vec::new();

                    for download_link in report.download_links {
                        let mut partial_records = match self
                            .context
                            .copilot_usage_metrics_client
                            .download_users_usage_report(&download_link)
                            .await
                        {
                            Ok(partial_records) => partial_records,
                            Err(error) => {
                                let _ = import_progress_handler
                                    .mark_failed(import_progress.id, &error.to_string())
                                    .await;
                                return Err(error.into());
                            }
                        };

                        records.append(&mut partial_records);
                    }

                    match self
                        .persist_org_daily_metrics(org_slug, current_day, &records)
                        .await
                    {
                        Ok(persisted_records) => {
                            summary.records_persisted +=
                                i64::try_from(persisted_records).map_err(|_| {
                                    CopilotMetricsError::InvalidState(
                                        "Persisted record count does not fit into i64".to_string(),
                                    )
                                })?;
                        }
                        Err(error) => {
                            let _ = import_progress_handler
                                .mark_failed(import_progress.id, &error.to_string())
                                .await;
                            return Err(error);
                        }
                    }
                }
                None if current_day == end_day => {
                    println!(
                        "No report content available yet for org={}, day={}; leaving cursor unchanged.",
                        org_slug, day_string
                    );
                    break;
                }
                None => {
                    println!(
                        "No report content available for org={}, day={}; advancing cursor.",
                        org_slug, day_string
                    );
                }
            }

            import_progress_handler
                .update_progress(import_progress.id, Some(&day_string), 1)
                .await?;
            summary.days_advanced += 1;
            summary.last_completed_report_day = Some(current_day);
            current_day = next_day(current_day)?;
        }

        import_progress_handler
            .mark_completed(import_progress.id)
            .await?;

        Ok(summary)
    }

    async fn persist_org_daily_metrics(
        &self,
        org_slug: &str,
        report_day: Date,
        records: &[CopilotDailyUserMetricsRecord],
    ) -> Result<usize, CopilotMetricsError> {
        let mut transaction = self.context.store.conn_pool.begin().await?;

        for record in records {
            let record_day = Date::parse(&record.day, DAY_FORMAT)?;
            if record_day != report_day {
                return Err(CopilotMetricsError::InvalidState(format!(
                    "Report day mismatch for org {}: expected {}, got {}",
                    org_slug, report_day, record.day
                )));
            }

            let user_metric_id = self
                .upsert_user_metric(&mut transaction, org_slug, report_day, record)
                .await?;
            self.clear_child_metrics(&mut transaction, user_metric_id)
                .await?;
            self.insert_feature_metrics(
                &mut transaction,
                user_metric_id,
                &record.totals_by_feature,
            )
            .await?;
            self.insert_ide_metrics(&mut transaction, user_metric_id, &record.totals_by_ide)
                .await?;
            self.insert_language_feature_metrics(
                &mut transaction,
                user_metric_id,
                &record.totals_by_language_feature,
            )
            .await?;
            self.insert_language_model_metrics(
                &mut transaction,
                user_metric_id,
                &record.totals_by_language_model,
            )
            .await?;
            self.insert_model_feature_metrics(
                &mut transaction,
                user_metric_id,
                &record.totals_by_model_feature,
            )
            .await?;
        }

        transaction.commit().await?;

        Ok(records.len())
    }

    async fn upsert_user_metric(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        org_slug: &str,
        report_day: Date,
        record: &CopilotDailyUserMetricsRecord,
    ) -> Result<Uuid, CopilotMetricsError> {
        let enterprise_id = record
            .enterprise_id
            .as_deref()
            .filter(|value| !value.is_empty());
        let totals_by_cli = serde_json::to_value(&record.totals_by_cli)?;
        let raw_payload = serde_json::to_value(record)?;

        Ok(sqlx::query_scalar(
            r#"
            INSERT INTO engineering_metrics.copilot_user_daily_metrics (
                org_slug,
                report_day,
                user_id,
                user_login,
                organization_id,
                enterprise_id,
                user_initiated_interaction_count,
                code_generation_activity_count,
                code_acceptance_activity_count,
                loc_suggested_to_add_sum,
                loc_suggested_to_delete_sum,
                loc_added_sum,
                loc_deleted_sum,
                used_agent,
                used_chat,
                used_cli,
                used_copilot_coding_agent,
                used_copilot_cloud_agent,
                totals_by_cli,
                raw_payload,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, NOW()
            )
            ON CONFLICT (org_slug, report_day, user_id)
            DO UPDATE SET
                user_login = EXCLUDED.user_login,
                organization_id = EXCLUDED.organization_id,
                enterprise_id = EXCLUDED.enterprise_id,
                user_initiated_interaction_count = EXCLUDED.user_initiated_interaction_count,
                code_generation_activity_count = EXCLUDED.code_generation_activity_count,
                code_acceptance_activity_count = EXCLUDED.code_acceptance_activity_count,
                loc_suggested_to_add_sum = EXCLUDED.loc_suggested_to_add_sum,
                loc_suggested_to_delete_sum = EXCLUDED.loc_suggested_to_delete_sum,
                loc_added_sum = EXCLUDED.loc_added_sum,
                loc_deleted_sum = EXCLUDED.loc_deleted_sum,
                used_agent = EXCLUDED.used_agent,
                used_chat = EXCLUDED.used_chat,
                used_cli = EXCLUDED.used_cli,
                used_copilot_coding_agent = EXCLUDED.used_copilot_coding_agent,
                used_copilot_cloud_agent = EXCLUDED.used_copilot_cloud_agent,
                totals_by_cli = EXCLUDED.totals_by_cli,
                raw_payload = EXCLUDED.raw_payload,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(org_slug)
        .bind(report_day)
        .bind(record.user_id)
        .bind(&record.user_login)
        .bind(&record.organization_id)
        .bind(enterprise_id)
        .bind(record.user_initiated_interaction_count)
        .bind(record.code_generation_activity_count)
        .bind(record.code_acceptance_activity_count)
        .bind(record.loc_suggested_to_add_sum)
        .bind(record.loc_suggested_to_delete_sum)
        .bind(record.loc_added_sum)
        .bind(record.loc_deleted_sum)
        .bind(record.used_agent)
        .bind(record.used_chat)
        .bind(record.used_cli)
        .bind(record.used_copilot_coding_agent)
        .bind(record.used_copilot_cloud_agent)
        .bind(totals_by_cli)
        .bind(raw_payload)
        .fetch_one(&mut **transaction)
        .await?)
    }

    async fn clear_child_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
    ) -> Result<(), CopilotMetricsError> {
        for table_name in [
            "engineering_metrics.copilot_user_daily_feature_metrics",
            "engineering_metrics.copilot_user_daily_ide_metrics",
            "engineering_metrics.copilot_user_daily_language_feature_metrics",
            "engineering_metrics.copilot_user_daily_language_model_metrics",
            "engineering_metrics.copilot_user_daily_model_feature_metrics",
        ] {
            let delete_query = format!("DELETE FROM {} WHERE user_metric_id = $1", table_name);
            sqlx::query(&delete_query)
                .bind(user_metric_id)
                .execute(&mut **transaction)
                .await?;
        }

        Ok(())
    }

    async fn insert_feature_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
        metrics: &[CopilotFeatureMetrics],
    ) -> Result<(), CopilotMetricsError> {
        for metric in metrics {
            sqlx::query(
                r#"
                INSERT INTO engineering_metrics.copilot_user_daily_feature_metrics (
                    user_metric_id,
                    feature,
                    user_initiated_interaction_count,
                    code_generation_activity_count,
                    code_acceptance_activity_count,
                    loc_suggested_to_add_sum,
                    loc_suggested_to_delete_sum,
                    loc_added_sum,
                    loc_deleted_sum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(user_metric_id)
            .bind(&metric.feature)
            .bind(metric.user_initiated_interaction_count)
            .bind(metric.code_generation_activity_count)
            .bind(metric.code_acceptance_activity_count)
            .bind(metric.loc_suggested_to_add_sum)
            .bind(metric.loc_suggested_to_delete_sum)
            .bind(metric.loc_added_sum)
            .bind(metric.loc_deleted_sum)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    async fn insert_ide_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
        metrics: &[CopilotIdeMetrics],
    ) -> Result<(), CopilotMetricsError> {
        for metric in metrics {
            sqlx::query(
                r#"
                INSERT INTO engineering_metrics.copilot_user_daily_ide_metrics (
                    user_metric_id,
                    ide,
                    user_initiated_interaction_count,
                    code_generation_activity_count,
                    code_acceptance_activity_count,
                    loc_suggested_to_add_sum,
                    loc_suggested_to_delete_sum,
                    loc_added_sum,
                    loc_deleted_sum,
                    last_known_plugin_version,
                    last_known_ide_version
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(user_metric_id)
            .bind(&metric.ide)
            .bind(metric.user_initiated_interaction_count)
            .bind(metric.code_generation_activity_count)
            .bind(metric.code_acceptance_activity_count)
            .bind(metric.loc_suggested_to_add_sum)
            .bind(metric.loc_suggested_to_delete_sum)
            .bind(metric.loc_added_sum)
            .bind(metric.loc_deleted_sum)
            .bind(to_json_value(&metric.last_known_plugin_version)?)
            .bind(to_json_value(&metric.last_known_ide_version)?)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    async fn insert_language_feature_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
        metrics: &[CopilotLanguageFeatureMetrics],
    ) -> Result<(), CopilotMetricsError> {
        for metric in metrics {
            sqlx::query(
                r#"
                INSERT INTO engineering_metrics.copilot_user_daily_language_feature_metrics (
                    user_metric_id,
                    language,
                    feature,
                    code_generation_activity_count,
                    code_acceptance_activity_count,
                    loc_suggested_to_add_sum,
                    loc_suggested_to_delete_sum,
                    loc_added_sum,
                    loc_deleted_sum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(user_metric_id)
            .bind(&metric.language)
            .bind(&metric.feature)
            .bind(metric.code_generation_activity_count)
            .bind(metric.code_acceptance_activity_count)
            .bind(metric.loc_suggested_to_add_sum)
            .bind(metric.loc_suggested_to_delete_sum)
            .bind(metric.loc_added_sum)
            .bind(metric.loc_deleted_sum)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    async fn insert_language_model_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
        metrics: &[CopilotLanguageModelMetrics],
    ) -> Result<(), CopilotMetricsError> {
        for metric in metrics {
            sqlx::query(
                r#"
                INSERT INTO engineering_metrics.copilot_user_daily_language_model_metrics (
                    user_metric_id,
                    language,
                    model,
                    code_generation_activity_count,
                    code_acceptance_activity_count,
                    loc_suggested_to_add_sum,
                    loc_suggested_to_delete_sum,
                    loc_added_sum,
                    loc_deleted_sum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(user_metric_id)
            .bind(&metric.language)
            .bind(&metric.model)
            .bind(metric.code_generation_activity_count)
            .bind(metric.code_acceptance_activity_count)
            .bind(metric.loc_suggested_to_add_sum)
            .bind(metric.loc_suggested_to_delete_sum)
            .bind(metric.loc_added_sum)
            .bind(metric.loc_deleted_sum)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    async fn insert_model_feature_metrics(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        user_metric_id: Uuid,
        metrics: &[CopilotModelFeatureMetrics],
    ) -> Result<(), CopilotMetricsError> {
        for metric in metrics {
            sqlx::query(
                r#"
                INSERT INTO engineering_metrics.copilot_user_daily_model_feature_metrics (
                    user_metric_id,
                    model,
                    feature,
                    user_initiated_interaction_count,
                    code_generation_activity_count,
                    code_acceptance_activity_count,
                    loc_suggested_to_add_sum,
                    loc_suggested_to_delete_sum,
                    loc_added_sum,
                    loc_deleted_sum
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(user_metric_id)
            .bind(&metric.model)
            .bind(&metric.feature)
            .bind(metric.user_initiated_interaction_count)
            .bind(metric.code_generation_activity_count)
            .bind(metric.code_acceptance_activity_count)
            .bind(metric.loc_suggested_to_add_sum)
            .bind(metric.loc_suggested_to_delete_sum)
            .bind(metric.loc_added_sum)
            .bind(metric.loc_deleted_sum)
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    fn import_progress_group_key(org_slug: &str) -> String {
        format!("github_org:{}", org_slug)
    }
}

fn next_day(day: Date) -> Result<Date, CopilotMetricsError> {
    day.next_day().ok_or_else(|| {
        CopilotMetricsError::InvalidState(format!("No next day available after {}", day))
    })
}

fn to_json_value<T: serde::Serialize>(value: &T) -> Result<Value, CopilotMetricsError> {
    Ok(serde_json::to_value(value)?)
}
