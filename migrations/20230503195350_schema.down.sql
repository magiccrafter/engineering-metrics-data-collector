-- Add down migration script here
DROP TABLE IF EXISTS engineering_metrics.issues;
DROP TABLE IF EXISTS engineering_metrics.merge_requests;
DROP TABLE IF EXISTS engineering_metrics.closed_issues_on_merge;
DROP SCHEMA IF EXISTS engineering_metrics;