use engineering_metrics_data_collector::client::copilot_usage_metrics_client::CopilotUsageMetricsClient;
use engineering_metrics_data_collector::component::copilot_collector_runs::{
    CopilotCollectorRun, CopilotCollectorRunsHandler,
};
use engineering_metrics_data_collector::component::copilot_metrics::CopilotMetricsHandler;
use engineering_metrics_data_collector::context::CopilotContext;
use engineering_metrics_data_collector::store::Store;
use futures::future::join_all;
use std::env;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

type TaskError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = OffsetDateTime::now_utc();
    env::set_var("RUST_BACKTRACE", "1");
    dotenv::dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable is not set.");
    let github_api_token =
        env::var("GITHUB_API_TOKEN").expect("GITHUB_API_TOKEN environment variable is not set.");
    let github_api_endpoint =
        env::var("GITHUB_API_ENDPOINT").unwrap_or_else(|_| "https://api.github.com".to_string());
    let github_api_version =
        env::var("GITHUB_API_VERSION").unwrap_or_else(|_| "2026-03-10".to_string());
    let copilot_org_list = env::var("GITHUB_COPILOT_ORG_LIST")
        .expect("GITHUB_COPILOT_ORG_LIST environment variable is not set.");
    let report_lag_days: i64 = env::var("COPILOT_REPORT_LAG_DAYS")
        .unwrap_or_else(|_| "2".to_string())
        .parse()
        .expect("COPILOT_REPORT_LAG_DAYS must be a valid number");
    let initial_ingestion_date = env::var("COPILOT_INITIAL_INGESTION_DATE")
        .ok()
        .filter(|value| !value.is_empty());

    let org_slugs: Vec<String> = copilot_org_list
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if org_slugs.is_empty() {
        return Err("GITHUB_COPILOT_ORG_LIST did not contain any org slugs".into());
    }

    let store = Store::new(&database_url).await;
    store.migrate().await?;

    let copilot_usage_metrics_client = CopilotUsageMetricsClient::new(
        &github_api_token,
        github_api_endpoint,
        &github_api_version,
    )?;
    let context = CopilotContext {
        store: store.clone(),
        copilot_usage_metrics_client,
        github_api_version,
        report_lag_days,
    };

    let copilot_metrics_handler = CopilotMetricsHandler {
        context: context.clone(),
    };
    let copilot_collector_runs_handler = CopilotCollectorRunsHandler {
        store: store.clone(),
    };

    let mut tasks = Vec::new();
    for org_slug in org_slugs {
        let handler = copilot_metrics_handler.clone();
        let runs_handler = copilot_collector_runs_handler.clone();
        let initial_ingestion_date = initial_ingestion_date.clone();

        tasks.push(tokio::spawn(async move {
            import_org_metrics(handler, runs_handler, org_slug, initial_ingestion_date).await
        }));
    }

    let mut all_imports_successful = true;

    for task_result in join_all(tasks).await {
        match task_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                eprintln!("Copilot metrics import failed: {}", error);
                all_imports_successful = false;
            }
            Err(error) => {
                eprintln!("Copilot metrics task failed to join: {}", error);
                all_imports_successful = false;
            }
        }
    }

    let end_time = OffsetDateTime::now_utc();
    if all_imports_successful {
        println!("All Copilot imports completed successfully.");
    } else {
        println!("Some Copilot imports failed.");
    }
    println!("Time elapsed: {:?}", end_time - start_time);

    Ok(())
}

async fn import_org_metrics(
    handler: CopilotMetricsHandler,
    runs_handler: CopilotCollectorRunsHandler,
    org_slug: String,
    initial_ingestion_date: Option<String>,
) -> Result<(), TaskError> {
    let run_started_at = OffsetDateTime::now_utc();
    let last_successful_run = runs_handler
        .fetch_last_successful_collector_run(&org_slug)
        .await?;

    let updated_after = match last_successful_run {
        Some(run) => run.last_successful_run_completed_at.format(&Rfc3339)?,
        None => match initial_ingestion_date {
            Some(initial_ingestion_date) => initial_ingestion_date,
            None => OffsetDateTime::now_utc().format(&Rfc3339)?,
        },
    };

    println!(
        "Starting Copilot metrics import for org={}, updated_after={}",
        org_slug, updated_after
    );

    let summary = handler
        .import_org_users_usage_metrics(&org_slug, &updated_after)
        .await?;

    if let Some(last_completed_report_day) = summary.last_completed_report_day {
        let run_completed_at = OffsetDateTime::now_utc();
        runs_handler
            .persist_successful_run(&CopilotCollectorRun {
                org_slug: org_slug.clone(),
                last_successful_run_started_at: run_started_at,
                last_successful_run_completed_at: run_completed_at,
                last_completed_report_day,
            })
            .await?;

        println!(
            "Completed Copilot metrics import for org={}, days_advanced={}, records_persisted={}, last_completed_report_day={}",
            org_slug,
            summary.days_advanced,
            summary.records_persisted,
            last_completed_report_day
        );
    } else {
        println!(
            "No Copilot report days were advanced for org={}; leaving successful-run watermark unchanged.",
            org_slug
        );
    }

    Ok(())
}
