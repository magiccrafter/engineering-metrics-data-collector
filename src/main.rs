use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);

    let my_str = include_str!("gitlab_group_mrs_query.graphql");
    print!("{my_str}");

    Ok(())
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
