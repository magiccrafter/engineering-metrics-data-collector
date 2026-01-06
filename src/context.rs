use crate::{
    client::{gitlab_graphql_client::GitlabGraphQLClient, gitlab_rest_client::GitlabRestClient},
    store::Store,
};

#[derive(Debug, Clone)]
pub struct GitlabContext {
    pub store: Store,
    pub gitlab_graphql_client: GitlabGraphQLClient,
    pub gitlab_rest_client: GitlabRestClient,
}
