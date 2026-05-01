use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CopilotUsageMetricsError {
    #[error("Failed to execute REST request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Invalid authorization header: {0}")]
    InvalidHeader(String),
    #[error("GitHub API returned unexpected status {status}: {body}")]
    UnexpectedStatus { status: StatusCode, body: String },
}

#[derive(Debug, Clone)]
pub struct CopilotUsageMetricsClient {
    api_client: reqwest::Client,
    download_client: reqwest::Client,
    endpoint: String,
}

impl CopilotUsageMetricsClient {
    pub fn new(
        authorization_header: &str,
        endpoint: String,
        api_version: &str,
    ) -> Result<Self, CopilotUsageMetricsError> {
        let authorization_header_value = reqwest::header::HeaderValue::from_str(
            &normalize_authorization_header(authorization_header),
        )
        .map_err(|e| CopilotUsageMetricsError::InvalidHeader(e.to_string()))?;
        let api_version_header_value = reqwest::header::HeaderValue::from_str(api_version)
            .map_err(|e| CopilotUsageMetricsError::InvalidHeader(e.to_string()))?;
        let accept_header_value =
            reqwest::header::HeaderValue::from_static("application/vnd.github+json");

        let api_client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .default_headers(
                [
                    (reqwest::header::AUTHORIZATION, authorization_header_value),
                    (reqwest::header::ACCEPT, accept_header_value),
                    (
                        reqwest::header::HeaderName::from_static("x-github-api-version"),
                        api_version_header_value,
                    ),
                ]
                .into_iter()
                .collect(),
            )
            .build()?;

        let download_client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .build()?;

        Ok(Self {
            api_client,
            download_client,
            endpoint,
        })
    }

    pub async fn fetch_org_users_usage_report_for_day(
        &self,
        org_slug: &str,
        day: &str,
    ) -> Result<Option<CopilotUsersUsageReportDay>, CopilotUsageMetricsError> {
        let url = format!(
            "{}/orgs/{}/copilot/metrics/reports/users-1-day",
            self.endpoint, org_slug
        );

        let response = self
            .api_client
            .get(&url)
            .query(&[("day", day)])
            .send()
            .await?;
        let status = response.status();

        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        if !status.is_success() {
            let body = response.text().await?;
            return Err(CopilotUsageMetricsError::UnexpectedStatus { status, body });
        }

        Ok(Some(response.json::<CopilotUsersUsageReportDay>().await?))
    }

    pub async fn download_users_usage_report(
        &self,
        download_url: &str,
    ) -> Result<Vec<CopilotDailyUserMetricsRecord>, CopilotUsageMetricsError> {
        let response = self.download_client.get(download_url).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await?;
            return Err(CopilotUsageMetricsError::UnexpectedStatus { status, body });
        }

        let body = response.text().await?;
        let mut records = Vec::new();

        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }

            records.push(serde_json::from_str::<CopilotDailyUserMetricsRecord>(line)?);
        }

        Ok(records)
    }
}

fn normalize_authorization_header(authorization_header: &str) -> String {
    if authorization_header.starts_with("Bearer ") || authorization_header.starts_with("token ") {
        authorization_header.to_string()
    } else {
        format!("Bearer {}", authorization_header)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotUsersUsageReportDay {
    pub download_links: Vec<String>,
    pub report_day: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotDailyUserMetricsRecord {
    pub user_id: i64,
    pub user_login: String,
    pub day: String,
    pub organization_id: String,
    #[serde(default)]
    pub enterprise_id: Option<String>,
    #[serde(default)]
    pub user_initiated_interaction_count: i64,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub totals_by_ide: Vec<CopilotIdeMetrics>,
    #[serde(default)]
    pub totals_by_feature: Vec<CopilotFeatureMetrics>,
    #[serde(default)]
    pub totals_by_language_feature: Vec<CopilotLanguageFeatureMetrics>,
    #[serde(default)]
    pub totals_by_language_model: Vec<CopilotLanguageModelMetrics>,
    #[serde(default)]
    pub totals_by_model_feature: Vec<CopilotModelFeatureMetrics>,
    #[serde(default)]
    pub used_agent: bool,
    #[serde(default)]
    pub used_chat: bool,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
    #[serde(default)]
    pub used_cli: bool,
    #[serde(default)]
    pub totals_by_cli: Option<CopilotCliMetrics>,
    #[serde(default)]
    pub used_copilot_coding_agent: bool,
    #[serde(default)]
    pub used_copilot_cloud_agent: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotIdeMetrics {
    pub ide: String,
    #[serde(default)]
    pub user_initiated_interaction_count: i64,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
    #[serde(default)]
    pub last_known_plugin_version: Option<CopilotVersionSample>,
    #[serde(default)]
    pub last_known_ide_version: Option<CopilotIdeVersionSample>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotFeatureMetrics {
    pub feature: String,
    #[serde(default)]
    pub user_initiated_interaction_count: i64,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotLanguageFeatureMetrics {
    pub language: String,
    pub feature: String,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotLanguageModelMetrics {
    pub language: String,
    pub model: String,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModelFeatureMetrics {
    pub model: String,
    pub feature: String,
    #[serde(default)]
    pub user_initiated_interaction_count: i64,
    #[serde(default)]
    pub code_generation_activity_count: i64,
    #[serde(default)]
    pub code_acceptance_activity_count: i64,
    #[serde(default)]
    pub loc_suggested_to_add_sum: i64,
    #[serde(default)]
    pub loc_suggested_to_delete_sum: i64,
    #[serde(default)]
    pub loc_added_sum: i64,
    #[serde(default)]
    pub loc_deleted_sum: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotCliMetrics {
    #[serde(default)]
    pub session_count: i64,
    #[serde(default)]
    pub request_count: i64,
    #[serde(default)]
    pub prompt_count: i64,
    #[serde(default)]
    pub token_usage: Option<CopilotCliTokenUsage>,
    #[serde(default)]
    pub last_known_cli_version: Option<CopilotCliVersionSample>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotCliTokenUsage {
    #[serde(default)]
    pub output_tokens_sum: i64,
    #[serde(default)]
    pub prompt_tokens_sum: i64,
    #[serde(default)]
    pub avg_tokens_per_request: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotVersionSample {
    pub sampled_at: String,
    pub plugin: String,
    pub plugin_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotIdeVersionSample {
    pub sampled_at: String,
    pub ide_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotCliVersionSample {
    pub sampled_at: String,
    pub cli_version: String,
}
