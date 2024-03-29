use crate::context::GitlabContext;

use serde_json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct IssueHandler {
    pub context: GitlabContext,
}

#[derive(Debug)]
pub struct Issue {
    pub issue_id: String,
    pub issue_iid: String,
    pub issue_title: String,
    pub issue_web_url: String,
    pub project_id: String,
    pub created_at: OffsetDateTime,
    // date and time when the issue was last updated
    pub updated_at: OffsetDateTime,
    // date and time when the issue was closed
    pub closed_at: Option<OffsetDateTime>,
    // username of the user who created the issue, i.e. @username
    pub created_by: String,
    // username of the user who last updated the issue, i.e. @username
    pub updated_by: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct IssuesWithPageInfo {
    pub issues: Vec<Issue>,
    pub page_info: PageInfo,
}

#[derive(Debug)]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

impl IssueHandler {
    pub async fn fetch_group_issues(
        &self,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> IssuesWithPageInfo {
        let group_data = self
            .context
            .gitlab_graphql_client
            .fetch_group_issues(group_full_path, updated_after, after_pointer_token)
            .await;

        let mut issues: Vec<Issue> = Vec::new();
        for mr in group_data.issues.nodes.expect("GroupIssues is None") {
            let mr_ref = mr.as_ref().expect("mr is None");
            issues.push(Issue {
                issue_id: mr_ref.id.clone(),
                issue_iid: mr_ref.iid.clone(),
                issue_title: mr_ref.title.clone(),
                issue_web_url: mr_ref.web_url.clone(),
                project_id: mr_ref.project_id.clone().to_string(),
                created_at: OffsetDateTime::parse(&mr_ref.created_at.clone(), &Rfc3339).unwrap(),
                closed_at: mr_ref
                    .closed_at
                    .as_ref()
                    .map(|closed_at| OffsetDateTime::parse(&closed_at.clone(), &Rfc3339).unwrap()),
                updated_at: OffsetDateTime::parse(&mr_ref.updated_at.clone(), &Rfc3339).unwrap(),
                created_by: mr_ref.author.username.clone(),
                updated_by: mr_ref.updated_by.as_ref().map(|m_by| m_by.username.clone()),
                labels: mr_ref.labels.as_ref().map(|labels| {
                    labels
                        .nodes
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|label| label.as_ref().expect("label is None").title.clone())
                        .collect()
                }),
            });
        }

        IssuesWithPageInfo {
            issues,
            page_info: PageInfo {
                end_cursor: group_data.issues.page_info.end_cursor,
                has_next_page: group_data.issues.page_info.has_next_page,
            },
        }
    }

    pub async fn persist_issue(&self, issue: &Issue) {
        let mut conn = self.context.store.conn_pool.acquire().await.unwrap();

        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.issues (issue_id, issue_iid, issue_title, issue_web_url, labels, created_at, updated_at, closed_at, created_by, updated_by, project_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (issue_id) DO 
            UPDATE SET 
                issue_iid = $2,
                issue_title = $3,
                issue_web_url = $4,
                labels = $5,
                created_at = $6,
                updated_at = $7,
                closed_at = $8,
                created_by = $9,
                updated_by = $10,
                project_id = $11
            "#)
            .bind(&issue.issue_id)
            .bind(&issue.issue_iid)
            .bind(&issue.issue_title)
            .bind(&issue.issue_web_url)
            .bind(serde_json::to_value(&issue.labels).unwrap())
            .bind(issue.created_at)
            .bind(issue.updated_at)
            .bind(issue.closed_at)
            .bind(&issue.created_by)
            .bind(&issue.updated_by)
            .bind(&issue.project_id)
        .execute(&mut conn)
        .await
        .unwrap();
    }

    pub async fn import_issues(&self, group_full_path: &str, updated_after: &str) {
        let mut has_more_merge_issues = true;
        let mut after_pointer_token = Option::None;

        while has_more_merge_issues {
            let res = self
                .fetch_group_issues(group_full_path, updated_after, after_pointer_token.clone())
                .await;

            for i in res.issues {
                self.persist_issue(&i).await;
            }

            after_pointer_token = res.page_info.end_cursor;
            has_more_merge_issues = res.page_info.has_next_page;
        }
        println!("Done importing issues data for group={}.", &group_full_path);
    }
}
