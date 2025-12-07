use engineering_metrics_data_collector::client::atlassian_rest_client::AtlassianRestClient;
use engineering_metrics_data_collector::client::gitlab_graphql_client::GitlabGraphQLClient;
use engineering_metrics_data_collector::client::gitlab_rest_client::GitlabRestClient;
use engineering_metrics_data_collector::component::issue::IssueHandler;
use engineering_metrics_data_collector::component::merge_request::MergeRequestHandler;
use engineering_metrics_data_collector::component::project::ProjectHandler;
use engineering_metrics_data_collector::component::{collector_runs, external_issue};

use engineering_metrics_data_collector::context::{AtlassianContext, GitlabContext};
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
    let external_issue_tracker_enabled =
        env::var("EXTERNAL_ISSUE_TRACKER_ENABLED").map_or_else(|_| false, |val| val == "true");

    let gitlab_graphql_client =
        GitlabGraphQLClient::new(&authorization_header, gitlab_graphql_endpoint).await;
    let gitlab_rest_client =
        GitlabRestClient::new(&authorization_header, gitlab_rest_endpoint).await;

    let store = Store::new(&database_url).await;
    store.migrate().await.unwrap();

    let collector_runs_handler = collector_runs::CollectorRunsHandler {
        store: store.clone(),
    };
    let last_successful_collector_run = collector_runs_handler
        .fetch_last_successfull_collector_run()
        .await;
    let updated_after = match &last_successful_collector_run {
        Some(run) => run.last_successful_run_completed_at.format(&Rfc3339)?,
        None => OffsetDateTime::now_utc().format(&Rfc3339)?,
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
    };

    let project_handler = ProjectHandler {
        context: context.clone(),
    };
    let merge_request_handler = MergeRequestHandler {
        context: context.clone(),
    };
    let issue_handler = IssueHandler {
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

        // issues
        let ua2 = updated_after.clone();
        let gfp3 = group_full_path.clone();
        let issue_handler = issue_handler.clone();
        let task = tokio::spawn(async move {
            println!(
                "Starting issues import for group={}, updated_after={}",
                &gfp3, &ua2
            );
            issue_handler.import_issues(&gfp3, &ua2).await;
        });
        futures.push(task);

        let results = futures::future::join_all(futures).await;
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                eprintln!("Task {} failed for group {}: {:?}", i, group_full_path, e);
            }
        }
    }

    let mut futures = Vec::new();
    for _ in &group_full_paths {
        if external_issue_tracker_enabled {
            let atlassian_rest_endpoint = env::var("ATLASSIAN_REST_ENDPOINT")
                .expect("ATLASSIAN_REST_ENDPOINT environment variable is not set.")
                .to_string();
            let atlassian_authorization_header = env::var("ATLASSIAN_API_TOKEN")
                .expect("ATLASSIAN_API_TOKEN environment variable is not set.")
                .to_string();
            let atlassian_jira_issue_url_prefix = env::var("ATLASSIAN_JIRA_ISSUE_URL_PREFIX")
                .expect("ATLASSIAN_JIRA_ISSUE_URL_PREFIX environment variable is not set.")
                .to_string();

            let atlassian_jira_rest_client =
                AtlassianRestClient::new(&atlassian_authorization_header, atlassian_rest_endpoint)
                    .await;
            let atlassian_context = AtlassianContext {
                store: store.clone(),
                atlassian_jira_issue_url_prefix,
                atlassian_jira_rest_client,
            };

            let external_issue_handler = external_issue::ExternalIssueHandler {
                context: atlassian_context,
            };

            let ua3 = updated_after.clone();
            let task = tokio::spawn(async move {
                external_issue_handler.import_external_issues(&ua3).await;
            });
            futures.push(task);
        } else {
            println!("External issue tracker is disabled.");
        }
    }
    futures::future::join_all(futures).await;

    let end_time = OffsetDateTime::now_utc();
    collector_runs_handler
        .persist_successful_run(&collector_runs::CollectorRun {
            last_successful_run_started_at: start_time,
            last_successful_run_completed_at: end_time,
        })
        .await;
    let elapsed = end_time - start_time;
    println!("Time elapsed: {:?}", elapsed);

    Ok(())
}
