CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'SYSTEM',
    modified_by TEXT NOT NULL DEFAULT 'SYSTEM'
);

CREATE TABLE user_statuses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'SYSTEM',
    modified_by TEXT NOT NULL DEFAULT 'SYSTEM'
);

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    avatar TEXT,
    gender TEXT,
    age INTEGER,
    accept_terms_and_conditions BOOLEAN NOT NULL DEFAULT FALSE,
    allow_email_communications BOOLEAN NOT NULL DEFAULT FALSE,
    ip_address TEXT NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    auth_hash TEXT,
    auth_hash_expiration TIMESTAMPTZ,
    two_factor_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    two_factor_secret TEXT,
    user_status_id UUID NOT NULL REFERENCES user_statuses (id),
    role_id UUID NOT NULL REFERENCES roles (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'SYSTEM',
    modified_by TEXT NOT NULL DEFAULT 'SYSTEM'
);

CREATE INDEX users_is_active_last_seen_idx ON users (is_active, last_seen);
CREATE INDEX users_user_status_id_is_active_idx ON users (user_status_id, is_active);
CREATE INDEX users_role_id_idx ON users (role_id);
CREATE INDEX users_auth_hash_idx ON users (auth_hash);
CREATE INDEX users_created_at_idx ON users (created_at);
CREATE INDEX users_created_at_id_idx ON users (created_at DESC, id);

CREATE TABLE webhook_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    events TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    retry_count INTEGER NOT NULL DEFAULT 3,
    timeout_seconds INTEGER NOT NULL DEFAULT 30,
    created_by TEXT,
    modified_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_triggered_at TIMESTAMPTZ
);

CREATE INDEX webhook_subscriptions_is_active_idx ON webhook_subscriptions (is_active);
CREATE INDEX webhook_subscriptions_created_at_idx ON webhook_subscriptions (created_at);

CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES webhook_subscriptions (id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL,
    http_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX webhook_deliveries_subscription_id_idx ON webhook_deliveries (subscription_id);
CREATE INDEX webhook_deliveries_status_idx ON webhook_deliveries (status);
CREATE INDEX webhook_deliveries_event_type_idx ON webhook_deliveries (event_type);
CREATE INDEX webhook_deliveries_created_at_idx ON webhook_deliveries (created_at);
CREATE INDEX webhook_deliveries_next_retry_at_idx ON webhook_deliveries (next_retry_at);

CREATE TRIGGER roles_set_updated_at BEFORE UPDATE ON roles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER user_statuses_set_updated_at BEFORE UPDATE ON user_statuses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER webhook_subscriptions_set_updated_at BEFORE UPDATE ON webhook_subscriptions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER webhook_deliveries_set_updated_at BEFORE UPDATE ON webhook_deliveries
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
