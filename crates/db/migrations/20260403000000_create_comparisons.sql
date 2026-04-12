-- Comparisons: cross-workspace solution comparison evaluated by an agent
CREATE TABLE comparisons (
    id                   BLOB PRIMARY KEY,
    project_id           BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title                TEXT,
    executor_profile_id  TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'pending'
                           CHECK (status IN ('pending','running','completed','failed')),
    result               TEXT,
    execution_process_id BLOB REFERENCES execution_processes(id) ON DELETE SET NULL,
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX idx_comparisons_project_id ON comparisons(project_id);

-- Junction table: which workspaces are part of a comparison
CREATE TABLE comparison_workspaces (
    comparison_id BLOB NOT NULL REFERENCES comparisons(id) ON DELETE CASCADE,
    workspace_id  BLOB NOT NULL REFERENCES workspaces(id)  ON DELETE CASCADE,
    label         TEXT,
    display_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (comparison_id, workspace_id)
);
