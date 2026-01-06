use engineering_metrics_data_collector::client::gitlab_graphql_client::GitlabGraphQLClient;
use engineering_metrics_data_collector::client::gitlab_rest_client::GitlabRestClient;
use engineering_metrics_data_collector::component::merge_request::{self, DiffStatsSummary};
use engineering_metrics_data_collector::context::GitlabContext;
use engineering_metrics_data_collector::store::Store;
use testcontainers::runners::AsyncRunner;
mod postgres_container;

use serde_json::json;
use sqlx::Row;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn should_successfully_import_merge_requests_from_gitlab_to_the_database() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;

    store.migrate().await.unwrap();

    let graphql_mock_server = MockServer::start().await;
    let graphql_mock_server_response = get_graphql_query_response_mock().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(graphql_mock_server_response))
        .mount(&graphql_mock_server)
        .await;

    let rest_mock_server = MockServer::start().await;
    let rest_mock_server_response = get_rest_empty_response_mock().await;
    Mock::given(method("GET"))
        .and(path("/projects/52263413/merge_requests/777/closes_issues"))
        .respond_with(ResponseTemplate::new(200).set_body_string(rest_mock_server_response))
        .mount(&rest_mock_server)
        .await;

    let rest_mock_server_response2 =
        get_rest_closed_issues_on_merge_for_mr_888_response_mock().await;
    Mock::given(method("GET"))
        .and(path("/projects/52263413/merge_requests/888/closes_issues"))
        .respond_with(ResponseTemplate::new(200).set_body_string(rest_mock_server_response2))
        .mount(&rest_mock_server)
        .await;

    const DUMMY: &String = &String::new();
    let merge_request_handler = merge_request::MergeRequestHandler {
        context: GitlabContext {
            store: store.to_owned(),
            gitlab_rest_client: GitlabRestClient::new(DUMMY, rest_mock_server.uri()).unwrap(),
            gitlab_graphql_client: GitlabGraphQLClient::new(DUMMY, graphql_mock_server.uri())
                .unwrap(),
            ai_base_url: "http://localhost:11434/v1".to_string(),
            ai_model: "llama3".to_string(),
            ai_api_key: "test-key".to_string(),
            ai_max_context_chars: 10000,
        },
    };

    merge_request_handler
        .import_merge_requests(DUMMY, DUMMY)
        .await;

    let mut conn = store.conn_pool.acquire().await.unwrap();
    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, project_id, created_at, merged_at, diff_stats_summary, mr_web_url
        FROM engineering_metrics.merge_requests")
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 2);

    // fetch concrete merge request that is merged with id equal to gid://gitlab/MergeRequest/221742778
    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, mr_web_url, project_id, created_at, merged_at, diff_stats_summary,
            project_name, updated_at, created_by, merged_by, approved, approved_by
        FROM engineering_metrics.merge_requests
        WHERE mr_id = 'gid://gitlab/MergeRequest/221742778'")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        result.get::<String, _>("mr_id"),
        "gid://gitlab/MergeRequest/221742778"
    );
    assert_eq!(result.get::<String, _>("mr_iid"), "777");
    assert_eq!(
        result.get::<String, _>("mr_title"),
        "Resolve \"pipeline check\""
    );
    assert_eq!(
        result.get::<String, _>("mr_web_url"),
        "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/221742778"
    );
    assert_eq!(result.get::<String, _>("project_id"), "52263413");
    assert_eq!(
        result.get::<Option<String>, _>("project_name"),
        Some("cool_project_1".to_string())
    );
    assert_eq!(
        result.get::<OffsetDateTime, _>("created_at"),
        OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<Option<OffsetDateTime>, _>("updated_at"),
        Some(OffsetDateTime::parse("2020-03-02T09:10:00Z", &Rfc3339).unwrap())
    );
    assert_eq!(
        result.get::<Option<OffsetDateTime>, _>("merged_at"),
        Some(OffsetDateTime::parse("2020-03-02T09:20:00Z", &Rfc3339).unwrap())
    );
    assert_eq!(
        result.get::<Option<String>, _>("created_by"),
        Some("dev1".to_string())
    );
    assert_eq!(
        result.get::<Option<String>, _>("merged_by"),
        Some("dev1".to_string())
    );
    assert_eq!(result.get::<Option<bool>, _>("approved"), Some(true));
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("approved_by"),
        Some(json!(["dev2"]))
    );
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("diff_stats_summary"),
        Some(json!({
            "additions": 2,
            "deletions": 2,
            "changes": 4,
            "file_count": 1,
        }))
    );

    // fetch concrete merge request that is not merged with id equal to gid://gitlab/MergeRequest/221706264
    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, mr_web_url, project_id, created_at, merged_at, diff_stats_summary,
            project_name, updated_at, created_by, merged_by, approved, approved_by
        FROM engineering_metrics.merge_requests
        WHERE mr_id = 'gid://gitlab/MergeRequest/221706264'")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        result.get::<String, _>("mr_id"),
        "gid://gitlab/MergeRequest/221706264"
    );
    assert_eq!(result.get::<String, _>("mr_iid"), "888");
    assert_eq!(
        result.get::<String, _>("mr_title"),
        "Resolve \"Increase the size of login session cache\""
    );
    assert_eq!(
        result.get::<String, _>("mr_web_url"),
        "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/221706264"
    );
    assert_eq!(result.get::<String, _>("project_id"), "52263413");
    assert_eq!(
        result.get::<String, _>("project_name"),
        "cool_project_1".to_string()
    );
    assert_eq!(
        result.get::<OffsetDateTime, _>("created_at"),
        OffsetDateTime::parse("2020-03-02T09:30:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<OffsetDateTime, _>("updated_at"),
        OffsetDateTime::parse("2020-03-02T09:40:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<Option<OffsetDateTime>, _>("merged_at"),
        Option::None
    );
    assert_eq!(
        result.get::<Option<String>, _>("created_by"),
        Some("dev3".to_string())
    );
    assert_eq!(result.get::<Option<String>, _>("merged_by"), Option::None);
    assert_eq!(result.get::<Option<bool>, _>("approved"), Some(false));
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("approved_by"),
        Some(json!([]))
    );
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("diff_stats_summary"),
        Some(serde_json::Value::Null)
    );
}

