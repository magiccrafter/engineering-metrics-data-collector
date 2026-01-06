use crate::context::GitlabContext;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client as GenAiClient;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::env;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct MergeRequestHandler {
    pub context: GitlabContext,
}

#[derive(Debug)]
pub struct MergeRequest {
    pub mr_id: String,
    pub mr_iid: String,
    pub mr_title: String,
    pub mr_description: Option<String>,
    pub mr_web_url: String,
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub merged_at: Option<OffsetDateTime>,
    pub created_by: String,
    pub merged_by: Option<String>,
    pub approved: bool,
    pub approved_by: Option<Vec<String>>,
    pub diff_stats_summary: Option<DiffStatsSummary>,
    pub labels: Option<Vec<String>>,
    // AI fields
    pub mr_ai_title: Option<String>,
    pub mr_ai_summary: Option<String>,
    pub mr_ai_model: Option<String>,
    pub mr_ai_category: Option<String>,
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

#[derive(Deserialize)]
struct AiResponse {
    category: String,
    title: String,
    summary: String,
}

impl MergeRequestHandler {
    pub async fn fetch_group_merge_requests(
        &self,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> MergeRequestsWithPageInfo {
        let group_data = self
            .context
            .gitlab_graphql_client
            .fetch_group_merge_requests(group_full_path, updated_after, after_pointer_token)
            .await
            .expect("Failed to fetch group merge requests - check GitLab API credentials and group access permissions");

        let mut merge_requests: Vec<MergeRequest> = Vec::new();
        for mr in group_data
            .merge_requests
            .nodes
            .expect("GroupMergeReqsGroupMergeRequestsNodes is None")
        {
            let mr_ref = mr.as_ref().expect("mr is None");
            merge_requests.push(MergeRequest {
                mr_id: mr_ref.id.clone(),
                mr_iid: mr_ref.iid.clone(),
                mr_title: mr_ref.title.clone(),
                mr_description: mr_ref.description.clone(),
                mr_web_url: mr_ref.web_url.clone(),
                project_id: mr_ref.project_id.clone().to_string(),
                project_name: mr_ref.project.name.clone(),
                project_path: mr_ref.project.path.clone(),
                created_at: OffsetDateTime::parse(&mr_ref.created_at.clone(), &Rfc3339).unwrap(),
                updated_at: OffsetDateTime::parse(&mr_ref.updated_at.clone(), &Rfc3339).unwrap(),
                merged_at: mr_ref
                    .merged_at
                    .clone()
                    .map(|m_at| OffsetDateTime::parse(m_at.as_str(), &Rfc3339).unwrap()),
                created_by: mr_ref.author.username.clone(),
                merged_by: mr_ref.merge_user.as_ref().map(|m_by| m_by.username.clone()),
                approved_by: mr_ref.approved_by.as_ref().map(|a_by| {
                    a_by.nodes
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|a_by_node| {
                            a_by_node
                                .as_ref()
                                .expect("a_by_node is None")
                                .username
                                .clone()
                        })
                        .collect()
                }),
                approved: mr_ref.approved,
                diff_stats_summary: mr_ref
                    .diff_stats_summary
                    .as_ref()
                    .map(|diff_stats_summary| DiffStatsSummary {
                        additions: diff_stats_summary.additions as i32,
                        deletions: diff_stats_summary.deletions as i32,
                        changes: diff_stats_summary.changes as i32,
                        file_count: diff_stats_summary.file_count as i32,
                    }),
                labels: mr_ref.labels.as_ref().map(|labels| {
                    labels
                        .nodes
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|label| label.as_ref().expect("label is None").title.clone())
                        .collect()
                }),
                mr_ai_title: None,
                mr_ai_summary: None,
                mr_ai_model: None,
                mr_ai_category: None,
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

    pub async fn persist_merge_request(&self, merge_request: &MergeRequest) {
        let mut conn = self.context.store.conn_pool.acquire().await.unwrap();

        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.merge_requests (mr_id, mr_iid, mr_title, mr_web_url, project_id, project_name, project_path, 
                created_at, updated_at, merged_at, 
                created_by, merged_by, approved, approved_by, diff_stats_summary, labels, mr_ai_title, mr_ai_summary, mr_ai_model, mr_ai_category)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            ON CONFLICT (mr_id) DO 
            UPDATE SET 
                mr_iid = $2,
                mr_title = $3,
                updated_at = $9,
                merged_at = $10,
                merged_by = $12,
                approved = $13,
                approved_by = $14,
                diff_stats_summary = $15,
                labels = $16,
                mr_ai_title = COALESCE(EXCLUDED.mr_ai_title, engineering_metrics.merge_requests.mr_ai_title),
                mr_ai_summary = COALESCE(EXCLUDED.mr_ai_summary, engineering_metrics.merge_requests.mr_ai_summary),
                mr_ai_model = COALESCE(EXCLUDED.mr_ai_model, engineering_metrics.merge_requests.mr_ai_model),
                mr_ai_category = COALESCE(EXCLUDED.mr_ai_category, engineering_metrics.merge_requests.mr_ai_category)
            "#)
            .bind(&merge_request.mr_id)
            .bind(&merge_request.mr_iid)
            .bind(&merge_request.mr_title)
            .bind(&merge_request.mr_web_url)
            .bind(&merge_request.project_id)
            .bind(&merge_request.project_name)
            .bind(&merge_request.project_path)
            .bind(merge_request.created_at)
            .bind(merge_request.updated_at)
            .bind(merge_request.merged_at)
            .bind(&merge_request.created_by)
            .bind(&merge_request.merged_by)
            .bind(merge_request.approved)
            .bind(serde_json::to_value(&merge_request.approved_by).unwrap())
            .bind(serde_json::to_value(&merge_request.diff_stats_summary).unwrap())
            .bind(serde_json::to_value(&merge_request.labels).unwrap())
            .bind(&merge_request.mr_ai_title)
            .bind(&merge_request.mr_ai_summary)
            .bind(&merge_request.mr_ai_model)
            .bind(&merge_request.mr_ai_category)
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    pub async fn import_merge_requests(&self, group_full_path: &str, updated_after: &str) {
        let mut has_more_merge_requests = true;
        let mut after_pointer_token = Option::None;
        let mut total_imported = 0;

        let ai_client = GenAiClient::default();
        let ai_model = env::var("AI_MODEL").unwrap_or_else(|_| "llama3".to_string());
        // Note: AI_BASE_URL is handled by rust-genai env var OLLAMA_API_BASE_URL if set, or we can configure adapter.
        // Assuming user sets OLLAMA_API_BASE_URL or compatible env vars for rust-genai or we rely on default localhost.

        while has_more_merge_requests {
            let res = self
                .fetch_group_merge_requests(
                    group_full_path,
                    updated_after,
                    after_pointer_token.clone(),
                )
                .await;

            let batch_count = res.merge_requests.len();
            println!(
                "Fetched {} merge requests in this batch for group={}",
                batch_count, group_full_path
            );

            for mut merge_request in res.merge_requests {
                // Generate AI summary if merged
                if merge_request.merged_at.is_some() {
                    match self
                        .generate_ai_summary(&ai_client, &ai_model, &merge_request)
                        .await
                    {
                        Ok((title, summary, category)) => {
                            merge_request.mr_ai_title = Some(title);
                            merge_request.mr_ai_summary = Some(summary);
                            merge_request.mr_ai_category = Some(category);
                            merge_request.mr_ai_model = Some(ai_model.clone());
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to generate AI summary for MR {}: {}",
                                merge_request.mr_iid, e
                            );
                        }
                    }
                }

                self.persist_merge_request(&merge_request).await;
                total_imported += 1;
            }

            after_pointer_token = res.page_info.end_cursor;
            has_more_merge_requests = res.page_info.has_next_page;
        }
        println!(
            "Done importing merge requests data for group={}. Total imported: {}",
            &group_full_path, total_imported
        );
    }

    async fn generate_ai_summary(
        &self,
        ai_client: &GenAiClient,
        ai_model: &str,
        mr: &MergeRequest,
    ) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let changes = self
            .context
            .gitlab_rest_client
            .fetch_merge_request_changes(&mr.project_id, &mr.mr_iid)
            .await?;

        // Convert changes to string representation
        let changes_diff = changes
            .iter()
            .take(20) // Limit to 20 files to avoid huge context
            .map(|c| format!("File: {}\nDiff:\n{}", c.new_path, c.diff))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            r#"You are an expert Code Reviewer and Release Manager. Analyze the provided PR metadata (title, description, and code changes).

Your goal is to output a single valid JSON object containing a categorization, a summary, and a perfect conventional commit title.

**1. Analyze & Categorize**
Determine the category strictly from this list:
- Feature: Adds new functionality or business logic.
- Bugfix: Fixes incorrect behavior or errors.
- Refactor: Restructures code without changing external behavior.
- Platform: Changes to CI/CD, Docker, Build scripts, or Infrastructure as Code.
- Chore: Dependency updates, documentation, or minor maintenance.

**2. Generate Title (Conventional Commits)**
Generate a PR title following the format: `type(scope): description`.
- Map your chosen Category to a Type:
  - Feature -> feat
  - Bugfix -> fix
  - Refactor -> refactor
  - Platform -> chore (or 'ci' if applicable)
  - Chore -> chore (or 'docs'/'style' if specific)
- Scope: A short noun describing the section of the codebase (e.g., api, auth, ui).
- Description: Imperative mood ("add" not "added"), max 100 chars total, no trailing period.

**3. Generate Summary**
Write a concise summary (2-3 sentences) explaining *what* changed and *why*.

**Output Format**
Return ONLY a raw JSON object (no markdown formatting, no code blocks) with the following keys:
{{
  "category": "String (from list above)",
  "title": "String (Conventional Commit format)",
  "summary": "String"
}}

PR Title: {}
PR Description: {}
PR Changes:
{}"#,
            mr.mr_title,
            mr.mr_description.as_deref().unwrap_or(""),
            changes_diff
        );

        let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

        let response = ai_client.exec_chat(ai_model, chat_req, None).await?;
        let content = response.content.texts().join("\n");

        // Clean markdown code blocks if present
        let clean_content = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let ai_resp: AiResponse = serde_json::from_str(clean_content)?;

        Ok((ai_resp.title, ai_resp.summary, ai_resp.category))
    }
}
