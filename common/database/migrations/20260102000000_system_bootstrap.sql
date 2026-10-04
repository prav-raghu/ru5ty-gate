CREATE TABLE system_bootstrap (
    id TEXT PRIMARY KEY DEFAULT 'singleton' CHECK (id = 'singleton'),
    admin_bootstrapped BOOLEAN NOT NULL DEFAULT FALSE,
    bootstrapped_at TIMESTAMPTZ,
    bootstrapped_user_id UUID
);
