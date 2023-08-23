# engineering-metrics-data-collector

[![Build Status](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector/graph/badge.svg?token=OMJGUHD1B2)](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector)

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