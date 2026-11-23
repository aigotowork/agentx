CREATE TABLE control_leases (
    host TEXT NOT NULL REFERENCES hosts(name),
    lease_id TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    is_current INTEGER NOT NULL CHECK(is_current IN (0,1)),
    workspaces TEXT NOT NULL,
    directory_launch INTEGER NOT NULL DEFAULT 0 CHECK(directory_launch IN (0,1)),
    workspace_management INTEGER NOT NULL DEFAULT 0 CHECK(workspace_management IN (0,1)),
    PRIMARY KEY(host,lease_id)
) WITHOUT ROWID;
CREATE UNIQUE INDEX control_current_lease ON control_leases(host) WHERE is_current=1;
CREATE TABLE control_operations (
    ordinal INTEGER PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    host TEXT NOT NULL,
    installation_id TEXT NOT NULL,
    execution_lease_id TEXT NOT NULL,
    command TEXT NOT NULL,
    claim_id TEXT UNIQUE,
    outcome TEXT,
    FOREIGN KEY(host,execution_lease_id) REFERENCES control_leases(host,lease_id)
);
CREATE INDEX control_pending ON control_operations(host,execution_lease_id,ordinal) WHERE outcome IS NULL;
PRAGMA user_version=3;
