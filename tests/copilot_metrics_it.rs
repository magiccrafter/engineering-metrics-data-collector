use engineering_metrics_data_collector::client::copilot_usage_metrics_client::CopilotUsageMetricsClient;
use engineering_metrics_data_collector::component::copilot_metrics::CopilotMetricsHandler;
use engineering_metrics_data_collector::context::CopilotContext;
use engineering_metrics_data_collector::store::Store;
use serde_json::json;
use sqlx::Row;
use testcontainers::runners::AsyncRunner;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod postgres_container;

#[tokio::test]
async fn should_successfully_import_copilot_metrics_from_github_to_the_database() {
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();

    let report_day = OffsetDateTime::now_utc().date() - Duration::days(2);
    let report_day_string = report_day.format(day_format()).unwrap();

    let mock_server = MockServer::start().await;
    mount_report_day_mocks(
        &mock_server,
        "test-org",
        &report_day_string,
        vec![
            format!("{}/downloads/part1", mock_server.uri()),
            format!("{}/downloads/part2", mock_server.uri()),
        ],
    )
    .await;
    mount_download_mock(
        &mock_server,
        "/downloads/part1",
        &build_jsonl_body(&[build_user_record(
            &report_day_string,
            101,
            "user-one",
            false,
        )]),
        200,
    )
    .await;
    mount_download_mock(
        &mock_server,
        "/downloads/part2",
        &build_jsonl_body(&[build_user_record(&report_day_string, 202, "user-two", true)]),
        200,
    )
    .await;

    let handler = CopilotMetricsHandler {
        context: CopilotContext {
            store: store.clone(),
            copilot_usage_metrics_client: CopilotUsageMetricsClient::new(
                "test-token",
                mock_server.uri(),
                "2026-03-10",
            )
            .unwrap(),
            github_api_version: "2026-03-10".to_string(),
            report_lag_days: 2,
        },
    };

    let updated_after = midnight_timestamp_for_day(&report_day_string).unwrap();
    let summary = handler
        .import_org_users_usage_metrics("test-org", &updated_after)
        .await
        .unwrap();

    assert_eq!(summary.days_advanced, 1);
    assert_eq!(summary.records_persisted, 2);
    assert_eq!(summary.last_completed_report_day, Some(report_day));

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let result =
        sqlx::query("SELECT COUNT(*) AS count FROM engineering_metrics.copilot_user_daily_metrics")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(result.get::<i64, _>("count"), 2);

    let first_row = sqlx::query(
        "SELECT org_slug, report_day, user_login, used_cli, totals_by_cli
         FROM engineering_metrics.copilot_user_daily_metrics
         WHERE user_id = 202",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(first_row.get::<String, _>("org_slug"), "test-org");
    assert_eq!(first_row.get::<String, _>("user_login"), "user-two");
    assert!(first_row.get::<bool, _>("used_cli"));
    assert_eq!(
        first_row.get::<serde_json::Value, _>("totals_by_cli"),
        json!({
            "session_count": 1,
            "request_count": 42,
            "prompt_count": 5,
            "token_usage": {
                "output_tokens_sum": 100,
                "prompt_tokens_sum": 2000,
                "avg_tokens_per_request": 50.0
            },
            "last_known_cli_version": {
                "sampled_at": format!("{}T09:00:00Z", report_day_string),
                "cli_version": "1.0.39"
            }
        })
    );

    let feature_rows = sqlx::query(
        "SELECT COUNT(*) AS count FROM engineering_metrics.copilot_user_daily_feature_metrics",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(feature_rows.get::<i64, _>("count"), 4);

    let ide_rows = sqlx::query(
        "SELECT COUNT(*) AS count FROM engineering_metrics.copilot_user_daily_ide_metrics",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(ide_rows.get::<i64, _>("count"), 2);
}

#[tokio::test]
async fn should_resume_copilot_metrics_import_after_a_failed_day() {
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();

    let first_day = OffsetDateTime::now_utc().date() - Duration::days(3);
    let second_day = OffsetDateTime::now_utc().date() - Duration::days(2);
    let first_day_string = first_day.format(day_format()).unwrap();
    let second_day_string = second_day.format(day_format()).unwrap();
    let updated_after = midnight_timestamp_for_day(&first_day_string).unwrap();

    let failing_server = MockServer::start().await;
    mount_report_day_mocks(
        &failing_server,
        "test-org",
        &first_day_string,
        vec![format!("{}/downloads/day1", failing_server.uri())],
    )
    .await;
    mount_report_day_mocks(
        &failing_server,
        "test-org",
        &second_day_string,
        vec![format!("{}/downloads/day2", failing_server.uri())],
    )
    .await;
    mount_download_mock(
        &failing_server,
        "/downloads/day1",
        &build_jsonl_body(&[build_user_record(
            &first_day_string,
            1001,
            "resume-user-one",
            false,
        )]),
        200,
    )
    .await;
    mount_download_mock(&failing_server, "/downloads/day2", "boom", 500).await;

    let failing_handler = CopilotMetricsHandler {
        context: CopilotContext {
            store: store.clone(),
            copilot_usage_metrics_client: CopilotUsageMetricsClient::new(
                "test-token",
                failing_server.uri(),
                "2026-03-10",
            )
            .unwrap(),
            github_api_version: "2026-03-10".to_string(),
            report_lag_days: 2,
        },
    };

    let first_attempt = failing_handler
        .import_org_users_usage_metrics("test-org", &updated_after)
        .await;
    assert!(first_attempt.is_err());

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let failed_progress = sqlx::query(
        "SELECT last_cursor, status FROM engineering_metrics.import_progress WHERE import_type = 'copilot_users_1_day'",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(
        failed_progress.get::<Option<String>, _>("last_cursor"),
        Some(first_day_string.clone())
    );
    assert_eq!(failed_progress.get::<String, _>("status"), "failed");

    let recovering_server = MockServer::start().await;
    mount_report_day_mocks(
        &recovering_server,
        "test-org",
        &first_day_string,
        vec![format!("{}/downloads/day1", recovering_server.uri())],
    )
    .await;
    mount_report_day_mocks(
        &recovering_server,
        "test-org",
        &second_day_string,
        vec![format!("{}/downloads/day2", recovering_server.uri())],
    )
    .await;
    mount_download_mock(
        &recovering_server,
        "/downloads/day1",
        &build_jsonl_body(&[build_user_record(
            &first_day_string,
            1001,
            "resume-user-one",
            false,
        )]),
        200,
    )
    .await;
    mount_download_mock(
        &recovering_server,
        "/downloads/day2",
        &build_jsonl_body(&[build_user_record(
            &second_day_string,
            1002,
            "resume-user-two",
            false,
        )]),
        200,
    )
    .await;

    let recovering_handler = CopilotMetricsHandler {
        context: CopilotContext {
            store: store.clone(),
            copilot_usage_metrics_client: CopilotUsageMetricsClient::new(
                "test-token",
                recovering_server.uri(),
                "2026-03-10",
            )
            .unwrap(),
            github_api_version: "2026-03-10".to_string(),
            report_lag_days: 2,
        },
    };

    let summary = recovering_handler
        .import_org_users_usage_metrics("test-org", &updated_after)
        .await
        .unwrap();

    assert_eq!(summary.days_advanced, 1);
    assert_eq!(summary.records_persisted, 1);
    assert_eq!(summary.last_completed_report_day, Some(second_day));

    let total_parent_rows =
        sqlx::query("SELECT COUNT(*) AS count FROM engineering_metrics.copilot_user_daily_metrics")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(total_parent_rows.get::<i64, _>("count"), 2);

    let completed_progress = sqlx::query(
        "SELECT last_cursor, status FROM engineering_metrics.import_progress WHERE import_type = 'copilot_users_1_day' ORDER BY started_at DESC LIMIT 1",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(
        completed_progress.get::<Option<String>, _>("last_cursor"),
        Some(second_day_string)
    );
    assert_eq!(completed_progress.get::<String, _>("status"), "completed");
}

async fn mount_report_day_mocks(
    mock_server: &MockServer,
    org_slug: &str,
    day: &str,
    download_links: Vec<String>,
) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/orgs/{}/copilot/metrics/reports/users-1-day",
            org_slug
        )))
        .and(query_param("day", day))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "download_links": download_links,
            "report_day": day,
        })))
        .mount(mock_server)
        .await;
}

