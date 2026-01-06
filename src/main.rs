use engineering_metrics_data_collector::client::gitlab_graphql_client::GitlabGraphQLClient;
use engineering_metrics_data_collector::client::gitlab_rest_client::GitlabRestClient;
use engineering_metrics_data_collector::component::collector_runs;
use engineering_metrics_data_collector::component::merge_request::MergeRequestHandler;
use engineering_metrics_data_collector::component::project::ProjectHandler;

use engineering_metrics_data_collector::context::GitlabContext;
use engineering_metrics_data_collector::store::Store;
use std::env;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = OffsetDateTime::now_utc();
    /*
       RUST_BACKTRACE is an environment variable that controls whether Rust programs display a backtrace when they encounter a panic.
       A backtrace is a list of function calls that shows the sequence of events that led up to the panic.
       By default, Rust programs do not display a backtrace when they panic.
       However, you can enable backtraces by setting the RUST_BACKTRACE environment variable to 1 or full.
       Setting RUST_BACKTRACE=1 will display a brief backtrace, while setting RUST_BACKTRACE=full will display a more detailed backtrace.
    */
    env::set_var("RUST_BACKTRACE", "1");
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable is not set.")
        .to_string();
    let gitlab_rest_endpoint = env::var("GITLAB_REST_ENDPOINT")
        .expect("GITLAB_REST_ENDPOINT environment variable is not set.")
        .to_string();
    let gitlab_graphql_endpoint = env::var("GITLAB_GRAPHQL_ENDPOINT")
        .expect("GITLAB_GRAPHQL_ENDPOINT environment variable is not set.")
        .to_string();
    let authorization_header = env::var("GITLAB_API_TOKEN")
        .expect("GITLAB_API_TOKEN environment variable is not set.")
        .to_string();
    let group_full_paths = env::var("GITLAB_FULL_PATH_GROUP_LIST")
        .expect("GITLAB_FULL_PATH_GROUP_LIST environment variable is not set.")
        .to_string();
    let ai_base_url = env::var("AI_BASE_URL")
        .expect("AI_BASE_URL environment variable is not set.")
        .to_string();
    let ai_model = env::var("AI_MODEL")
        .expect("AI_MODEL environment variable is not set.")
        .to_string();
    let ai_api_key = env::var("AI_API_KEY")
        .expect("AI_API_KEY environment variable is not set.")
        .to_string();
    let ai_max_context_chars: usize = env::var("AI_MAX_CONTEXT_CHARS")
        .expect("AI_MAX_CONTEXT_CHARS environment variable is not set.")
        .parse()
        .expect("AI_MAX_CONTEXT_CHARS must be a valid number");

    let gitlab_graphql_client =
        GitlabGraphQLClient::new(&authorization_header, gitlab_graphql_endpoint)?;
    let gitlab_rest_client = GitlabRestClient::new(&authorization_header, gitlab_rest_endpoint)?;

    let store = Store::new(&database_url).await;
    store.migrate().await?;

    let collector_runs_handler = collector_runs::CollectorRunsHandler {
        store: store.clone(),
    };
    let last_successful_collector_run = collector_runs_handler
        .fetch_last_successfull_collector_run()
        .await?;
    let updated_after = match &last_successful_collector_run {
        Some(run) => run.last_successful_run_completed_at.format(&Rfc3339)?,
        None => {
            // Check for INITIAL_INGESTION_DATE env var, otherwise use current time
            env::var("INITIAL_INGESTION_DATE")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| OffsetDateTime::now_utc().format(&Rfc3339).unwrap())
        }
    };

    println!(
        "Last successful collector run: {:?}",
        last_successful_collector_run
    );
    println!("Fetching data updated after: {}", updated_after);

    let context = GitlabContext {
        store: store.clone(),
        gitlab_rest_client,
        gitlab_graphql_client,
        ai_base_url,
        ai_model,
        ai_api_key,
        ai_max_context_chars,
    };

    let project_handler = ProjectHandler {
        context: context.clone(),
    };
    let merge_request_handler = MergeRequestHandler {
        context: context.clone(),
    };

    let group_full_paths: Vec<String> =
        group_full_paths.split(',').map(|s| s.to_string()).collect();
    for group_full_path in &group_full_paths {
        println!("Processing group: {}", group_full_path);
        let mut futures = Vec::new();

        // projects
        let project_handler = project_handler.clone();
        let gfp1 = group_full_path.clone();
        let task = tokio::spawn(async move {
            println!("Starting projects import for group={}", &gfp1);
            project_handler.import_projects(&gfp1).await;
        });
        futures.push(task);

        // merge requests
        let gfp2 = group_full_path.clone();
        let ua1 = updated_after.clone();
        let merge_request_handler = merge_request_handler.clone();
        let task = tokio::spawn(async move {
            println!(
                "Starting merge requests import for group={}, updated_after={}",
                &gfp2, &ua1
            );
            merge_request_handler
                .import_merge_requests(&gfp2, &ua1)
                .await;
        });
        futures.push(task);

        let results = futures::future::join_all(futures).await;
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                eprintln!("Task {} failed for group {}: {:?}", i, group_full_path, e);
            }
        }
    }

    let end_time = OffsetDateTime::now_utc();
    collector_runs_handler
        .persist_successful_run(&collector_runs::CollectorRun {
            last_successful_run_started_at: start_time,
            last_successful_run_completed_at: end_time,
        })
        .await?;
    let elapsed = end_time - start_time;
    println!("Time elapsed: {:?}", elapsed);

    Ok(())
}
