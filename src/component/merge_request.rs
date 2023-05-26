use crate::client::gitlab_graphql_client;
use crate::store::Store;

use serde::Deserialize;
use serde::Serialize;
use serde_json;
use sqlx::Row;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

// #[serde_as]
// #[derive(Serialize, Some(Deserialize), Debug)]
#[derive(Debug)]
pub struct MergeRequest {
    pub mr_id: String,
    pub mr_title: String,
    pub mr_web_url: String,
    pub project_id: String,
    pub project_name: String,
    pub created_at: OffsetDateTime,
    // date and time when the merge request was last updated
    pub updated_at: OffsetDateTime,
    // date and time when the merge request was merged
    pub merged_at: Option<OffsetDateTime>,
    // username of the user who created the merge request, i.e. @username
    pub created_by: String,
    // username of the user who merged the merge request, i.e. @username
    pub merged_by: Option<String>,
    // boolean flag indicating if the merge request was approved
    pub approved: bool,
    // list of usernames of the users who approved the merge request, i.e. @username
    pub approved_by: Option<Vec<String>>,
    // state: String,
    pub diff_stats_summary: Option<DiffStatsSummary>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiffStatsSummary {
    pub additions: i32,
    pub deletions: i32,
    pub changes: i32,
    pub file_count: i32,
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
    authorization_header: &str,
    group_full_path: &str,
    updated_after: &str,
    after_pointer_token: Option<String>,
) -> MergeRequestsWithPageInfo {
    let group_data = gitlab_graphql_client::GitlabGraphQLClient::new(authorization_header)
        .await
        .fetch_group_merge_requests(gitlab_graphql_client, group_full_path, updated_after, after_pointer_token)
        .await;
    println!("group_data: {:?}", &group_data);

    let mut merge_requests: Vec<MergeRequest> = Vec::new();
    for mr in group_data.merge_requests.nodes.expect("GroupMergeReqsGroupMergeRequestsNodes is None") {
        let mr_ref = mr.as_ref().expect("mr is None");
        merge_requests.push(MergeRequest {
            mr_id: mr_ref.id.clone(),
            mr_title: mr_ref.title.clone(),
            mr_web_url: mr_ref.web_url.clone(),
            project_id: mr_ref.project_id.clone().to_string(),
            project_name: mr_ref.project.name.clone(),
            created_at: OffsetDateTime::parse(
                &mr_ref.created_at.clone(),
                &Rfc3339,
            ).unwrap(),
            updated_at: OffsetDateTime::parse(
                &mr_ref.updated_at.clone(),
                &Rfc3339,
            ).unwrap(),
            merged_at: mr_ref.merged_at.clone()
                .map(|m_at| OffsetDateTime::parse(&m_at,&Rfc3339).unwrap()
            ),
            created_by: mr_ref.author.username.clone(),
            merged_by: mr_ref.merge_user.as_ref()
                .map(|m_by| m_by.username.clone()
            ),
            approved_by: mr_ref.approved_by.as_ref()
                .map(|a_by| a_by.nodes.as_ref().unwrap()
                    .iter()
                    .map(|a_by_node| a_by_node.as_ref().expect("a_by_node is None").username.clone())
                    .collect()
                ),
            approved: mr_ref.approved,
            diff_stats_summary: mr_ref.diff_stats_summary.as_ref()
            .map(|diff_stats_summary| DiffStatsSummary {
                    additions: diff_stats_summary.additions as i32,
                    deletions: diff_stats_summary.deletions as i32,
                    changes: diff_stats_summary.changes as i32,
                    file_count: diff_stats_summary.file_count as i32,
                }
            ),
        });
    }
    
    MergeRequestsWithPageInfo {
        merge_requests,
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
        INSERT INTO engineering_metrics.merge_requests (mr_id, mr_title, mr_web_url, project_id, project_name, created_at, updated_at, merged_at, 
            created_by, merged_by, approved, approved_by, diff_stats_summary)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (mr_id) DO 
        UPDATE SET 
            mr_title = $2,
            updated_at = $7,
            merged_at = $8,
            merged_by = $10,
            approved = $11,
            approved_by = $12,
            diff_stats_summary = $13
        "#)
        .bind(&merge_request.mr_id)
        .bind(&merge_request.mr_title)
        .bind(&merge_request.mr_web_url)
        .bind(&merge_request.project_id)
        .bind(&merge_request.project_name)
        .bind(merge_request.created_at)
        .bind(merge_request.updated_at)
        .bind(merge_request.merged_at)
        .bind(&merge_request.created_by)
        .bind(&merge_request.merged_by)
        .bind(merge_request.approved)
        .bind(serde_json::to_value(&merge_request.approved_by).unwrap())
        .bind(serde_json::to_value(&merge_request.diff_stats_summary).unwrap())
    .execute(&mut conn)
    .await
    .unwrap();
}

pub async fn import_merge_requests(
    gitlab_graphql_client: &str,
    authorization_header: &str,
    group_full_path: &str,
    updated_after: &str,
    store: &Store,
) {

    let mut has_more_merge_requests = true;
    let mut after_pointer_token = Option::None;

    while has_more_merge_requests {
        let res = fetch_group_merge_requests(
            gitlab_graphql_client,
            authorization_header,
            group_full_path,
            updated_after,
            after_pointer_token.clone(),
        ).await;

        for merge_request in res.merge_requests {
            persist_merge_request(store, &merge_request).await;
        }

        after_pointer_token = res.page_info.end_cursor;
        has_more_merge_requests = res.page_info.has_next_page;
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
        SELECT mr_id, mr_title, project_id, project_name, created_at, merged_at, diff_stats_summary
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
                mr_web_url: row.get("mr_web_url"),
                project_id: row.get("project_id"),
                project_name: row.get("project_name"),
                created_at: OffsetDateTime::parse(
                    &row.get::<String, _>("created_at"),
                    &Rfc3339,
                ).unwrap(),
                updated_at: OffsetDateTime::parse(
                    &row.get::<String, _>("updated_at"),
                    &Rfc3339,
                ).unwrap(),
                merged_at: Some(OffsetDateTime::parse(
                    &row.get::<String, _>("merged_at"),
                    &Rfc3339,
                ).unwrap()),
                created_by: row.get("created_by"),
                merged_by: row.get("merged_by"),
                approved: row.get("approved"),
                approved_by: match row.try_get("approved_by") {
                    Ok(value) => Some(serde_json::from_value(value).unwrap()),
                    Err(_) => None,
                },
                diff_stats_summary: match row.try_get("diff_stats_summary") {
                    Ok(value) => Some(serde_json::from_value(value).unwrap()),
                    Err(_) => Default::default(),
                }
            })
            .collect()
    })
    .unwrap();

    println!("merge_requests: {:?}", merge_requests);
}