#[tokio::test]
async fn should_persist_and_select_one_not_merged_mr_successfully() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();
    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();
    let mut conn = store.conn_pool.acquire().await.unwrap();

    let mr = merge_request::MergeRequest {
        mr_id: "gitlab/1".to_string(),
        mr_iid: "123".to_string(),
        mr_title: "awesome issue".to_string(),
        mr_web_url: "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/1".to_string(),
        project_id: "gitlab/1".to_string(),
        project_name: "cool project 1".to_string(),
        project_path: "cool-project-x".to_string(),
        created_at: OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap(),
        updated_at: OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap(),
        merged_at: Option::None,
        created_by: "user1".to_string(),
        merged_by: Option::None,
        approved: false,
        approved_by: Option::None,
        diff_stats_summary: Option::None,
        labels: Option::None,
        mr_ai_title: Option::None,
        mr_ai_summary: Option::None,
        mr_ai_model: Option::None,
        mr_ai_category: Option::None,
        mr_description: Option::None,
    };

    const DUMMY: &String = &String::new();
    let merge_request_handler = merge_request::MergeRequestHandler {
        context: GitlabContext {
            store,
            gitlab_rest_client: GitlabRestClient::new(DUMMY, DUMMY.to_string()).unwrap(),
            gitlab_graphql_client: GitlabGraphQLClient::new(DUMMY, DUMMY.to_string()).unwrap(),
            ai_base_url: "http://localhost:11434/v1".to_string(),
            ai_model: "llama3".to_string(),
            ai_api_key: "test-key".to_string(),
            ai_max_context_chars: 10000,
        },
    };

    merge_request_handler
        .persist_merge_request(&mr)
        .await
        .unwrap();

    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, mr_web_url, project_id, created_at, merged_at, diff_stats_summary,
        project_name, updated_at, created_by, merged_by, approved, approved_by
        FROM engineering_metrics.merge_requests")
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 1);

    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, mr_web_url, project_id, created_at, merged_at, diff_stats_summary,
        project_name, updated_at, created_by, merged_by, approved, approved_by
        FROM engineering_metrics.merge_requests")
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    assert_eq!(result.get::<String, _>("mr_id"), "gitlab/1");
    assert_eq!(result.get::<String, _>("mr_iid"), "123");
    assert_eq!(result.get::<String, _>("mr_title"), "awesome issue");
    assert_eq!(
        result.get::<String, _>("mr_web_url"),
        "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/1"
    );
    assert_eq!(result.get::<String, _>("project_id"), "gitlab/1");
    assert_eq!(
        result.get::<OffsetDateTime, _>("created_at"),
        OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<OffsetDateTime, _>("updated_at"),
        OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<Option<OffsetDateTime>, _>("merged_at"),
        Option::None
    );
    assert_eq!(result.get::<String, _>("created_by"), "user1");
    assert_eq!(result.get::<Option<String>, _>("merged_by"), Option::None);
    assert!(!result.get::<bool, _>("approved"));
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("approved_by"),
        Some(serde_json::Value::Null)
    );
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("diff_stats_summary"),
        Some(serde_json::Value::Null)
    );
}

