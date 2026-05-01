use engineering_metrics_data_collector::component::copilot_collector_runs::{
    CopilotCollectorRun, CopilotCollectorRunsHandler,
};
use engineering_metrics_data_collector::store::Store;
use testcontainers::runners::AsyncRunner;
use time::{Duration, OffsetDateTime};

mod postgres_container;

#[tokio::test]
async fn should_fetch_zero_copilot_collector_runs_for_a_new_org() {
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();

    let handler = CopilotCollectorRunsHandler {
        store: store.clone(),
    };

    let result = handler
        .fetch_last_successful_collector_run("test-org")
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn should_persist_and_fetch_the_last_copilot_collector_run_for_an_org() {
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();

    let handler = CopilotCollectorRunsHandler {
        store: store.clone(),
    };
    let started_at = OffsetDateTime::now_utc();
    let completed_at = started_at + Duration::seconds(1);
    let last_completed_report_day = completed_at.date() - Duration::days(1);

    handler
        .persist_successful_run(&CopilotCollectorRun {
            org_slug: "test-org".to_string(),
            last_successful_run_started_at: started_at,
            last_successful_run_completed_at: completed_at,
            last_completed_report_day,
        })
        .await
        .unwrap();

    let result = handler
        .fetch_last_successful_collector_run("test-org")
        .await
        .unwrap()
        .expect("expected one Copilot collector run");

    assert_eq!(result.org_slug, "test-org");
    assert_eq!(
        result.last_successful_run_started_at.unix_timestamp(),
        started_at.unix_timestamp()
    );
    assert_eq!(
        result.last_successful_run_completed_at.unix_timestamp(),
        completed_at.unix_timestamp()
    );
    assert_eq!(result.last_completed_report_day, last_completed_report_day);
}
