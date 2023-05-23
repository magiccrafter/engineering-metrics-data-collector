use crate::client;
use crate::client::gitlab_graphql_client;
use crate::store::Store;

use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use sqlx::Row;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct MergeRequest {
    pub mr_id: String,
    pub mr_title: String,
    pub project_id: String,
    // `OffsetDateTime`'s default serialization format is not standard.
    // https://docs.rs/serde_with/latest/serde_with/guide/serde_as_transformations/index.html#well-known-time-formats-for-offsetdatetime
    // #[serde_as(as = "Rfc3339")]
    // created_at: OffsetDateTime,
    // created_by: String,
    // #[serde_as(as = "Rfc3339")]
    // merged_at: OffsetDateTime,
    // merged_by: String,
    // #[serde_as(as = "Rfc3339")]
    // updated_at: OffsetDateTime,
    // updated_by: String,
    // state: String,
}

#[derive(Debug)]
pub struct MergeRequestsWithPageInfo {
    pub merge_requests: Vec<MergeRequest>,
    pub page_info: PageInfo,
}

#[derive(Debug)]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

pub async fn fetch_group_merge_requests(
    gitlab_graphql_client: &str,
    authorization_header: &String,
    group_full_path: &String,
    updated_after: &String,
    after_pointer_token: Option<String>,
) -> MergeRequestsWithPageInfo {
    let group_data = client::gitlab_graphql_client::GitlabGraphQLClient::new(&authorization_header.clone())
        .await
        .fetch_group_merge_requests(gitlab_graphql_client, &group_full_path.clone(), &updated_after.clone(), after_pointer_token.clone())
        .await;
    println!("group_data: {:?}", &group_data);

    let mut merge_requests: Vec<MergeRequest> = Vec::new();
    for mr in group_data.merge_requests.nodes.expect("GroupMergeReqsGroupMergeRequestsNodes is None") {
        let mr_ref = mr.as_ref();
        merge_requests.push(MergeRequest {
            mr_id: mr_ref.expect("mr.id is None").id.clone(),
            mr_title: mr_ref.expect("mr.title is None").title.clone(),
            project_id: mr_ref.expect("mr.project_id is None").project_id.clone().to_string(),
            // created_at: OffsetDateTime::parse(
            //     &mr_ref.expect("mr.created_at is None").created_at.clone(),
            //     &Rfc3339,
            // ).unwrap(),
        });
    }
    
    MergeRequestsWithPageInfo {
        merge_requests: merge_requests,
        page_info: PageInfo {
            end_cursor: group_data.merge_requests.page_info.end_cursor,
            has_next_page: group_data.merge_requests.page_info.has_next_page,
        },
    }
}

pub async fn persist_merge_request(
    store: &Store,
    merge_request: &MergeRequest,
) {
    let mut conn = store.conn_pool.acquire().await.unwrap();

    sqlx::query(
        r#"
        INSERT INTO engineering_metrics.merge_requests (mr_id, mr_title, project_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (mr_id) DO 
        UPDATE SET 
            mr_title = $2, 
            project_id = $3
        "#)
        .bind(&merge_request.mr_id)
        .bind(&merge_request.mr_title)
        .bind(&merge_request.project_id)
    .execute(&mut conn)
    .await
    .unwrap();
}

pub async fn import_merge_requests(
    gitlab_graphql_client: &str,
    authorization_header: &String,
    group_full_path: &String,
    updated_after: &String,
    store: &Store,
) -> () {

    let mut has_more_merge_requests = true;
    let mut after_pointer_token = Option::None;

    while has_more_merge_requests {
        let res = fetch_group_merge_requests(
            &gitlab_graphql_client,
            &authorization_header.clone(),
            &group_full_path.clone(),
            &updated_after.clone(),
            after_pointer_token.clone(),
        ).await;

        after_pointer_token = res.page_info.end_cursor;
        has_more_merge_requests = res.page_info.has_next_page;

        for merge_request in res.merge_requests {
            persist_merge_request(&store, &merge_request).await;
        }
    }
}

/// select all merge requests from postgresql where merge request updated_after column is creater than and print them
pub async fn print_merge_requests(
    store: &Store,
    updated_after: &String,
) {
    let mut conn = store.conn_pool.acquire().await.unwrap();
    let merge_requests: Vec<MergeRequest> = sqlx::query(
        r#"
        SELECT mr_id, mr_title, project_id
        FROM merge_requests
        WHERE updated_at > $1
        "#)
    .bind(updated_after)
    .fetch_all(&mut conn)
    .await
    .map(|recs| {
        recs
            .into_iter()
            .map(|row| MergeRequest {
                mr_id: row.get("mr_id"),
                mr_title: row.get("mr_title"),
                project_id: row.get("project_id"),
            })
            .collect()
    })
    .unwrap();

    println!("merge_requests: {:?}", merge_requests);
}
