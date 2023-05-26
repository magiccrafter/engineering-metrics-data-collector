use engineering_metrics_data_collector::client;
use engineering_metrics_data_collector::component::merge_request;

use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::env;
use std::env::var;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use engineering_metrics_data_collector::store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap().to_string();
    let gitlab_graphql_endpoint = env::var("GITLAB_GRAPHQL_ENDPOINT").unwrap().to_string();
    let authorization_header = env::var("EM_TOKEN").unwrap().to_string();
    let updated_after = env::var("UPDATED_AFTER").unwrap().to_string();
    let group_full_path = env::var("GROUP_FULL_PATH").unwrap().to_string();

    let mut has_more_merge_requests = true;
    let mut after_pointer_token: core::option::Option<String> = None;

    while has_more_merge_requests {
        let group_data = client::gitlab_graphql_client::GitlabGraphQLClient::new(&authorization_header.clone())
            .await
            .fetch_group_merge_requests(&gitlab_graphql_endpoint, &group_full_path.clone(), &updated_after.clone(), after_pointer_token.clone())
            .await;

        let mut merge_requests: Vec<MergeRequest2> = Vec::new();
        for mr in group_data.merge_requests.nodes.expect("GroupMergeReqsGroupMergeRequestsNodes is None") {
            let mr_ref = mr.as_ref();
            merge_requests.push(MergeRequest2 {
                mr_id: mr_ref.expect("mr.id is None").id.clone(),
                mr_title: mr_ref.expect("mr.title is None").title.clone(),
            });
        }
        for mr in merge_requests {
            println!("mr: {:?}", &mr);
        }
        after_pointer_token = group_data.merge_requests.page_info.end_cursor;
        println!("after_pointer_token: {:?}", &after_pointer_token);
        has_more_merge_requests = group_data.merge_requests.page_info.has_next_page;
        println!("has_next_page: {:?}", &has_more_merge_requests);
    }

    // import the merge requests
    let store = Store::new(&database_url).await;
    store.migrate().await.unwrap();
    merge_request::import_merge_requests(&gitlab_graphql_endpoint, &authorization_header, &group_full_path, &updated_after, &store).await;

    Ok(())
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct Issue {
    issue_id: String,
    issue_title: String,
    project_id: String,
    // `OffsetDateTime`'s default serialization format is not standard.
    // https://docs.rs/serde_with/latest/serde_with/guide/serde_as_transformations/index.html#well-known-time-formats-for-offsetdatetime
    #[serde_as(as = "Rfc3339")]
    created_at: OffsetDateTime,
    created_by: String,
    #[serde_as(as = "Rfc3339")]
    updated_at: OffsetDateTime,
    updated_by: String,
    #[serde_as(as = "Rfc3339")]
    closed_at: OffsetDateTime,
    closed_by: String,
}



#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct ClosedIssueOnMerge {
    issue_id: String,
    mr_id: String,
    mr_title: String,
}


#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MergeRequest2 {
    mr_id: String,
    mr_title: String,
}