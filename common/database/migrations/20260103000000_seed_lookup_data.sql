INSERT INTO roles (name) VALUES
    ('Super Admin'),
    ('Moderator'),
    ('Support'),
    ('Chat User')
ON CONFLICT (name) DO NOTHING;

INSERT INTO user_statuses (name) VALUES
    ('Online'),
    ('Offline'),
    ('Pending Verification'),
    ('Verified')
ON CONFLICT (name) DO NOTHING;
