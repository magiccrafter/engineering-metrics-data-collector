use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitlabGraphQLError {
    #[error("Failed to execute GraphQL query: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("GraphQL errors: {0:?}")]
    GraphQLErrors(Vec<String>),
    #[error(
        "Missing response data (no data returned from API - check authentication and permissions)"
    )]
    MissingData,
    #[error("Group not found: {0}")]
    GroupNotFound(String),
}

#[derive(Debug, Clone)]
pub struct GitlabGraphQLClient {
    client: reqwest::Client,
    url: String,
}

impl GitlabGraphQLClient {
    pub async fn new(authorization_header: &str, url: String) -> Self {
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

        GitlabGraphQLClient { client, url }
    }

    pub async fn fetch_group_merge_requests(
        &self,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> Result<group_merge_reqs::GroupMergeReqsGroup, GitlabGraphQLError> {
        let variables = group_merge_reqs::Variables {
            group_full_path: group_full_path.to_string(),
            updated_after: updated_after.to_string(),
            after: after_pointer_token,
        };

        // let qraphql_query = include_str!("gitlab_group_mrs_query.graphql");
        // println!("{qraphql_query}");

        let response =
            post_graphql::<GroupMergeReqs, _>(&self.client, &self.url, variables).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(GitlabGraphQLError::GraphQLErrors(
                    errors.iter().map(|e| e.message.clone()).collect(),
                ));
            }
        }

        let response_data = response.data.ok_or(GitlabGraphQLError::MissingData)?;
        response_data
            .group
            .ok_or_else(|| GitlabGraphQLError::GroupNotFound(group_full_path.to_string()))
    }

    pub async fn fetch_group_projects(
        &self,
        group_full_path: &str,
        after_pointer_token: Option<String>,
    ) -> Result<group_projects::GroupProjectsGroup, GitlabGraphQLError> {
        let variables = group_projects::Variables {
            group_full_path: group_full_path.to_string(),
            after: after_pointer_token,
        };

        // let qraphql_query = include_str!("gitlab_group_projects_query.graphql");
        // println!("{qraphql_query}");

        let response = post_graphql::<GroupProjects, _>(&self.client, &self.url, variables).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(GitlabGraphQLError::GraphQLErrors(
                    errors.iter().map(|e| e.message.clone()).collect(),
                ));
            }
        }

        let response_data = response.data.ok_or(GitlabGraphQLError::MissingData)?;
        response_data
            .group
            .ok_or_else(|| GitlabGraphQLError::GroupNotFound(group_full_path.to_string()))
    }

    pub async fn fetch_group_issues(
        &self,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> Result<group_issues::GroupIssuesGroup, GitlabGraphQLError> {
        let variables = group_issues::Variables {
            group_full_path: group_full_path.to_string(),
            updated_after: updated_after.to_string(),
            after: after_pointer_token,
        };

        let response = post_graphql::<GroupIssues, _>(&self.client, &self.url, variables).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(GitlabGraphQLError::GraphQLErrors(
                    errors.iter().map(|e| e.message.clone()).collect(),
                ));
            }
        }

        let response_data = response.data.ok_or(GitlabGraphQLError::MissingData)?;
        response_data
            .group
            .ok_or_else(|| GitlabGraphQLError::GroupNotFound(group_full_path.to_string()))
    }
}

type Time = String;

#[derive(GraphQLQuery, Clone)]
#[graphql(
    schema_path = "src/client/gitlab_group_mrs_schema.graphql",
    query_path = "src/client/gitlab_group_mrs_query.graphql",
    response_derives = "Debug"
)]
struct GroupMergeReqs;

#[derive(GraphQLQuery, Clone)]
#[graphql(
    schema_path = "src/client/gitlab_group_projects_schema.graphql",
    query_path = "src/client/gitlab_group_projects_query.graphql",
    response_derives = "Debug"
)]
struct GroupProjects;

#[derive(GraphQLQuery, Clone)]
#[graphql(
    schema_path = "src/client/gitlab_group_issues_schema.graphql",
    query_path = "src/client/gitlab_group_issues_query.graphql",
    response_derives = "Debug"
)]
struct GroupIssues;
