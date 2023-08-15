use engineering_metrics_data_collector::store::Store;
use engineering_metrics_data_collector::component::issue::{self};
use testcontainers::clients;
mod postgres_container;

use sqlx::Row;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wiremock::matchers::{method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn should_successfully_import_issues_from_gitlab_to_the_database() {
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
    issue::import_issues(&mock_server.uri(), DUMMY, DUMMY, DUMMY, &store).await;

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let result = sqlx::query("SELECT issue_id, issue_title, issue_web_url, project_id, created_at, closed_at
        FROM engineering_metrics.issues")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 2);

    // fetch concrete issue that is merged with id equal to gid://gitlab/Issue/111122223
    let result = sqlx::query("SELECT issue_id, issue_title, issue_web_url, project_id, created_at, created_by, 
            updated_at, updated_by, closed_at, labels
        FROM engineering_metrics.issues
        WHERE issue_id = 'gid://gitlab/Issue/111122223'")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.get::<String, _>("issue_id"), "gid://gitlab/Issue/111122223");
    assert_eq!(result.get::<String, _>("issue_title"), "fancy title 1");
    assert_eq!(result.get::<String, _>("issue_web_url"), "https://gitlab.com/group-1/group-2/project-1/-/issues/27");
    assert_eq!(result.get::<String, _>("project_id"), "40000011");
    assert_eq!(result.get::<OffsetDateTime, _>("created_at"), OffsetDateTime::parse("2023-08-15T13:27:19Z", &Rfc3339).unwrap());
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("updated_at"), Some(OffsetDateTime::parse("2023-08-15T13:37:28Z", &Rfc3339).unwrap()));
    assert_eq!(result.get::<Option<String>, _>("created_by"), Some("alice".to_string()));
    assert_eq!(result.get::<Option<String>, _>("updated_by"), Some("bob".to_string()));
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("closed_at"), Option::None);
    assert_eq!(result.get::<Option<serde_json::Value>, _>("labels"), Some(json!([])));
   

    // fetch concrete issue that is merged with id equal to gid://gitlab/Issue/111122224
    let result = sqlx::query("SELECT issue_id, issue_title, issue_web_url, project_id, created_at, created_by, 
            updated_at, updated_by, closed_at, labels
        FROM engineering_metrics.issues
        WHERE issue_id = 'gid://gitlab/Issue/111122224'")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.get::<String, _>("issue_id"), "gid://gitlab/Issue/111122224");
    assert_eq!(result.get::<String, _>("issue_title"), "fancy title 2");
    assert_eq!(result.get::<String, _>("issue_web_url"), "https://gitlab.com/group-1/group-3/project-5/-/issues/46");
    assert_eq!(result.get::<String, _>("project_id"), "40000012");
    assert_eq!(result.get::<OffsetDateTime, _>("created_at"), OffsetDateTime::parse("2023-08-15T13:16:48Z", &Rfc3339).unwrap());
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("updated_at"), Some(OffsetDateTime::parse("2023-08-15T13:32:39Z", &Rfc3339).unwrap()));
    assert_eq!(result.get::<Option<String>, _>("created_by"), Some("alice".to_string()));
    assert_eq!(result.get::<Option<String>, _>("updated_by"), Option::None);
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("closed_at"), Option::None);
    assert_eq!(result.get::<Option<serde_json::Value>, _>("labels"), Some(json!([])));
}

async fn get_graphql_query_response_mock() -> &'static str {
    return r#"
    {
        "data": {
            "queryComplexity": {
                "score": 50,
                "limit": 250
            },
            "group": {
                "id": "gid://gitlab/Group/52263413",
                "name": "cool_group",
                "issues": {
                    "nodes": [
                      {
                        "id": "gid://gitlab/Issue/111122223",
                        "createdAt": "2023-08-15T13:27:19Z",
                        "closedAt": null,
                        "projectId": 40000011,
                        "title": "fancy title 1",
                        "webUrl": "https://gitlab.com/group-1/group-2/project-1/-/issues/27",
                        "state": "opened",
                        "author": {
                          "username": "alice"
                        },
                        "updatedBy": {
                          "username": "bob"
                        },
                        "labels": {
                            "nodes": []
                        },
                        "updatedAt": "2023-08-15T13:37:28Z"
                      },
                      {
                        "id": "gid://gitlab/Issue/111122224",
                        "createdAt": "2023-08-15T13:16:48Z",
                        "closedAt": null,
                        "projectId": 40000012,
                        "title": "fancy title 2",
                        "webUrl": "https://gitlab.com/group-1/group-3/project-5/-/issues/46",
                        "state": "closed",
                        "author": {
                          "username": "alice"
                        },
                        "labels": {
                            "nodes": []
                        },
                        "updatedBy": null,
                        "updatedAt": "2023-08-15T13:32:39Z"
                      }
                    ],
                    "pageInfo": {
                      "endCursor": "eyJjcmVhdGVkX2F0IjoiMjAyMy0wOC0xNSAxMzoxNjo0OC45MDA2MjQwMDAgKzAwMDAiLCJpZCI6IjEzMjIyMTI3NCJ9",
                      "hasNextPage": false
                    }
                }
            }
        }
    }
    "#;
}