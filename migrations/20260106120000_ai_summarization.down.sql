-- Revert migration

ALTER TABLE engineering_metrics.merge_requests DROP COLUMN mr_ai_title;
ALTER TABLE engineering_metrics.merge_requests DROP COLUMN mr_ai_summary;
ALTER TABLE engineering_metrics.merge_requests DROP COLUMN mr_ai_model;
ALTER TABLE engineering_metrics.merge_requests DROP COLUMN mr_ai_category;

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
