# engineering-metrics-data-collector

# testing

`cargo test`

## integration testing

1. Start the local docker-engine, i.e. `colima start`

## test against local postgres
```bash
# start
docker run --name local-postgres -p 5432:5432 -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=postgres -d postgres

# stop & remove 
docker rm -f local-postgres
```

## notes
The following Gitlab API authentication methods are supported:
```bash
curl --header "PRIVATE-TOKEN: XXX" "https://gitlab.com/api/v4/projects/{}"
curl --header "Authorization: Bearer XXX" "https://gitlab.com/api/v4/projects/{}"
```