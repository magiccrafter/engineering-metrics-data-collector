-- add down migration script here
drop table if exists engineering_metrics.projects;
drop table if exists engineering_metrics.issues;
drop table if exists engineering_metrics.merge_requests;
drop table if exists engineering_metrics.closed_issues_on_merge;
drop schema if exists engineering_metrics;