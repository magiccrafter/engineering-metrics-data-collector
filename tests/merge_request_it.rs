use engineering_metrics_data_collector::store::Store;
use engineering_metrics_data_collector::component::merge_request;
use testcontainers::clients;
mod postgres_container;

use sqlx::Row;
use serde_json::json;
use reqwest::Client;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wiremock::matchers::{body_json, method, path, body_string};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn should_successfully_import_a_single_merge_request_from_gitlab_to_the_database() {
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
    merge_request::import_merge_requests(&mock_server.uri(), DUMMY, DUMMY, DUMMY, &store).await;

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let result = sqlx::query("SELECT mr_id, mr_title, project_id, created_at, merged_at
        FROM engineering_metrics.merge_requests")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 2);

    // fetch concrete merge request with id equal to gid://gitlab/MergeRequest/221742778
    let result = sqlx::query("SELECT mr_id, mr_title, project_id, created_at, merged_at
        FROM engineering_metrics.merge_requests
        WHERE mr_id = 'gid://gitlab/MergeRequest/221742778'")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.get::<String, _>("mr_id"), "gid://gitlab/MergeRequest/221742778");
    assert_eq!(result.get::<String, _>("mr_title"), "Resolve \"pipeline check\"");
    assert_eq!(result.get::<String, _>("project_id"), "52263413");
    assert_eq!(result.get::<OffsetDateTime, _>("created_at"), OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap());
    assert_eq!(result.get::<OffsetDateTime, _>("merged_at"), OffsetDateTime::parse("2020-03-02T09:20:00Z", &Rfc3339).unwrap());

    // fetch concrete merge request with id equal to gid://gitlab/MergeRequest/221706264
    let result = sqlx::query("SELECT mr_id, mr_title, project_id, created_at, merged_at
        FROM engineering_metrics.merge_requests
        WHERE mr_id = 'gid://gitlab/MergeRequest/221706264'")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.get::<String, _>("mr_id"), "gid://gitlab/MergeRequest/221706264");
    assert_eq!(result.get::<String, _>("mr_title"), "Resolve \"Increase the size of login session cache\"");
    assert_eq!(result.get::<String, _>("project_id"), "52263413");
    assert_eq!(result.get::<OffsetDateTime, _>("created_at"), OffsetDateTime::parse("2020-03-02T09:30:00Z", &Rfc3339).unwrap());
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("merged_at"), Option::None);
}

#[tokio::test]
async fn should_persist_and_select_one_mr_successfully() {
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

    let mut conn = store.conn_pool.acquire().await.unwrap();

    let mr = merge_request::MergeRequest {
        mr_id: "gitlab/1".to_string(),
        mr_title: "awesome issue".to_string(),
        project_id: "gitlab/1".to_string(),
        created_at: OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap(),
        merged_at: Option::None,
    };

    merge_request::persist_merge_request(&store, &mr).await;

    let result = sqlx::query("SELECT mr_id, mr_title, project_id, created_at, merged_at
        FROM engineering_metrics.merge_requests")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 1);

    let result = sqlx::query("SELECT mr_id, mr_title, project_id, created_at, merged_at
        FROM engineering_metrics.merge_requests")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    assert_eq!(result.get::<String, _>("mr_id"), "gitlab/1");
    assert_eq!(result.get::<String, _>("mr_title"), "awesome issue");
    assert_eq!(result.get::<String, _>("project_id"), "gitlab/1");
    assert_eq!(result.get::<OffsetDateTime, _>("created_at"), OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap());
    assert_eq!(result.get::<Option<OffsetDateTime>, _>("merged_at"), Option::None);
}

#[tokio::test]
async fn should_fetch_from_gitlab_graphql_successfully() {
    let mock_server = MockServer::start().await;
    let expected_body = get_graphql_query_response_mock().await;
    // let request_body = json!({ "query": "query { group(fullPath: \"gitlab\") { mergeRequests { nodes { id title } } } }" });

    Mock::given(method("POST"))
        // .and(path("/graphql"))
        // .and(body_string(request_body.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(expected_body))
        .mount(&mock_server)
        .await;

    let mut resp = surf::post(&mock_server.uri()).await.unwrap();
    println!("response body: {:?}, status: {:?}", &mut resp.body_string().await.unwrap(), resp.status());
    assert_eq!(resp.status(), 200);

    const DUMMY: &String = &String::new();
    let result = merge_request::fetch_group_merge_requests(&mock_server.uri(), DUMMY, DUMMY, DUMMY, Option::None).await;
    assert_eq!(result.merge_requests.len(), 2);
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
                "mergeRequests": {
                    "nodes": [{
                        "id": "gid://gitlab/MergeRequest/221742778",
                        "title": "Resolve \"pipeline check\"",
                        "draft": false,
                        "webUrl": "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/221742778",
                        "labels": {
                            "nodes": [{
                                "title": "engineering"
                            }]
                        },
                        "approved": true,
                        "approvedBy": {
                            "nodes": [{
                                "id": "gid://gitlab/User/2",
                                "username": "dev2"
                            }]
                        },
                        "author": {
                            "id": "gid://gitlab/User/1",
                            "username": "dev1"
                        },
                        "createdAt": "2020-03-02T09:00:00Z",
                        "updatedAt": "2020-03-02T09:10:00Z",
                        "mergedAt": "2020-03-02T09:20:00Z",
                        "projectId": 52263413,
                        "diffStatsSummary": {
                            "additions": 2,
                            "deletions": 2,
                            "changes": 4,
                            "fileCount": 1
                        },
                        "mergeUser": {
                            "id": "gid://gitlab/User/1",
                            "username": "dev1"
                        },
                        "state": "merged"
                    }, {
                        "id": "gid://gitlab/MergeRequest/221706264",
                        "title": "Resolve \"Increase the size of login session cache\"",
                        "draft": false,
                        "webUrl": "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/221706264",
                        "labels": {
                            "nodes": [{
                                "title": "product"
                            }]
                        },
                        "approved": true,
                        "approvedBy": {
                            "nodes": []
                        },
                        "author": {
                            "id": "gid://gitlab/User/3",
                            "username": "dev3"
                        },
                        "createdAt": "2020-03-02T09:30:00Z",
                        "updatedAt": "2020-03-02T09:40:00Z",
                        "mergedAt": null,
                        "projectId": 52263413,
                        "diffStatsSummary": {
                            "additions": 8,
                            "deletions": 6,
                            "changes": 14,
                            "fileCount": 2
                        },
                        "mergeUser": null,
                        "state": "opened"
                    }],
                    "pageInfo": {
                        "endCursor": "eyJjcmVhdGVkX2F0IjoiMjAyMy0wNS0yMyAwODoxNjo0MS40NTQ1MTQwMDAgKzAwMDAiLCJpZCI6IjIyNTQ3NzIxMSJ9",
                        "hasNextPage": false
                    }
                }
            }
        }
    }
    "#;
}

// curl "https://gitlab.com/api/graphql" --header "Authorization: Bearer TODO" \
//     --header "Content-Type: application/json" --request POST \
//     --data '{"query": "query {group(fullPath: \"TODO\") {id name mergeRequests {nodes {id title}}}}"}'