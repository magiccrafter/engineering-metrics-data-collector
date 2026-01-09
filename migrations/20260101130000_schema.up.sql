-- Engineering Metrics Data Collector - Database Schema
CREATE SCHEMA engineering_metrics;

-- Projects table
CREATE TABLE engineering_metrics.projects (
    p_id VARCHAR NOT NULL,
    p_name VARCHAR NOT NULL,
    p_path VARCHAR NOT NULL,
    p_full_path VARCHAR NOT NULL,
    p_web_url VARCHAR NOT NULL,
    topics JSONB NULL,
    PRIMARY KEY (p_id)
);
CREATE INDEX idx_projects_path ON engineering_metrics.projects (p_path);

-- Merge requests table with AI summarization fields
CREATE TABLE engineering_metrics.merge_requests (
    mr_id VARCHAR NOT NULL,
    mr_iid VARCHAR NOT NULL,
    mr_title VARCHAR NOT NULL,
    mr_web_url VARCHAR NOT NULL,
    project_id VARCHAR NOT NULL,
    project_name VARCHAR NOT NULL,
    project_path VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    merged_at TIMESTAMPTZ NULL,
    created_by VARCHAR NOT NULL,
    merged_by VARCHAR NULL,
    approved BOOLEAN NOT NULL,
    approved_by JSONB NULL,
    diff_stats_summary JSONB NULL,
    labels JSONB NULL,
    mr_ai_title VARCHAR NULL,
    mr_ai_summary VARCHAR NULL,
    mr_ai_model VARCHAR NULL,
    mr_ai_category VARCHAR NULL,
    PRIMARY KEY (mr_id)
);

-- Collector runs table to track successful collection runs
CREATE TABLE engineering_metrics.collector_runs (
    id SERIAL NOT NULL,
    last_successful_run_started_at TIMESTAMPTZ NOT NULL,
    last_successful_run_completed_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO engineering_metrics.collector_runs (last_successful_run_started_at, last_successful_run_completed_at)
VALUES ('2023-01-01T00:00:00Z', '2023-01-01T00:05:00Z');

-- Import progress table for resumable imports
CREATE TABLE engineering_metrics.import_progress (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    group_full_path VARCHAR NOT NULL,
    import_type VARCHAR NOT NULL,
    updated_after TIMESTAMPTZ NOT NULL,
    last_cursor VARCHAR NULL,
    total_processed INTEGER NOT NULL DEFAULT 0,
    status VARCHAR NOT NULL DEFAULT 'in_progress',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    error_message TEXT NULL,
    UNIQUE (group_full_path, import_type, updated_after, status)
);
CREATE INDEX idx_import_progress_group_type ON engineering_metrics.import_progress (group_full_path, import_type);
CREATE INDEX idx_import_progress_status ON engineering_metrics.import_progress (status);