async fn mount_download_mock(mock_server: &MockServer, target_path: &str, body: &str, status: u16) {
    Mock::given(method("GET"))
        .and(path(target_path))
        .respond_with(ResponseTemplate::new(status).set_body_string(body.to_string()))
        .mount(mock_server)
        .await;
}

fn build_jsonl_body(records: &[serde_json::Value]) -> String {
    let mut body = records
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    body.push('\n');
    body
}

fn build_user_record(
    day: &str,
    user_id: i64,
    user_login: &str,
    used_cli: bool,
) -> serde_json::Value {
    let cli_payload = if used_cli {
        Some(json!({
            "session_count": 1,
            "request_count": 42,
            "prompt_count": 5,
            "token_usage": {
                "output_tokens_sum": 100,
                "prompt_tokens_sum": 2000,
                "avg_tokens_per_request": 50.0
            },
            "last_known_cli_version": {
                "sampled_at": format!("{}T09:00:00Z", day),
                "cli_version": "1.0.39"
            }
        }))
    } else {
        None
    };

    json!({
        "user_id": user_id,
        "user_login": user_login,
        "day": day,
        "organization_id": "208004797",
        "enterprise_id": "",
        "user_initiated_interaction_count": 4,
        "code_generation_activity_count": 8,
        "code_acceptance_activity_count": 3,
        "totals_by_ide": [
            {
                "ide": "vscode",
                "user_initiated_interaction_count": 4,
                "code_generation_activity_count": 8,
                "code_acceptance_activity_count": 3,
                "loc_suggested_to_add_sum": 12,
                "loc_suggested_to_delete_sum": 2,
                "loc_added_sum": 10,
                "loc_deleted_sum": 1,
                "last_known_plugin_version": {
                    "sampled_at": format!("{}T08:00:00Z", day),
                    "plugin": "copilot-chat",
                    "plugin_version": "0.45.1"
                },
                "last_known_ide_version": {
                    "sampled_at": format!("{}T08:00:00Z", day),
                    "ide_version": "1.117.0"
                }
            }
        ],
        "totals_by_feature": [
            {
                "feature": "code_completion",
                "user_initiated_interaction_count": 0,
                "code_generation_activity_count": 8,
                "code_acceptance_activity_count": 3,
                "loc_suggested_to_add_sum": 12,
                "loc_suggested_to_delete_sum": 2,
                "loc_added_sum": 10,
                "loc_deleted_sum": 1
            },
            {
                "feature": "chat_panel_agent_mode",
                "user_initiated_interaction_count": 4,
                "code_generation_activity_count": 0,
                "code_acceptance_activity_count": 0,
                "loc_suggested_to_add_sum": 0,
                "loc_suggested_to_delete_sum": 0,
                "loc_added_sum": 0,
                "loc_deleted_sum": 0
            }
        ],
        "totals_by_language_feature": [
            {
                "language": "rust",
                "feature": "code_completion",
                "code_generation_activity_count": 8,
                "code_acceptance_activity_count": 3,
                "loc_suggested_to_add_sum": 12,
                "loc_suggested_to_delete_sum": 2,
                "loc_added_sum": 10,
                "loc_deleted_sum": 1
            }
        ],
        "totals_by_language_model": [
            {
                "language": "rust",
                "model": "gpt-5.4",
                "code_generation_activity_count": 8,
                "code_acceptance_activity_count": 3,
                "loc_suggested_to_add_sum": 12,
                "loc_suggested_to_delete_sum": 2,
                "loc_added_sum": 10,
                "loc_deleted_sum": 1
            }
        ],
        "totals_by_model_feature": [
            {
                "model": "gpt-5.4",
                "feature": "code_completion",
                "user_initiated_interaction_count": 0,
                "code_generation_activity_count": 8,
                "code_acceptance_activity_count": 3,
                "loc_suggested_to_add_sum": 12,
                "loc_suggested_to_delete_sum": 2,
                "loc_added_sum": 10,
                "loc_deleted_sum": 1
            }
        ],
        "used_agent": true,
        "used_chat": true,
        "loc_suggested_to_add_sum": 12,
        "loc_suggested_to_delete_sum": 2,
        "loc_added_sum": 10,
        "loc_deleted_sum": 1,
        "used_cli": used_cli,
        "totals_by_cli": cli_payload,
        "used_copilot_coding_agent": false,
        "used_copilot_cloud_agent": false
    })
}

fn midnight_timestamp_for_day(day: &str) -> Result<String, time::error::Parse> {
    let timestamp = format!("{}T00:00:00Z", day);
    Ok(OffsetDateTime::parse(&timestamp, &Rfc3339)?
        .format(&Rfc3339)
        .unwrap())
}

fn day_format() -> &'static [time::format_description::FormatItem<'static>] {
    time::macros::format_description!("[year]-[month]-[day]")
}
