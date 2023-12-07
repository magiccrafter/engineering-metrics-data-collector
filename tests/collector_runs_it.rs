use std::ops::Add;
use std::time::Duration;

use engineering_metrics_data_collector::component::collector_runs::{
    CollectorRun, CollectorRunsHandler,
};
use engineering_metrics_data_collector::store::Store;
use testcontainers::clients;
mod postgres_container;

use time::OffsetDateTime;

#[tokio::test]
async fn should_fetch_zero_collector_runs_from_db() {
    let docker = clients::Cli::default();
    let image = postgres_container::Postgres::default();
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5432);

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;

    store.migrate().await.unwrap();

    let collector_runs_handler = CollectorRunsHandler {
        store: store.clone(),
    };

    let result = collector_runs_handler
        .fetch_last_successfull_collector_run()
        .await;

    assert_eq!(result.is_none(), true);
}

#[tokio::test]
async fn should_persist_and_then_fetch_last_successful_collector_run_from_db() {
    let docker = clients::Cli::default();
    let image = postgres_container::Postgres::default();
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5432);

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;

    store.migrate().await.unwrap();

    let collector_runs_handler = CollectorRunsHandler {
        store: store.clone(),
    };

    let collector_run = CollectorRun {
        last_successful_run_started_at: OffsetDateTime::now_utc(),
        last_successful_run_completed_at: OffsetDateTime::now_utc() + Duration::from_secs(1),
    };

    collector_runs_handler
        .persist_successful_run(&collector_run)
        .await;

    let result = collector_runs_handler
        .fetch_last_successfull_collector_run()
        .await;

    assert_eq!(result.is_some(), true);

    let unwrapped_result = result.unwrap();
    assert_eq!(
        unwrapped_result
            .last_successful_run_started_at
            .unix_timestamp(),
        collector_run
            .last_successful_run_started_at
            .unix_timestamp()
    );
    assert_eq!(
        unwrapped_result.last_successful_run_completed_at,
        collector_run.last_successful_run_completed_at
    );
}
