use crate::{
    client::{
        atlassian_rest_client::AtlassianRestClient, gitlab_graphql_client::GitlabGraphQLClient,
        gitlab_rest_client::GitlabRestClient,
    },
    store::Store,
};

#[derive(Debug, Clone)]
pub struct GitlabContext {
    pub store: Store,
    pub gitlab_graphql_client: GitlabGraphQLClient,
    pub gitlab_rest_client: GitlabRestClient,
}

#[derive(Debug, Clone)]
pub struct AtlassianContext {
    pub store: Store,
    pub atlassian_jira_issue_url_prefix: String,
    pub atlassian_jira_rest_client: AtlassianRestClient,
}
