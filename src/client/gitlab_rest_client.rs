use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitlabRestError {
    #[error("Failed to execute REST request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Invalid authorization header: {0}")]
    InvalidHeader(String),
}

#[derive(Debug, Clone)]
pub struct GitlabRestClient {
    client: reqwest::Client,
    endpoint: String,
}

impl GitlabRestClient {
    pub fn new(authorization_header: &str, endpoint: String) -> Result<Self, GitlabRestError> {
        let header_value = reqwest::header::HeaderValue::from_str(authorization_header)
            .map_err(|e| GitlabRestError::InvalidHeader(e.to_string()))?;
        let client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .default_headers(
                std::iter::once((reqwest::header::AUTHORIZATION, header_value)).collect(),
            )
            .build()?;

        Ok(GitlabRestClient { client, endpoint })
    }

    pub async fn fetch_merge_request_changes(
        &self,
        project_id: &str,
        merge_request_iid: &str,
    ) -> Result<Vec<Change>, GitlabRestError> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/changes",
            self.endpoint, project_id, merge_request_iid
        );
        let res = self.client.get(&url).send().await?;
        let text = res.text().await?;
        let mr_with_changes: MergeRequestResponseWithChanges = serde_json::from_str(&text)?;
        Ok(mr_with_changes.changes.unwrap_or_default())
    }
}

#[derive(Debug, Deserialize)]
pub struct MergeRequestResponseWithChanges {
    pub changes: Option<Vec<Change>>,
}

#[derive(Debug, Deserialize)]
pub struct Change {
    pub diff: String,
    pub new_path: String,
    pub old_path: String,
}