#[tokio::test]
async fn should_persist_and_select_one_merged_mr_successfully() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();
    let store = Store::new(&format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        port
    ))
    .await;
    store.migrate().await.unwrap();
    let mut conn = store.conn_pool.acquire().await.unwrap();

    let mr = merge_request::MergeRequest {
        mr_id: "gitlab_mr/2".to_string(),
        mr_iid: "234".to_string(),
        mr_title: "awesome issue".to_string(),
        mr_web_url: "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/2".to_string(),
        project_id: "gitlab/2".to_string(),
        project_name: "cool project 2".to_string(),
        project_path: "cool-project-2".to_string(),
        created_at: OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap(),
        updated_at: OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap(),
        merged_at: Some(OffsetDateTime::parse("2020-03-02T09:20:00Z", &Rfc3339).unwrap()),
        created_by: "user1".to_string(),
        merged_by: Some("user2".to_string()),
        approved: true,
        approved_by: Some(vec!["user3".to_string()]),
        diff_stats_summary: Some(DiffStatsSummary {
            additions: 10,
            deletions: 5,
            changes: 15,
            file_count: 2,
        }),
        labels: Some(vec!["bug".to_string(), "engineering".to_string()]),
        mr_ai_title: Option::None,
        mr_ai_summary: Option::None,
        mr_ai_model: Option::None,
        mr_ai_category: Option::None,
        mr_description: Option::None,
    };

    const DUMMY: &String = &String::new();
    let merge_request_handler = merge_request::MergeRequestHandler {
        context: GitlabContext {
            store,
            gitlab_rest_client: GitlabRestClient::new(DUMMY, DUMMY.to_string()).unwrap(),
            gitlab_graphql_client: GitlabGraphQLClient::new(DUMMY, DUMMY.to_string()).unwrap(),
            ai_base_url: "http://localhost:11434/v1".to_string(),
            ai_model: "llama3".to_string(),
            ai_api_key: "test-key".to_string(),
            ai_max_context_chars: 10000,
        },
    };

    merge_request_handler
        .persist_merge_request(&mr)
        .await
        .unwrap();

    let result = sqlx::query("SELECT mr_id, mr_iid, mr_title, mr_web_url, project_id, project_name, project_path, created_at, merged_at, diff_stats_summary,
        project_name, updated_at, created_by, merged_by, approved, approved_by
        FROM engineering_metrics.merge_requests")
        .fetch_one(&mut *conn)
        .await
        .unwrap();

    assert_eq!(result.get::<String, _>("mr_id"), "gitlab_mr/2");
    assert_eq!(result.get::<String, _>("mr_iid"), "234");
    assert_eq!(result.get::<String, _>("mr_title"), "awesome issue");
    assert_eq!(
        result.get::<String, _>("mr_web_url"),
        "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/2"
    );
    assert_eq!(result.get::<String, _>("project_id"), "gitlab/2");
    assert_eq!(result.get::<String, _>("project_name"), "cool project 2");
    assert_eq!(result.get::<String, _>("project_path"), "cool-project-2");
    assert_eq!(
        result.get::<OffsetDateTime, _>("created_at"),
        OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<OffsetDateTime, _>("updated_at"),
        OffsetDateTime::parse("2020-03-02T09:00:00Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        result.get::<Option<OffsetDateTime>, _>("merged_at"),
        Some(OffsetDateTime::parse("2020-03-02T09:20:00Z", &Rfc3339).unwrap())
    );
    assert_eq!(result.get::<String, _>("created_by"), "user1");
    assert_eq!(
        result.get::<Option<String>, _>("merged_by"),
        Some("user2".to_string())
    );
    assert!(result.get::<bool, _>("approved"));
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("approved_by"),
        Some(serde_json::json!(["user3"]))
    );
    assert_eq!(
        result.get::<Option<serde_json::Value>, _>("diff_stats_summary"),
        Some(serde_json::json!({
            "additions": 10,
            "deletions": 5,
            "changes": 15,
            "file_count": 2,
        }))
    );
}

