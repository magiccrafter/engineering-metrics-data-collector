-- Add up migration script here
CREATE SCHEMA engineering_metrics;

CREATE TABLE engineering_metrics.issues (
    issue_id VARCHAR PRIMARY KEY,
    issue_title VARCHAR NOT NULL,
    project_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    created_by VARCHAR NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    updated_by VARCHAR NOT NULL,
    closed_at TIMESTAMPTZ NOT NULL,
    closed_by VARCHAR NOT NULL
);

CREATE TABLE engineering_metrics.merge_requests (
    mr_id VARCHAR PRIMARY KEY,
    mr_title VARCHAR NOT NULL,
    project_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    created_by VARCHAR NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    updated_by VARCHAR NOT NULL,
    merged_at TIMESTAMPTZ NOT NULL,
    merged_by VARCHAR NOT NULL,
    mr_state VARCHAR NOT NULL
);

CREATE TABLE engineering_metrics.closed_issues_on_merge (
    issue_id VARCHAR NOT NULL,
    mr_id VARCHAR NOT NULL,
    mr_title VARCHAR NOT NULL,
    project_id VARCHAR NOT NULL,
    primary key (issue_id, mr_id)
);