use crate::{
    client::{
        copilot_usage_metrics_client::CopilotUsageMetricsClient,
        gitlab_graphql_client::GitlabGraphQLClient, gitlab_rest_client::GitlabRestClient,
    },
    store::Store,
};

#[derive(Debug, Clone)]
pub struct GitlabContext {
    pub store: Store,
    pub gitlab_graphql_client: GitlabGraphQLClient,
    pub gitlab_rest_client: GitlabRestClient,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_api_key: String,
    pub ai_max_context_chars: usize,
    pub upsert_merge_requests: bool,
}

#[derive(Debug, Clone)]
pub struct CopilotContext {
    pub store: Store,
    pub copilot_usage_metrics_client: CopilotUsageMetricsClient,
    pub github_api_version: String,
    pub report_lag_days: i64,
}
