-- Migration to drop issue tables and extend merge_requests

DROP TABLE engineering_metrics.issues;
DROP TABLE engineering_metrics.closed_issues_on_merge;
DROP TABLE engineering_metrics.external_issues;

ALTER TABLE engineering_metrics.merge_requests ADD COLUMN mr_ai_title VARCHAR NULL;
ALTER TABLE engineering_metrics.merge_requests ADD COLUMN mr_ai_summary VARCHAR NULL;
ALTER TABLE engineering_metrics.merge_requests ADD COLUMN mr_ai_model VARCHAR NULL;
ALTER TABLE engineering_metrics.merge_requests ADD COLUMN mr_ai_category VARCHAR NULL;