#[tokio::test]
async fn should_fetch_from_gitlab_graphql_successfully() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();
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

    let mut resp = surf::post(mock_server.uri()).await.unwrap();
    println!(
        "response body: {:?}, status: {:?}",
        &mut resp.body_string().await.unwrap(),
        resp.status()
    );
    assert_eq!(resp.status(), 200);

    const DUMMY: &String = &String::new();
    let merge_request_handler = merge_request::MergeRequestHandler {
        context: GitlabContext {
            store,
            gitlab_rest_client: GitlabRestClient::new(DUMMY, DUMMY.to_string()).unwrap(),
            gitlab_graphql_client: GitlabGraphQLClient::new(DUMMY, mock_server.uri()).unwrap(),
            ai_base_url: "http://localhost:11434/v1".to_string(),
            ai_model: "llama3".to_string(),
            ai_api_key: "test-key".to_string(),
            ai_max_context_chars: 10000,
        },
    };

    let result = merge_request_handler
        .fetch_group_merge_requests(DUMMY, DUMMY, Option::None)
        .await
        .unwrap();

    assert_eq!(result.merge_requests.len(), 2);
}

async fn get_graphql_query_response_mock() -> &'static str {
    r#"
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
                        "iid": "777",
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
                        "project": {
                            "name": "cool_project_1",
                            "path": "cool-project-1"
                        },
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
                        "iid": "888",
                        "title": "Resolve \"Increase the size of login session cache\"",
                        "draft": false,
                        "webUrl": "https://gitlab.com/gitlab-org/gitlab/-/merge_requests/221706264",
                        "labels": {
                            "nodes": [{
                                "title": "product"
                            }]
                        },
                        "approved": false,
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
                        "project": {
                            "name": "cool_project_1",
                            "path": "cool-project-1"
                        },
                        "diffStatsSummary": null,
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
    "#
}

async fn get_rest_empty_response_mock() -> &'static str {
    r#"
    []
    "#
}

async fn get_rest_closed_issues_on_merge_for_mr_888_response_mock() -> &'static str {
    r#"
    [{
        "id": 111222333444,
        "iid": 52,
        "project_id": 52263413,
        "title": "test title",
        "description": "",
        "state": "closed",
        "created_at": "2023-07-28T13:20:30.675Z",
        "updated_at": "2023-07-28T15:10:30.781Z",
        "closed_at": "2023-07-28T15:10:30.755Z",
        "closed_by": {
            "id": 12345678,
            "username": "test",
            "name": "test test",
            "state": "active",
            "avatar_url": "https://gitlab.com/uploads/-/system/user/avatar/12345678/avatar.png",
            "web_url": "https://gitlab.com/test"
        },
        "labels": ["bug", "test"],
        "milestone": null,
        "assignees": [],
        "author": {
            "id": 12345678,
            "username": "test",
            "name": "test test",
            "state": "active",
            "avatar_url": "https://gitlab.com/uploads/-/system/user/avatar/12345678/avatar.png",
            "web_url": "https://gitlab.com/test"
        },
        "type": "ISSUE",
        "assignee": null,
        "user_notes_count": 0,
        "merge_requests_count": 1,
        "upvotes": 0,
        "downvotes": 0,
        "due_date": null,
        "confidential": false,
        "discussion_locked": null,
        "issue_type": "issue",
        "web_url": "https://gitlab.com/test/-/issues/52",
        "time_stats": {
            "time_estimate": 0,
            "total_time_spent": 0,
            "human_time_estimate": null,
            "human_total_time_spent": null
        },
        "task_completion_status": {
            "count": 0,
            "completed_count": 0
        },
        "weight": null,
        "blocking_issues_count": 0
    }]
    "#
}
