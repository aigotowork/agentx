-- Proposed physical schema, not an installed AgentX migration.
-- Requires the existing agent_creations and agent_creation_instances tables.
-- Service validation owns key grammar, Unicode rules and per-owner size limits.
CREATE TABLE agent_label_sets (
    owner_creation TEXT NOT NULL COLLATE BINARY PRIMARY KEY
        REFERENCES agent_creations(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL DEFAULT 0
        CHECK(typeof(revision) = 'integer' AND revision >= 0)
) WITHOUT ROWID;

CREATE TABLE agent_labels (
    owner_creation TEXT NOT NULL COLLATE BINARY
        REFERENCES agent_label_sets(owner_creation) ON DELETE CASCADE,
    label_key TEXT NOT NULL COLLATE BINARY
        CHECK(typeof(label_key) = 'text'
              AND length(CAST(label_key AS BLOB)) BETWEEN 1 AND 127),
    label_value TEXT NOT NULL COLLATE BINARY
        CHECK(typeof(label_value) = 'text'
              AND length(CAST(label_value AS BLOB)) BETWEEN 1 AND 256),
    PRIMARY KEY(owner_creation, label_key)
) WITHOUT ROWID;

-- Equality/positive-existence seed; covers owner IDs without table hydration.
CREATE INDEX agent_labels_by_key_value_owner
    ON agent_labels(label_key, label_value, owner_creation);

-- Existing aliases otherwise have only (name, incarnation) forward lookup.
CREATE INDEX agent_creation_instances_by_creation
    ON agent_creation_instances(creation, name, incarnation);
