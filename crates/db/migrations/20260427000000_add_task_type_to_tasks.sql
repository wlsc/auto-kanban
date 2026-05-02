ALTER TABLE tasks ADD COLUMN task_type TEXT NOT NULL DEFAULT 'regular'
    CHECK (task_type IN ('regular', 'comparison'));
