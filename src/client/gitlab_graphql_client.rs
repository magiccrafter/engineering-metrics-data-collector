use graphql_client::GraphQLQuery;
use serde::Serialize;
use thiserror::Error;

/// Custom post_graphql helper that works with any reqwest version.
/// This decouples us from graphql_client's reqwest dependency.
async fn post_graphql<Q: GraphQLQuery>(
    client: &reqwest::Client,
    url: &str,
    variables: Q::Variables,
) -> Result<graphql_client::Response<Q::ResponseData>, reqwest::Error>
where
    Q::Variables: Serialize,
{
    let body = Q::build_query(variables);
    client.post(url).json(&body).send().await?.json().await
}

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
    #[error("Invalid authorization header: {0}")]
    InvalidHeader(String),
}

#[derive(Debug, Clone)]
pub struct GitlabGraphQLClient {
    client: reqwest::Client,
    url: String,
}

impl GitlabGraphQLClient {
    pub fn new(authorization_header: &str, url: String) -> Result<Self, GitlabGraphQLError> {
        let header_value = reqwest::header::HeaderValue::from_str(authorization_header)
            .map_err(|e| GitlabGraphQLError::InvalidHeader(e.to_string()))?;
        let client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .default_headers(
                std::iter::once((reqwest::header::AUTHORIZATION, header_value)).collect(),
            )
            .build()?;

        Ok(GitlabGraphQLClient { client, url })
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

        let response = post_graphql::<GroupMergeReqs>(&self.client, &self.url, variables).await?;

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

        let response = post_graphql::<GroupProjects>(&self.client, &self.url, variables).await?;

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
