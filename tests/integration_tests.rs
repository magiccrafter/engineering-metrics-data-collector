use testcontainers::{clients};

mod postgres_container;

#[tokio::test]
async fn test_add() {
    let docker = clients::Cli::default();
    let image = postgres_container::Postgres::default();
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5432);
    print!("{}", port);

    assert_eq!(port > 1000, true);
}
