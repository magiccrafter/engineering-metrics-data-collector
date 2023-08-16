#[derive(Debug, Clone)]
pub struct GitlabRestClient {
    client: reqwest::Client,
}

impl GitlabRestClient {
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

        GitlabRestClient { client }
    }

    pub async fn fetch_closed_issues_on_merge(
        &self,
        gitlab_rest_endpoint: &str,
        project_id: &str,
        merge_request_id: &str,
        merge_request_iid: &str,
    ) -> Result<Vec<ClosedIssueOnMerge>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/closes_issues",
            gitlab_rest_endpoint, project_id, merge_request_iid
        );
        let res = &self.client.get(&url).send().await?.text().await?;
        let issues: Vec<Issue> = serde_json::from_str(&res)?;
        let result = issues
            .iter()
            .map(|issue| ClosedIssueOnMerge {
                merge_request_id: merge_request_id.to_string(),
                merge_request_iid: merge_request_iid.to_string(),
                issue_id: issue.id.to_string(),
                issue_iid: issue.iid.to_string(),
                project_id: project_id.to_string(),
            })
            .collect();
        Ok(result)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Issue {
    id: usize,
    iid: usize,
}

pub struct ClosedIssueOnMerge {
    pub merge_request_id: String,
    pub merge_request_iid: String,
    pub issue_id: String,
    pub issue_iid: String,
    pub project_id: String,
}

