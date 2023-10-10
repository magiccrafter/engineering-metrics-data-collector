use engineering_metrics_data_collector::client::gitlab_graphql_client::GitlabGraphQLClient;
use engineering_metrics_data_collector::client::gitlab_rest_client::GitlabRestClient;
use engineering_metrics_data_collector::component::project::ProjectHandler;
use engineering_metrics_data_collector::context::GitlabContext;
use engineering_metrics_data_collector::store::Store;
use testcontainers::clients;
mod postgres_container;

use serde_json::json;
use sqlx::Row;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn should_successfully_import_projects_from_gitlab_to_the_database() {
    let docker = clients::Cli::default();
    let image = postgres_container::Postgres::default();
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5432);

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;

    store.migrate().await.unwrap();

    let mock_server = MockServer::start().await;
    let expected_body = get_graphql_query_response_mock().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(expected_body))
        .mount(&mock_server)
        .await;
    const DUMMY: &String = &String::new();
    let project_handler = ProjectHandler {
        context: GitlabContext {
            store: store.clone(),
            gitlab_rest_client: GitlabRestClient::new(DUMMY, DUMMY.to_string()).await,
            gitlab_graphql_client: GitlabGraphQLClient::new(DUMMY, mock_server.uri()).await,
        },
    };

    project_handler.import_projects(DUMMY).await;

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let result = sqlx::query("SELECT p_id, p_name, p_path, p_full_path, p_web_url, topics from engineering_metrics.projects")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 2);

    // fetch concrete project that has p_path equal to "cool_project_1"
    let result = sqlx::query("SELECT p_id, p_name, p_path, p_full_path, p_web_url, topics from engineering_metrics.projects WHERE p_path = 'project-1'")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        result.get::<String, _>("p_id"),
        "gid://gitlab/Project/444444"
    );
    assert_eq!(result.get::<String, _>("p_name"), "Project 1 Name");
    assert_eq!(result.get::<String, _>("p_path"), "project-1");
    assert_eq!(
        result.get::<String, _>("p_full_path"),
        "full/path/to/project-1"
    );
    assert_eq!(
        result.get::<String, _>("p_web_url"),
        "https://gitlab.com/full/path/to/project-1"
    );
    assert_eq!(
        result.get::<serde_json::Value, _>("topics"),
        json!(["foo", "bar"])
    );

    // fetch concrete project that has p_path equal to "cool_project_2"
    let result = sqlx::query("SELECT p_id, p_name, p_path, p_full_path, p_web_url, topics from engineering_metrics.projects WHERE p_path = 'project-2'")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        result.get::<String, _>("p_id"),
        "gid://gitlab/Project/333333"
    );
    assert_eq!(result.get::<String, _>("p_name"), "project 2 name");
    assert_eq!(result.get::<String, _>("p_path"), "project-2");
    assert_eq!(
        result.get::<String, _>("p_full_path"),
        "full/path/to/project-2"
    );
    assert_eq!(
        result.get::<String, _>("p_web_url"),
        "https://gitlab.com/full/path/to/project-2"
    );
    assert_eq!(result.get::<serde_json::Value, _>("topics"), json!([]));
}

async fn get_graphql_query_response_mock() -> &'static str {
    return r#"
    {
        "data": {
          "queryComplexity": {
            "score": 27,
            "limit": 250
          },
          "group": {
            "id": "gid://gitlab/Group/123456",
            "name": "foo-group",
            "projects": {
              "nodes": [
                {
                  "id": "gid://gitlab/Project/444444",
                  "name": "Project 1 Name",
                  "path": "project-1",
                  "fullPath": "full/path/to/project-1",
                  "webUrl": "https://gitlab.com/full/path/to/project-1",
                  "topics": ["foo", "bar"]
                },
                {
                  "id": "gid://gitlab/Project/333333",
                  "name": "project 2 name",
                  "path": "project-2",
                  "fullPath": "full/path/to/project-2",
                  "webUrl": "https://gitlab.com/full/path/to/project-2",
                  "topics": []
                }
              ],
              "pageInfo": {
                "endCursor": "eyJpZCI7IjQ3MTE3ODAzIn1",
                "hasNextPage": false
              }
            }
          }
        }
      }
    "#;
}
