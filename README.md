# engineering-metrics-data-collector

# testing

`cargo test`

## integration testing

1. Start the local docker-engine, i.e. `colima start`

## test against local postgres
```
docker run --name local-postgres -p 5432:5432 -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=postgres -d postgres
```