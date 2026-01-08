# engineering-metrics-data-collector

[![Build Status](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector/graph/badge.svg?token=OMJGUHD1B2)](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector)

A Rust-based collector for GitLab engineering metrics. Imports projects and merge requests into PostgreSQL with optional AI-powered summarization.

## Features

- GitLab REST and GraphQL API integration
- Incremental updates with resumable imports
- Optional AI enhancement for merge requests
- Concurrent processing of multiple groups

## Quick Start

### Prerequisites

- Rust 1.73+, PostgreSQL 18+, Docker (optional)
- GitLab API token

### Installation

```bash
cargo build --release
# or
docker build -t engineering-metrics-data-collector .
```

### Configuration

Create `.env` file:

```bash
DATABASE_URL=postgresql://user:password@localhost:5432/dbname
GITLAB_REST_ENDPOINT=https://gitlab.com/api/v4
GITLAB_GRAPHQL_ENDPOINT=https://gitlab.com/api/graphql
GITLAB_API_TOKEN=your_token
GITLAB_FULL_PATH_GROUP_LIST=group1,group2

# Optional AI configuration
AI_BASE_URL=https://api.openai.com/v1
AI_MODEL=gpt-4
AI_API_KEY=your_key
AI_MAX_CONTEXT_CHARS=8000

# Optional settings
UPSERT_MERGE_REQUESTS=true
INITIAL_INGESTION_DATE=2024-01-01T00:00:00Z
```

### Run

```bash
cargo run --release
# or
docker run --env-file .env engineering-metrics-data-collector
```

## Database Schema

Tables in `engineering_metrics` schema:
- `projects` - GitLab project metadata
- `merge_requests` - MR data with AI enhancement fields
- `collector_runs` - Tracks successful runs for incremental updates
- `import_progress` - Enables resumable imports

## Development

```bash
cargo fmt              # Format code
cargo check            # Check compilation
cargo clippy --all-targets  # Lint
cargo test             # Run tests
```

**Guidelines**: No `.unwrap()`, use `Result` and `?`. Prefer `&str` over `String`. Keep code clippy-clean.

## Testing

Integration tests require Docker. Tests use [wiremock](https://crates.io/crates/wiremock) for API mocking.

```bash
# Start local PostgreSQL
docker run --name local-postgres -p 5432:5432 \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres -d postgres:18

# Cleanup
docker rm -f local-postgres
```

## GitLab API Authentication

Supported methods:

```bash
# Personal Access Token
curl --header "PRIVATE-TOKEN: XXX" "https://gitlab.com/api/v4/projects/{id}"

# OAuth2 Bearer Token
curl --header "Authorization: Bearer XXX" "https://gitlab.com/api/v4/projects/{id}"
```
