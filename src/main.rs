use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::env;
use std::env::var;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let qraphql_query = include_str!("gitlab_group_mrs_query.graphql");
    println!("{qraphql_query}");

    let endpoint = "https://gitlab.com/api/graphql";
    let authorization_header = env::var("EM_TOKEN").unwrap().to_string();
    println!("{authorization_header}");
    let updated_after = env::var("UPDATED_AFTER").unwrap().to_string();
    let group_full_path = env::var("GROUP_FULL_PATH").unwrap().to_string();

    let mut has_more_merge_requests = true;
    let mut after_pointer_token: core::option::Option<String> = None;

    while has_more_merge_requests {
        let variables = group_merge_reqs::Variables {
            group_full_path: group_full_path.clone(),
            updated_after: updated_after.clone(),
            after: after_pointer_token.clone(),
        };

        let client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-manager")
            .default_headers(
                std::iter::once((
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&authorization_header).unwrap(),
                ))
                .collect(),
            )
            .build()?;
        let response = post_graphql::<GroupMergeReqs, _>(&client, endpoint, variables).await?;

        let response_data = response.data.expect("missing response data");
        let group_data = response_data.group.unwrap();
        println!("group_data: {:?}", &group_data);
        after_pointer_token = group_data.merge_requests.page_info.end_cursor;
        println!("after_pointer_token: {:?}", &after_pointer_token);
        has_more_merge_requests = group_data.merge_requests.page_info.has_next_page;
        println!("has_next_page: {:?}", &has_more_merge_requests);
    }

    Ok(())
}

type Time = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gitlab_group_mrs_schema.graphql",
    query_path = "src/gitlab_group_mrs_query.graphql",
    response_derives = "Debug"
)]
struct GroupMergeReqs;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct Issue {
    issue_id: String,
    issue_title: String,
    mr_id: String,
    project_id: String,
    // `OffsetDateTime`'s default serialization format is not standard.
    // https://docs.rs/serde_with/latest/serde_with/guide/serde_as_transformations/index.html#well-known-time-formats-for-offsetdatetime
    #[serde_as(as = "Rfc3339")]
    created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    closed_at: OffsetDateTime,
    created_by: String,
    closed_by: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MergeRequest {
    mr_id: String,
    mr_title: String,
    project_id: String,
    // `OffsetDateTime`'s default serialization format is not standard.
    // https://docs.rs/serde_with/latest/serde_with/guide/serde_as_transformations/index.html#well-known-time-formats-for-offsetdatetime
    #[serde_as(as = "Rfc3339")]
    created_at: OffsetDateTime,
    created_by: String,
    #[serde_as(as = "Rfc3339")]
    merged_at: OffsetDateTime,
    merged_by: String,
    #[serde_as(as = "Rfc3339")]
    updated_at: OffsetDateTime,
    updated_by: String,
    state: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct ClosedIssueOnMerge {
    issue_id: String,
    mr_id: String,
    mr_title: String,
}
