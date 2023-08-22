use graphql_client::{reqwest::post_graphql, GraphQLQuery};

#[derive(Debug, Clone)]
pub struct GitlabGraphQLClient {
    client: reqwest::Client,
}

impl GitlabGraphQLClient {
    
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

        GitlabGraphQLClient { client }
    }

    pub async fn fetch_group_merge_requests(
        &self,
        gitlab_graphql_endpoint: &str,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> group_merge_reqs::GroupMergeReqsGroup {
        let variables = group_merge_reqs::Variables {
            group_full_path: group_full_path.to_string(),
            updated_after: updated_after.to_string(),
            after: after_pointer_token,
        };
        
        // let qraphql_query = include_str!("gitlab_group_mrs_query.graphql");
        // println!("{qraphql_query}");

        let response = post_graphql::<GroupMergeReqs, _>(&self.client, gitlab_graphql_endpoint, variables).await.expect("failed to execute graphql query");

        let response_data = response.data.expect("missing response data");
        response_data.group.unwrap()
    }

    pub async fn fetch_group_projects(
        &self,
        gitlab_graphql_endpoint: &str,
        group_full_path: &str,
        after_pointer_token: Option<String>,
    ) -> group_projects::GroupProjectsGroup {
        let variables = group_projects::Variables {
            group_full_path: group_full_path.to_string(),
            after: after_pointer_token,
        };

        // let qraphql_query = include_str!("gitlab_group_projects_query.graphql");
        // println!("{qraphql_query}");
        
        let response = post_graphql::<GroupProjects, _>(&self.client, gitlab_graphql_endpoint, variables).await.expect("failed to execute graphql query");

        let response_data = response.data.expect("missing response data");
        response_data.group.unwrap()
    }

    pub async fn fetch_group_issues(
        &self,
        gitlab_graphql_endpoint: &str,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> group_issues::GroupIssuesGroup {
        let variables = group_issues::Variables {
            group_full_path: group_full_path.to_string(),
            updated_after: updated_after.to_string(),
            after: after_pointer_token,
        };
        
        let response = post_graphql::<GroupIssues, _>(&self.client, gitlab_graphql_endpoint, variables).await.expect("failed to execute graphql query");

        let response_data = response.data.expect("missing response data");
        response_data.group.unwrap()
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