CREATE TABLE engineering_metrics.copilot_user_daily_metrics (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    org_slug VARCHAR NOT NULL,
    report_day DATE NOT NULL,
    user_id BIGINT NOT NULL,
    user_login VARCHAR NOT NULL,
    organization_id VARCHAR NOT NULL,
    enterprise_id VARCHAR NULL,
    user_initiated_interaction_count BIGINT NOT NULL DEFAULT 0,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    used_agent BOOLEAN NOT NULL DEFAULT FALSE,
    used_chat BOOLEAN NOT NULL DEFAULT FALSE,
    used_cli BOOLEAN NOT NULL DEFAULT FALSE,
    used_copilot_coding_agent BOOLEAN NOT NULL DEFAULT FALSE,
    used_copilot_cloud_agent BOOLEAN NOT NULL DEFAULT FALSE,
    totals_by_cli JSONB NULL,
    raw_payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_slug, report_day, user_id)
);

CREATE INDEX idx_copilot_user_daily_metrics_org_day
    ON engineering_metrics.copilot_user_daily_metrics (org_slug, report_day DESC);
CREATE INDEX idx_copilot_user_daily_metrics_user_login
    ON engineering_metrics.copilot_user_daily_metrics (user_login);

CREATE TABLE engineering_metrics.copilot_user_daily_feature_metrics (
    user_metric_id UUID NOT NULL REFERENCES engineering_metrics.copilot_user_daily_metrics (id) ON DELETE CASCADE,
    feature VARCHAR NOT NULL,
    user_initiated_interaction_count BIGINT NOT NULL DEFAULT 0,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_metric_id, feature)
);

CREATE TABLE engineering_metrics.copilot_user_daily_ide_metrics (
    user_metric_id UUID NOT NULL REFERENCES engineering_metrics.copilot_user_daily_metrics (id) ON DELETE CASCADE,
    ide VARCHAR NOT NULL,
    user_initiated_interaction_count BIGINT NOT NULL DEFAULT 0,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    last_known_plugin_version JSONB NULL,
    last_known_ide_version JSONB NULL,
    PRIMARY KEY (user_metric_id, ide)
);

CREATE TABLE engineering_metrics.copilot_user_daily_language_feature_metrics (
    user_metric_id UUID NOT NULL REFERENCES engineering_metrics.copilot_user_daily_metrics (id) ON DELETE CASCADE,
    language VARCHAR NOT NULL,
    feature VARCHAR NOT NULL,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_metric_id, language, feature)
);

CREATE TABLE engineering_metrics.copilot_user_daily_language_model_metrics (
    user_metric_id UUID NOT NULL REFERENCES engineering_metrics.copilot_user_daily_metrics (id) ON DELETE CASCADE,
    language VARCHAR NOT NULL,
    model VARCHAR NOT NULL,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_metric_id, language, model)
);

CREATE TABLE engineering_metrics.copilot_user_daily_model_feature_metrics (
    user_metric_id UUID NOT NULL REFERENCES engineering_metrics.copilot_user_daily_metrics (id) ON DELETE CASCADE,
    model VARCHAR NOT NULL,
    feature VARCHAR NOT NULL,
    user_initiated_interaction_count BIGINT NOT NULL DEFAULT 0,
    code_generation_activity_count BIGINT NOT NULL DEFAULT 0,
    code_acceptance_activity_count BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_add_sum BIGINT NOT NULL DEFAULT 0,
    loc_suggested_to_delete_sum BIGINT NOT NULL DEFAULT 0,
    loc_added_sum BIGINT NOT NULL DEFAULT 0,
    loc_deleted_sum BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_metric_id, model, feature)
);

CREATE TABLE engineering_metrics.copilot_collector_runs (
    id SERIAL PRIMARY KEY,
    org_slug VARCHAR NOT NULL,
    last_successful_run_started_at TIMESTAMPTZ NOT NULL,
    last_successful_run_completed_at TIMESTAMPTZ NOT NULL,
    last_completed_report_day DATE NOT NULL
);

CREATE INDEX idx_copilot_collector_runs_org_started_at
    ON engineering_metrics.copilot_collector_runs (org_slug, last_successful_run_started_at DESC);