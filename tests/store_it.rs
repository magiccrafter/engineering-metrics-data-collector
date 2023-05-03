use engineering_metrics_data_manager::store::Store;
use testcontainers::clients;
mod postgres_container;

#[tokio::test]
async fn should_successfully_create_a_new_store_and_establish_a_connection() {
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
    let result = sqlx::query("SELECT 1 + 1 AS result")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 1);
}
