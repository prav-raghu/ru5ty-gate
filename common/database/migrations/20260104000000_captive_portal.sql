CREATE TABLE venues (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    session_duration_secs INTEGER NOT NULL DEFAULT 3600 CHECK (session_duration_secs > 0),
    redirect_url TEXT,
    allow_new_sessions BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT,
    modified_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX venues_created_at_id_idx ON venues (created_at DESC, id);

CREATE TABLE gateways (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venue_id UUID NOT NULL REFERENCES venues (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    api_key_hash TEXT NOT NULL,
    last_heartbeat_at TIMESTAMPTZ,
    agent_version TEXT,
    uptime_secs BIGINT,
    active_sessions INTEGER,
    pending_events INTEGER,
    last_sync_ok_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT,
    modified_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (venue_id, name)
);

CREATE INDEX gateways_venue_id_idx ON gateways (venue_id);

CREATE TABLE captive_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venue_id UUID NOT NULL REFERENCES venues (id) ON DELETE CASCADE,
    gateway_id UUID NOT NULL REFERENCES gateways (id) ON DELETE CASCADE,
    mac_identifier TEXT NOT NULL,
    client_identifier TEXT,
    gateway_name TEXT,
    granted_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    end_reason TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT,
    modified_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (gateway_id, mac_identifier, granted_at)
);

CREATE INDEX captive_sessions_venue_granted_idx ON captive_sessions (venue_id, granted_at DESC, id);
CREATE INDEX captive_sessions_open_idx ON captive_sessions (gateway_id, mac_identifier) WHERE ended_at IS NULL;

CREATE TRIGGER venues_set_updated_at BEFORE UPDATE ON venues
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER gateways_set_updated_at BEFORE UPDATE ON gateways
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER captive_sessions_set_updated_at BEFORE UPDATE ON captive_sessions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
