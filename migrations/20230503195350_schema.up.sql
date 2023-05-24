-- add up migration script here
create schema engineering_metrics;

create table engineering_metrics.issues (
    issue_id varchar not null,
    issue_title varchar not null,
    project_id varchar not null,
    created_at timestamptz not null,
    created_by varchar not null,
    updated_at timestamptz not null,
    updated_by varchar not null,
    closed_at timestamptz not null,
    closed_by varchar not null,
    primary key (issue_id)
);

create table engineering_metrics.merge_requests (
    mr_id varchar not null,
    mr_title varchar not null,
    project_id varchar not null,
    project_name varchar not null,
    created_at timestamptz not null,
    -- created_by varchar not null,
    -- updated_at timestamptz not null,
    -- updated_by varchar not null,
    merged_at timestamptz null,
    -- merged_by varchar not null,
    -- mr_state varchar not null,
    primary key (mr_id)
);

create table engineering_metrics.closed_issues_on_merge (
    issue_id varchar not null,
    mr_id varchar not null,
    mr_title varchar not null,
    project_id varchar not null,
    primary key (issue_id, mr_id)
);