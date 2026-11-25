CREATE TABLE agent_facts (
    name TEXT NOT NULL,
    incarnation TEXT NOT NULL,
    facts TEXT NOT NULL,
    reported_at INTEGER NOT NULL,
    PRIMARY KEY(name,incarnation),
    FOREIGN KEY(name,incarnation) REFERENCES agents(name,incarnation)
) WITHOUT ROWID;
CREATE TABLE agent_transcript (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    incarnation TEXT NOT NULL,
    item_key TEXT NOT NULL,
    role INTEGER NOT NULL CHECK(role IN (0,1)),
    text TEXT NOT NULL,
    UNIQUE(name,incarnation,item_key),
    FOREIGN KEY(name,incarnation) REFERENCES agents(name,incarnation)
);
CREATE INDEX agent_transcript_history ON agent_transcript(name,incarnation,id);
