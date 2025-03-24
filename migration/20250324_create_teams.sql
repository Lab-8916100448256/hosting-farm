-- Create teams table
CREATE TABLE IF NOT EXISTS teams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pid TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name TEXT NOT NULL,
    description TEXT,
    slug TEXT NOT NULL UNIQUE
);

-- Create team_memberships table
CREATE TABLE IF NOT EXISTS team_memberships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pid TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    team_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    invitation_token TEXT,
    invitation_email TEXT,
    invitation_expires_at DATETIME,
    accepted_at DATETIME,
    FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_team_memberships_team_id ON team_memberships (team_id);
CREATE INDEX IF NOT EXISTS idx_team_memberships_user_id ON team_memberships (user_id);
CREATE INDEX IF NOT EXISTS idx_team_memberships_invitation_token ON team_memberships (invitation_token);
