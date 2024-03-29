-- add up migration script here
create schema engineering_metrics;

create table engineering_metrics.projects (
    p_id varchar not null,
    p_name varchar not null,
    p_path varchar not null,
    p_full_path varchar not null,
    p_web_url varchar not null,
    topics jsonb null,
    primary key (p_id)
);
create index idx_projects_path on engineering_metrics.projects (p_path);

create table engineering_metrics.issues (
    issue_id varchar not null,
    issue_iid varchar not null,
    issue_title varchar not null,
    issue_web_url varchar not null,
    project_id varchar not null,
    created_at timestamptz not null,
    created_by varchar not null,
    updated_at timestamptz not null,
    updated_by varchar null,
    closed_at timestamptz null,
    labels jsonb null,
    primary key (issue_id)
);

create table engineering_metrics.merge_requests (
    mr_id varchar not null,
    mr_iid varchar not null,
    mr_title varchar not null,
    mr_web_url varchar not null,
    project_id varchar not null,
    project_name varchar not null,
    project_path varchar not null,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    merged_at timestamptz null,
    created_by varchar not null,
    merged_by varchar null,
    approved boolean not null,
    approved_by jsonb null,
    diff_stats_summary jsonb null,
    labels jsonb null,
    primary key (mr_id)
);

create table engineering_metrics.closed_issues_on_merge (
    issue_id varchar not null,
    issue_iid varchar null,
    mr_id varchar not null,
    mr_iid varchar not null,
    project_id varchar not null,
    created_at timestamptz not null,
    primary key (issue_id, mr_id)
);

create table engineering_metrics.external_issues (
    issue_tracker varchar not null,
    issue_id varchar not null,
    issue_display_id varchar null,
    title varchar not null,
    web_url varchar not null,
    imported_at timestamptz not null,
    primary key (issue_tracker, issue_id)
);

create table engineering_metrics.collector_runs (
    id serial not null,
    last_successful_run_started_at timestamptz not null,
    last_successful_run_completed_at timestamptz not null,
    primary key (id)
);

insert into engineering_metrics.collector_runs (last_successful_run_started_at, last_successful_run_completed_at)
values ('2023-01-01T00:00:00Z', '2023-01-01T00:05:00Z');