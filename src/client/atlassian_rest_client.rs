use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AtlassianRestClient {
    client: reqwest::Client,
}

impl AtlassianRestClient {
    pub async fn new(authorization_header: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .default_headers(
                std::iter::once((
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(authorization_header).unwrap(),
                ))
                .collect(),
            )
            .build()
            .unwrap();

        AtlassianRestClient { client }
    }

    async fn fetch_jira_issue(&self, issue_key: &str, jira_base_url: &str) -> Result<JiraIssue, reqwest::Error> {
        let url = format!("{}/rest/api/2/issue/{}", jira_base_url, issue_key);
        let response = &self.client
            .get(&url)
            .send()
            .await?
            .json::<JiraIssue>()
            .await?;
    
        Ok(response.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
}