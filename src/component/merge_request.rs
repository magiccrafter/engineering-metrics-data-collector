use crate::client::gitlab_graphql_client::GitlabGraphQLError;
use crate::client::gitlab_rest_client::GitlabRestError;
use crate::component::import_progress::ImportProgressHandler;
use crate::context::GitlabContext;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client as GenAiClient, ModelIden, ServiceTarget};
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Error, Debug)]
pub enum MergeRequestError {
    #[error("GitLab GraphQL error: {0}")]
    GraphQLError(#[from] GitlabGraphQLError),
    #[error("GitLab REST error: {0}")]
    RestError(#[from] GitlabRestError),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Date parsing error: {0}")]
    DateParseError(#[from] time::error::Parse),
    #[error("Missing data: {0}")]
    MissingData(String),
}

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
    pub total_count: i32,
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
    ) -> Result<MergeRequestsWithPageInfo, MergeRequestError> {
        let group_data = self
            .context
            .gitlab_graphql_client
            .fetch_group_merge_requests(group_full_path, updated_after, after_pointer_token)
            .await?;

        let mut merge_requests: Vec<MergeRequest> = Vec::new();
        let nodes = group_data.merge_requests.nodes.ok_or_else(|| {
            MergeRequestError::MissingData(
                "GroupMergeReqsGroupMergeRequestsNodes is None".to_string(),
            )
        })?;

        for mr in nodes {
            let mr_ref = mr
                .as_ref()
                .ok_or_else(|| MergeRequestError::MissingData("mr is None".to_string()))?;
            merge_requests.push(MergeRequest {
                mr_id: mr_ref.id.clone(),
                mr_iid: mr_ref.iid.clone(),
                mr_title: mr_ref.title.clone(),
                mr_description: mr_ref.description.clone(),
                mr_web_url: mr_ref.web_url.clone(),
                project_id: mr_ref.project_id.clone().to_string(),
                project_name: mr_ref.project.name.clone(),
                project_path: mr_ref.project.path.clone(),
                created_at: OffsetDateTime::parse(&mr_ref.created_at.clone(), &Rfc3339)?,
                updated_at: OffsetDateTime::parse(&mr_ref.updated_at.clone(), &Rfc3339)?,
                merged_at: mr_ref
                    .merged_at
                    .clone()
                    .map(|m_at| OffsetDateTime::parse(m_at.as_str(), &Rfc3339))
                    .transpose()?,
                created_by: mr_ref.author.username.clone(),
                merged_by: mr_ref.merge_user.as_ref().map(|m_by| m_by.username.clone()),
                approved_by: mr_ref.approved_by.as_ref().map(|a_by| {
                    a_by.nodes
                        .as_ref()
                        .map(|nodes| {
                            nodes
                                .iter()
                                .filter_map(|a_by_node| {
                                    a_by_node.as_ref().map(|node| node.username.clone())
                                })
                                .collect()
                        })
                        .unwrap_or_default()
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
                        .map(|nodes| {
                            nodes
                                .iter()
                                .filter_map(|label| label.as_ref().map(|l| l.title.clone()))
                                .collect()
                        })
                        .unwrap_or_default()
                }),
                mr_ai_title: None,
                mr_ai_summary: None,
                mr_ai_model: None,
                mr_ai_category: None,
            });
        }

        Ok(MergeRequestsWithPageInfo {
            merge_requests,
            page_info: PageInfo {
                end_cursor: group_data.merge_requests.page_info.end_cursor,
                has_next_page: group_data.merge_requests.page_info.has_next_page,
            },
            total_count: group_data.merge_requests.count as i32,
        })
    }

    pub async fn persist_merge_request(
        &self,
        merge_request: &MergeRequest,
    ) -> Result<(), MergeRequestError> {
        let mut conn = self.context.store.conn_pool.acquire().await?;

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
            .bind(serde_json::to_value(&merge_request.approved_by)?)
            .bind(serde_json::to_value(&merge_request.diff_stats_summary)?)
            .bind(serde_json::to_value(&merge_request.labels)?)
            .bind(&merge_request.mr_ai_title)
            .bind(&merge_request.mr_ai_summary)
            .bind(&merge_request.mr_ai_model)
            .bind(&merge_request.mr_ai_category)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    pub async fn import_merge_requests(
        &self,
        group_full_path: &str,
        updated_after: &str,
    ) -> Result<(), MergeRequestError> {
        // Parse updated_after timestamp for filtering merged MRs
        let updated_after_time = OffsetDateTime::parse(updated_after, &Rfc3339)?;

        // Initialize import progress handler
        let import_progress_handler = ImportProgressHandler {
            store: self.context.store.clone(),
        };

        // Get or resume existing import
        let import_progress = import_progress_handler
            .get_or_create_import(group_full_path, "merge_requests", updated_after_time)
            .await
            .map_err(|e| MergeRequestError::DatabaseError(sqlx::Error::Protocol(e.to_string())))?;

        let mut has_more_merge_requests = true;
        let mut after_pointer_token = import_progress.last_cursor.clone();
        let mut total_imported = import_progress.total_processed;
        let mut total_count: Option<i32> = None;

        // Get AI configuration from context
        let ai_base_url = self.context.ai_base_url.clone();
        let ai_model = self.context.ai_model.clone();
        let ai_api_key = self.context.ai_api_key.clone();

        // Configure GenAI client with custom ServiceTargetResolver
        let ai_base_url_clone = ai_base_url.clone();
        let ai_api_key_clone = ai_api_key.clone();

        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                let ServiceTarget { model, .. } = service_target;
                let endpoint = Endpoint::from_owned(ai_base_url_clone.clone());
                let auth = AuthData::from_single(ai_api_key_clone.clone());
                let model = ModelIden::new(AdapterKind::Ollama, model.model_name);
                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model,
                })
            },
        );

        let ai_client = GenAiClient::builder()
            .with_service_target_resolver(target_resolver)
            .build();

        while has_more_merge_requests {
            let res = match self
                .fetch_group_merge_requests(
                    group_full_path,
                    updated_after,
                    after_pointer_token.clone(),
                )
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    // Mark the import as failed but preserve the cursor for resume
                    let _ = import_progress_handler
                        .mark_failed(import_progress.id, &e.to_string())
                        .await;
                    eprintln!(
                        "Failed to fetch merge requests for group {}: {}. Progress saved at cursor: {:?}",
                        group_full_path, e, after_pointer_token
                    );
                    return Err(e);
                }
            };

            let batch_count = res.merge_requests.len();

            // Capture total count on first fetch (it's the same for all pages)
            if total_count.is_none() {
                total_count = Some(res.total_count);
                println!(
                    "Total merge requests to process for group={}: {}",
                    group_full_path, res.total_count
                );
            }

            println!(
                "Fetched {} merge requests in this batch for group={}",
                batch_count, group_full_path
            );

            let mut batch_processed = 0;
            for mut merge_request in res.merge_requests {
                // Only process MRs that were merged after the updated_after time
                // This ensures we don't re-process old MRs that were just updated (e.g., commented on)
                match merge_request.merged_at {
                    Some(merged_at) if merged_at >= updated_after_time => {}
                    _ => continue,
                };

                // Generate AI summary for merged MRs
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
                            merge_request.mr_web_url, e
                        );
                    }
                }

                if let Err(e) = self.persist_merge_request(&merge_request).await {
                    eprintln!(
                        "Failed to persist merge request {}: {}",
                        merge_request.mr_iid, e
                    );
                }
                batch_processed += 1;
                total_imported += 1;
            }

            // Update progress after each batch - this is our checkpoint
            let next_cursor = res.page_info.end_cursor.as_deref();
            if let Err(e) = import_progress_handler
                .update_progress(import_progress.id, next_cursor, batch_processed)
                .await
            {
                eprintln!("Failed to update import progress: {}", e);
            }

            // Display progress with total count if available
            match total_count {
                Some(total) => println!(
                    "Progress: {}/{} merge requests processed for group={}",
                    total_imported, total, group_full_path
                ),
                None => println!(
                    "Progress: {} merge requests processed for group={}",
                    total_imported, group_full_path
                ),
            }

            after_pointer_token = res.page_info.end_cursor;
            has_more_merge_requests = res.page_info.has_next_page;
        }

        // Mark import as completed
        if let Err(e) = import_progress_handler
            .mark_completed(import_progress.id)
            .await
        {
            eprintln!("Failed to mark import as completed: {}", e);
        }

        println!(
            "Done importing merge requests data for group={}. Total imported: {}",
            &group_full_path, total_imported
        );

        Ok(())
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

        // Convert changes to string representation, limited to configured max characters
        let mut changes_diff = String::new();
        let max_chars = self.context.ai_max_context_chars;

        for change in changes.iter() {
            let file_diff = format!("File: {}\nDiff:\n{}\n\n", change.new_path, change.diff);
            if changes_diff.len() + file_diff.len() > max_chars {
                // Add as much as we can from this file
                let remaining = max_chars.saturating_sub(changes_diff.len());
                if remaining > 0 {
                    changes_diff.push_str(&file_diff[..remaining.min(file_diff.len())]);
                }
                break;
            }
            changes_diff.push_str(&file_diff);
        }

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

        const MAX_RETRIES: u32 = 3;
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;

        for attempt in 1..=MAX_RETRIES {
            let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt.clone())]);

            let response = match ai_client.exec_chat(ai_model, chat_req, None).await {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!(
                        "AI request failed (attempt {}/{}): {}",
                        attempt, MAX_RETRIES, e
                    );
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let content = response.content.texts().join("\n");

            // Clean markdown code blocks if present
            let clean_content = content
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            match serde_json::from_str::<AiResponse>(clean_content) {
                Ok(ai_resp) => {
                    return Ok((ai_resp.title, ai_resp.summary, ai_resp.category));
                }
                Err(e) => {
                    eprintln!(
                        "Failed to parse AI response as JSON (attempt {}/{}): {}. Response: {}",
                        attempt, MAX_RETRIES, e, clean_content
                    );
                    last_error = Some(Box::new(e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Box::new(std::io::Error::other(
                "AI summary generation failed after all retries",
            ))
        }))
    }
}
