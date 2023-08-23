use engineering_metrics_data_collector::component::{
    external_issue, issue, merge_request, project,
};

use engineering_metrics_data_collector::store::Store;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let updated_after = env::var("UPDATED_AFTER")
        .expect("UPDATED_AFTER environment variable is not set.")
        .to_string();
    let group_full_paths = env::var("GITLAB_FULL_PATH_GROUP_LIST")
        .expect("GITLAB_FULL_PATH_GROUP_LIST environment variable is not set.")
        .to_string();
    let external_issue_tracker_enabled =
        env::var("EXTERNAL_ISSUE_TRACKER_ENABLED").map_or_else(|_| false, |val| val == "true");

    let store = Store::new(&database_url).await;
    store.migrate().await.unwrap();

    let group_full_paths: Vec<&str> = group_full_paths.split(',').collect();
    for group_full_path in group_full_paths {
        project::import_projects(
            &gitlab_graphql_endpoint,
            &authorization_header,
            group_full_path,
            &store,
        )
        .await;
        merge_request::import_merge_requests(
            &gitlab_rest_endpoint,
            &gitlab_graphql_endpoint,
            &authorization_header,
            group_full_path,
            &updated_after,
            &store,
        )
        .await;
        issue::import_issues(
            &gitlab_graphql_endpoint,
            &authorization_header,
            group_full_path,
            &updated_after,
            &store,
        )
        .await;
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
            external_issue::import_external_issues(
                &atlassian_jira_issue_url_prefix,
                &atlassian_rest_endpoint,
                &atlassian_authorization_header,
                &updated_after,
                &store,
            )
            .await;
        } else {
            println!("External issue tracker is disabled.");
        }
    }

    Ok(())
}
