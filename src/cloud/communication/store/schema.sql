CREATE TABLE IF NOT EXISTS scope_metadata (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    generation TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hosts (
    name TEXT PRIMARY KEY,
    installation_id TEXT NOT NULL,
    last_seen INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS agents (
    name TEXT NOT NULL,
    host TEXT NOT NULL REFERENCES hosts(name),
    incarnation TEXT NOT NULL,
    retired INTEGER NOT NULL DEFAULT 0 CHECK(retired IN (0,1)),
    PRIMARY KEY(name,incarnation)
) WITHOUT ROWID;
CREATE UNIQUE INDEX IF NOT EXISTS agents_active_name ON agents(name) WHERE retired=0;
CREATE TABLE IF NOT EXISTS claims (
    claim_id TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL,
    agent_incarnation TEXT NOT NULL,
    closed INTEGER NOT NULL DEFAULT 0 CHECK(closed IN (0,1)),
    FOREIGN KEY(agent_name,agent_incarnation) REFERENCES agents(name,incarnation)
) WITHOUT ROWID;
CREATE UNIQUE INDEX IF NOT EXISTS claims_open_agent ON claims(agent_name,agent_incarnation) WHERE closed=0;
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    fingerprint BLOB NOT NULL,
    from_name TEXT NOT NULL,
    from_host TEXT NOT NULL,
    from_incarnation TEXT NOT NULL,
    to_name TEXT NOT NULL,
    to_host TEXT NOT NULL,
    to_incarnation TEXT NOT NULL,
    body TEXT NOT NULL,
    intent TEXT NOT NULL CHECK(intent IN ('request','inform','ack')),
    reply_to INTEGER REFERENCES messages(id),
    thread TEXT,
    created_at INTEGER NOT NULL,
    claim_id TEXT REFERENCES claims(claim_id),
    confirmed INTEGER NOT NULL DEFAULT 0 CHECK(confirmed IN (0,1)),
    FOREIGN KEY(from_name,from_incarnation) REFERENCES agents(name,incarnation),
    FOREIGN KEY(to_name,to_incarnation) REFERENCES agents(name,incarnation)
);
CREATE INDEX IF NOT EXISTS messages_pending ON messages(to_name,to_incarnation,id) WHERE confirmed=0 AND claim_id IS NULL;
CREATE INDEX IF NOT EXISTS messages_claim ON messages(claim_id,id) WHERE confirmed=0 AND claim_id IS NOT NULL;
