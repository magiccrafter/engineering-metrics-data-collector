use serde::Deserialize;

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

    pub async fn fetch_jira_issue(&self, issue_key: &str, jira_rest_endpoint: &str) -> Result<JiraIssue, reqwest::Error> {
        let url = format!("{}/issue/{}", jira_rest_endpoint, issue_key);
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