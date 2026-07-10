-- T-068.9: engine-derived compatibility edges between registry items
-- (registry-compat.schema.json#/$defs/edge), modpack-scoped like registry_items.
-- Nodes are full Enfusion resource_name strings (graph identity — no FK, matching
-- the registry_items precedent). edge_type is plain text so new edge families ship
-- via a schema-enum bump alone, with no DDL change.
CREATE TABLE registry_compat (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    modpack_id uuid NOT NULL,
    from_node text NOT NULL,
    to_node text NOT NULL,
    edge_type text NOT NULL,
    evidence text,
    created_at timestamptz DEFAULT now() NOT NULL,
    updated_at timestamptz DEFAULT now() NOT NULL,
    CONSTRAINT registry_compat_pkey PRIMARY KEY (id)
);

-- Upsert target for the idempotent importer (ON CONFLICT).
CREATE UNIQUE INDEX idx_registry_compat_edge
    ON registry_compat (modpack_id, from_node, to_node, edge_type);

-- Adjacency lookups: "what fits this host" / "where does this item go".
CREATE INDEX idx_registry_compat_modpack_to ON registry_compat (modpack_id, to_node);
CREATE INDEX idx_registry_compat_modpack_from ON registry_compat (modpack_id, from_node);
