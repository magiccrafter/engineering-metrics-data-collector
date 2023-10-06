use std::sync::Arc;

use crate::client::atlassian_rest_client::{self, JiraIssue};
use crate::store::Store;

use sqlx::Row;
use time::OffsetDateTime;

#[derive(Debug)]
pub struct ExternalIssue {
    pub issue_tracker: String,
    pub issue_id: String,
    pub issue_display_id: String,
    pub title: String,
    pub web_url: String,
}

pub async fn import_external_issues(
    atlassian_jira_issue_url_prefix: &str,
    atlassian_rest_endpoint: &str,
    authorization_header: &str,
    updated_after: &str,
    store: &Arc<Store>,
) {
    select_newly_closed_external_issues_on_merge_and_try_importing_them_as_external_issues(
        atlassian_jira_issue_url_prefix,
        atlassian_rest_endpoint,
        authorization_header,
        store,
        20,
        updated_after,
    )
    .await;
}

pub async fn try_importing_jira_issues(
    atlassian_jira_issue_url_prefix: &str,
    atlassian_rest_endpoint: &str,
    authorization_header: &str,
    closed_external_issues_on_merge: &Vec<ClosedExternalIssueOnMerge>,
    store: &Store,
) {
    for i in closed_external_issues_on_merge {
        let jira_issue =
            try_fetching_jira_issue(atlassian_rest_endpoint, authorization_header, &i.issue_id)
                .await;

        if let Some(issue) = jira_issue.as_ref() {
            let external_issue = ExternalIssue {
                issue_tracker: "jira".to_string(),
                issue_id: issue.id.clone(),
                issue_display_id: i.issue_id.clone(),
                title: issue.fields.summary.clone(),
                web_url: format!("{}{}", atlassian_jira_issue_url_prefix, i.issue_id),
            };
            persist_external_issue(store, &external_issue).await;
        } else {
            println!(
                "Jira issue with id={} not found or not accessible.",
                i.issue_id
            );
        }
    }
}

pub async fn try_fetching_jira_issue(
    atlassian_rest_endpoint: &str,
    authorization_header: &str,
    issue_id: &str,
) -> Option<JiraIssue> {
    let rest_client = atlassian_rest_client::AtlassianRestClient::new(authorization_header).await;
    let result = rest_client
        .fetch_jira_issue(issue_id, atlassian_rest_endpoint)
        .await;

    match result {
        Ok(data) => Some(data),
        Err(_) => {
            println!("Error fetching Jira issue with id={}", issue_id);
            None
        }
    }
}

pub async fn persist_external_issue(store: &Store, issue: &ExternalIssue) {
    let mut conn = store.conn_pool.acquire().await.unwrap();
    sqlx::query(
        r#"
        INSERT INTO engineering_metrics.external_issues (issue_tracker, issue_id, issue_display_id, title, web_url, imported_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (issue_tracker, issue_id) DO
        UPDATE SET
            issue_display_id = $3,
            title = $4,
            web_url = $5,
            imported_at = NOW()
        "#)
        .bind(&issue.issue_tracker)
        .bind(&issue.issue_id)
        .bind(&issue.issue_display_id)
        .bind(&issue.title)
        .bind(&issue.web_url)
    .execute(&mut conn)
    .await
    .unwrap();
}

#[derive(Debug)]
pub struct ClosedExternalIssueOnMerge {
    pub issue_id: String,
    pub created_at: OffsetDateTime,
}

// Selects and imports all external issues that are closed on merge and haven't been resolved and stored yet.
// external issues are the ones that have issue_iid = null
async fn select_newly_closed_external_issues_on_merge_and_try_importing_them_as_external_issues(
    atlassian_jira_issue_url_prefix: &str,
    atlassian_rest_endpoint: &str,
    authorization_header: &str,
    store: &Store,
    page_size: i32,
    updated_after: &str,
) {
    let mut page_number = 1;

    let mut conn = store.conn_pool.acquire().await.unwrap();
    loop {
        let offset = (page_number - 1) * page_size;
        let rows = sqlx::query(
            r#"
            SELECT issue_id, created_at FROM engineering_metrics.closed_issues_on_merge 
            WHERE issue_iid is null and created_at >= to_timestamp($3, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')
            ORDER BY issue_id, created_at 
            LIMIT $1 
            OFFSET $2
        "#)
        .bind(page_size)
        .bind(offset)
        .bind(updated_after)
        .fetch_all(&mut conn)
        .await
        .unwrap();

        if rows.is_empty() {
            break;
        }

        let closed_issues: Vec<ClosedExternalIssueOnMerge> = rows
            .iter()
            .map(|row| ClosedExternalIssueOnMerge {
                issue_id: row.get(0),
                created_at: row.get(1),
            })
            .collect();

        try_importing_jira_issues(
            atlassian_jira_issue_url_prefix,
            atlassian_rest_endpoint,
            authorization_header,
            &closed_issues,
            store,
        )
        .await;

        page_number += 1;
    }
}
