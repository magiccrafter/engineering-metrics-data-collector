use graphql_client::{reqwest::post_graphql, GraphQLQuery};

#[derive(Debug, Clone)]
pub struct GitlabGraphQLClient {
    client: reqwest::Client,
}

impl GitlabGraphQLClient {
    const ENDPOINT: &'static str = "https://gitlab.com/api/graphql";

    pub async fn new(authorization_header: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("engineering-metrics-data-collector")
            .default_headers(
                std::iter::once((
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&authorization_header).unwrap(),
                ))
                .collect(),
            )
            .build()
            .unwrap();

        GitlabGraphQLClient { client }
    }

    pub async fn fetch_group_merge_requests(
        &self,
        group_full_path: &str,
        updated_after: &str,
        after_pointer_token: Option<String>,
    ) -> Result<group_merge_reqs::GroupMergeReqsGroup, Box<dyn std::error::Error>> {
        let variables = group_merge_reqs::Variables {
            group_full_path: group_full_path.to_string(),
            updated_after: updated_after.to_string(),
            after: after_pointer_token,
        };

        let response = post_graphql::<GroupMergeReqs, _>(&self.client, GitlabGraphQLClient::ENDPOINT, variables).await?;

        let response_data = response.data.expect("missing response data");
        let group_data = response_data.group.unwrap();

        Ok(group_data)
    }
}

type Time = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gitlab_group_mrs_schema.graphql",
    query_path = "src/gitlab_group_mrs_query.graphql",
    response_derives = "Debug"
)]
struct GroupMergeReqs;