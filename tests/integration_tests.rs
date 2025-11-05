use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use testcontainers::runners::AsyncRunner;

mod postgres_container;

#[tokio::test]
async fn should_successfully_start_potgres_container() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();
    print!("{}", port);

    assert_eq!(port > 1000, true);
}

#[tokio::test]
async fn should_successfully_select_data_from_database() {
    // testcontainers 0.22 uses AsyncRunner trait
    let image = postgres_container::Postgres::default();
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5432).await.unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            port
        ))
        .await
        .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let result = sqlx::query("SELECT 1 + 1 AS result")
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(result.rows_affected(), 1);

    let row = sqlx::query("SELECT 1 + 1 AS result")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.get::<i32, _>("result"), 2);
}
