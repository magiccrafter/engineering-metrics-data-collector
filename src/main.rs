use gql_client::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::collections::HashMap;
use std::env;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);

    // let qraphql_query = include_str!("gitlab_group_mrs_query.graphql");
    let qraphql_query = include_str!("test.graphql");
    println!("{qraphql_query}");

    let endpoint = "https://gitlab.com/api/graphql";
    let authorization_header = env::var("EM_TOKEN").unwrap_or("none".to_string());
    println!("{authorization_header}");
    let headers = HashMap::from([
        ("Authorization", authorization_header.as_str()),
        ("Content-Type", "application/json"),
    ]);
    let client = Client::new_with_headers(endpoint, headers);

    let group_full_path = env::var("GROUP_FULL_PATH").unwrap_or("none".to_string());
    println!("{group_full_path}");
    let vars = GroupMergeRequestGraphQLVars {
        group_full_path: group_full_path,
    };
    let data = client
        .query_with_vars::<GroupData, GroupMergeRequestGraphQLVars>(qraphql_query, vars)
        .await
        .unwrap();

    println!("Group Data: {:?}", data);

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Data {
    user: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    id: String,
    name: String,
}

#[derive(Serialize)]
pub struct Vars {
    id: u32,
}

#[derive(Serialize)]
pub struct GroupMergeRequestGraphQLVars {
    group_full_path: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupData {
    group: Group,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MergeRequest {}

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
