# engineering-metrics-data-collector

[![Build Status](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector/graph/badge.svg?token=OMJGUHD1B2)](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector)

## Testing

The testing suite is mainly integration testing. It requires a local docker-engine to be running. 
Integration tests spin up a local postgres database and run the application against it. The 3rd party APIs are mocked using [wiremock](https://crates.io/crates/wiremock)

`cargo test` runs all tests.

### Starting a local postgres database
```bash
# start
docker run --name local-postgres -p 5432:5432 -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=postgres -d postgres

# stop & remove 
docker rm -f local-postgres
```

## 3rd party API authentication methods
The following Gitlab API authentication methods are supported:
```bash
curl --header "PRIVATE-TOKEN: XXX" "https://gitlab.com/api/v4/projects/{}"
curl --header "Authorization: Bearer XXX" "https://gitlab.com/api/v4/projects/{}"
```

The Atlassian's API authentication method is Basic Authentication:
```bash
# generate base64 string from user:api_token used for Basic Authentication header
echo -n user@example.com:api_token_string | base64

curl curl -D- \
   -X GET \
   -H "Authorization: Basic some_base64_string" \
   -H "Content-Type: application/json" \
   "https://your-domain.atlassian.net/rest/api/2/issue/ISSUE-Z"
```
