# engineering-metrics-data-collector

[![Build Status](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/magiccrafter/engineering-metrics-data-collector/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector/graph/badge.svg?token=OMJGUHD1B2)](https://codecov.io/gh/magiccrafter/engineering-metrics-data-collector)

A Rust-based collector for GitLab engineering metrics and GitHub Copilot usage metrics. Imports projects, merge requests, and Copilot user metrics into PostgreSQL with optional AI-powered summarization for merge requests.

## Features

- GitLab REST and GraphQL API integration
- GitHub Copilot org user usage metrics ingestion via daily report downloads
- Incremental updates with resumable imports
- Optional AI enhancement for merge requests
- Concurrent processing of multiple groups

## Quick Start

### Prerequisites

- Rust 1.73+, PostgreSQL 18+, Docker (optional)
- GitLab API token
- GitHub token with Copilot metrics access when running the Copilot collector

### Build

This repository now produces 2 Cargo binaries:

- `engineering-metrics-data-collector` - the GitLab collector
- `copilot_metrics_collector` - the GitHub Copilot metrics collector

```bash
# Build both binaries
cargo build --release

# Build only the GitLab collector
cargo build --release --bin engineering-metrics-data-collector

# Build only the Copilot metrics collector
cargo build --release --bin copilot_metrics_collector
```

The compiled executables are written to:

```bash
target/release/engineering-metrics-data-collector
target/release/copilot_metrics_collector
```

The current Dockerfile still packages only the default GitLab collector binary:

```bash
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

# Optional GitHub Copilot collector configuration
GITHUB_API_TOKEN=your_github_token
GITHUB_API_ENDPOINT=https://api.github.com
GITHUB_API_VERSION=2026-03-10
GITHUB_COPILOT_ORG_LIST=org1,org2
COPILOT_REPORT_LAG_DAYS=2
COPILOT_INITIAL_INGESTION_DATE=2026-01-01T00:00:00Z

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
# Run the GitLab collector
cargo run --release --bin engineering-metrics-data-collector

# Or run the current Docker image, which starts the GitLab collector binary
docker run --env-file .env engineering-metrics-data-collector
```

### Run Copilot Metrics Collector

```bash
cargo run --release --bin copilot_metrics_collector
```

This binary fetches daily org-level user usage reports from:

```bash
GET /orgs/{org}/copilot/metrics/reports/users-1-day?day=YYYY-MM-DD
```

The Copilot flow runs independently from the GitLab collector and keeps its own successful-run watermark and resumable import progress.

## Database Schema

Tables in `engineering_metrics` schema:
- `projects` - GitLab project metadata
- `merge_requests` - MR data with AI enhancement fields
- `collector_runs` - Tracks successful runs for incremental updates
- `import_progress` - Enables resumable imports
- `copilot_user_daily_metrics` - Copilot per-user per-day fact table
- `copilot_user_daily_*` - Copilot feature, IDE, language, and model breakdown tables
- `copilot_collector_runs` - Tracks successful Copilot report ingestion by org

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
