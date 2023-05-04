use engineering_metrics_data_collector::store::Store;
use engineering_metrics_data_collector::component::merge_request;
use testcontainers::clients;
mod postgres_container;

use sqlx::Row;
use serde_json::json;
use reqwest::Client;
use wiremock::matchers::{body_json, method, path, body_string};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn should_successfully_import_a_single_merge_request_from_gitlab() {
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

    // mock the graphql client response with a single merge request
    
}

#[tokio::test]
async fn hello_reqwest() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let resp = Client::new().get(&mock_server.uri()).send().await.unwrap();

    assert_eq!(resp.status(), 200);
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
    };

    merge_request::persist_merge_request(&store, &mr).await;

    let result = sqlx::query("SELECT mr_id, mr_title, project_id
        FROM engineering_metrics.merge_requests")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 1);

    let result = sqlx::query("SELECT mr_id, mr_title, project_id
        FROM engineering_metrics.merge_requests")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    assert_eq!(result.get::<String, _>("mr_id"), "gitlab/1");
    assert_eq!(result.get::<String, _>("mr_title"), "awesome issue");
    assert_eq!(result.get::<String, _>("project_id"), "gitlab/1");

}

#[tokio::test]
async fn should_fetch_from_gitlab_graphql_successfully() {
    // let mock_server = MockServer::start().await;

    // let expected_body = r#"
    // {
    //     "data": {
    //         "group": {
    //             "mergeRequests": {
    //                 "nodes": [{
    //                     "id": "gid://gitlab/MergeRequest/221742778",
    //                     "title": "Resolve \"pipeline check\""
    //                 }, {
    //                     "id": "gid://gitlab/MergeRequest/221706264",
    //                     "title": "Resolve \"Increase the size of login session cache\""
    //                 }]
    //             }
    //         }
    //     }
    // }
    // "#;

    // let expected_body = json!({ "a": 1, "c": { "e": 2 } });
    // let request_body = json!({ "query": "query { group(fullPath: \"gitlab\") { mergeRequests { nodes { id title } } } }" });

    // Mock::given(method("POST"))
    //     // .and(path("/graphql"))
    //     // .and(body_string(expected_body.to_string()))
    //     .and(body_json(expected_body))
    //     .respond_with(ResponseTemplate::new(200))
    //     .mount(&mock_server)
    //     .await;

    // let status = surf::post(&mock_server.uri())
    // .await.unwrap().body_string().await.unwrap();

    // println!("status: {:?}", &status);

    // assert_eq!(status, "404".to_string());

    // let resp = Client::new().post(&mock_server.uri()).send().await.unwrap();
    // println!("resp: {:?}", resp);

    // const DUMMY: &String = &String::new();
    // let result = merge_request::fetch_group_merge_requests(&mock_server.uri(), DUMMY, DUMMY, DUMMY, Option::None).await;
    // assert_eq!(result.len(), 1);
}

// curl "https://gitlab.com/api/graphql" --header "Authorization: Bearer TODO" \
//     --header "Content-Type: application/json" --request POST \
//     --data '{"query": "query {group(fullPath: \"TODO\") {id name mergeRequests {nodes {id title}}}}"}'